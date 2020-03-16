// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use super::{
    common::parse_hex,
    fetch::Range,
    helpers::{parse_coins_amount, vec_to_hex, xorname_from_pk, xorname_to_hex},
    safe_net::AppendOnlyDataRawData,
    SafeApp,
};
use crate::{Error, Result};
use async_trait::async_trait;
use log::{debug, trace};
use safe_nd::{
    Coins, MDataSeqValue, PublicKey as SafeNdPublicKey, SeqMutableData, Transaction, TransactionId,
    XorName,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, str};
use threshold_crypto::{PublicKey, SecretKey};
use tiny_keccak::sha3_256;

const FAKE_VAULT_FILE: &str = "./fake_vault_data.json";

#[derive(Debug, Serialize, Deserialize)]
struct SafeKey {
    owner: PublicKey,
    value: String,
}

type AppendOnlyDataFake = Vec<(Vec<u8>, Vec<u8>)>;
type TxStatusList = BTreeMap<String, String>;
type XorNameStr = String;
type SeqMutableDataFake = BTreeMap<String, MDataSeqValue>;

#[derive(Default, Serialize, Deserialize)]
struct FakeData {
    coin_balances: BTreeMap<XorNameStr, SafeKey>,
    txs: BTreeMap<XorNameStr, TxStatusList>, // keep track of TX status per tx ID, per xorname
    published_seq_append_only: BTreeMap<XorNameStr, AppendOnlyDataFake>, // keep a versioned map of data per xorname
    mutable_data: BTreeMap<XorNameStr, SeqMutableDataFake>,
    published_immutable_data: BTreeMap<XorNameStr, Vec<u8>>,
}

#[derive(Default)]
pub struct SafeAppFake {
    fake_vault: FakeData,
}

/// Writes the fake Vault data onto the file
impl Drop for SafeAppFake {
    fn drop(&mut self) {
        let serialised = serde_json::to_string(&self.fake_vault)
            .expect("Failed to serialised fake vault data to write on file");
        trace!("Writing serialised fake vault data = {}", serialised);

        let mut file =
            fs::File::create(&FAKE_VAULT_FILE).expect("Failed to create fake vault DB file");
        let _ = file
            .write(serialised.as_bytes())
            .expect("Failed to write fake vault DB file");
    }
}

impl SafeAppFake {
    // private helper
    fn get_balance_from_xorname(&self, xorname: &XorName) -> Result<Coins> {
        match self.fake_vault.coin_balances.get(&xorname_to_hex(xorname)) {
            None => Err(Error::ContentNotFound("SafeKey data not found".to_string())),
            Some(coin_balance) => parse_coins_amount(&coin_balance.value),
        }
    }

    fn fetch_pk_from_xorname(&self, xorname: &XorName) -> Result<PublicKey> {
        match self.fake_vault.coin_balances.get(&xorname_to_hex(xorname)) {
            None => Err(Error::ContentNotFound("SafeKey data not found".to_string())),
            Some(coin_balance) => Ok(coin_balance.owner),
        }
    }

    async fn substract_coins(&mut self, sk: SecretKey, amount: Coins) -> Result<()> {
        let from_balance = self.get_balance_from_sk(sk.clone()).await?;
        match from_balance.checked_sub(amount) {
            None => Err(Error::NotEnoughBalance(from_balance.to_string())),
            Some(new_balance_coins) => {
                let from_pk = sk.public_key();
                self.fake_vault.coin_balances.insert(
                    xorname_to_hex(&xorname_from_pk(from_pk)),
                    SafeKey {
                        owner: from_pk,
                        value: new_balance_coins.to_string(),
                    },
                );
                Ok(())
            }
        }
    }
}

#[async_trait]
impl SafeApp for SafeAppFake {
    fn new() -> Self {
        let fake_vault = match fs::File::open(&FAKE_VAULT_FILE) {
            Ok(file) => {
                let deserialised: FakeData =
                    serde_json::from_reader(&file).expect("Failed to read fake vault DB file");
                deserialised
            }
            Err(error) => {
                debug!("Error reading mock file. {}", error.to_string());
                FakeData::default()
            }
        };

        Self { fake_vault }
    }

    fn connect(&mut self, _app_id: &str, _auth_credentials: Option<&str>) -> Result<()> {
        debug!("Using mock so there is no connection to network");
        Ok(())
    }

    async fn create_balance(
        &mut self,
        from_sk: Option<SecretKey>,
        new_balance_owner: PublicKey,
        amount: Coins,
    ) -> Result<XorName> {
        if let Some(sk) = from_sk {
            // 1 nano is the creation cost
            let amount_with_cost = Coins::from_nano(amount.as_nano() + 1);
            self.substract_coins(sk, amount_with_cost).await?;
        };

        let to_xorname = xorname_from_pk(new_balance_owner);
        self.fake_vault.coin_balances.insert(
            xorname_to_hex(&to_xorname),
            SafeKey {
                owner: new_balance_owner,
                value: amount.to_string(),
            },
        );

        Ok(to_xorname)
    }

    async fn allocate_test_coins(&mut self, owner_sk: SecretKey, amount: Coins) -> Result<XorName> {
        let to_pk = owner_sk.public_key();
        let xorname = xorname_from_pk(to_pk);
        self.fake_vault.coin_balances.insert(
            xorname_to_hex(&xorname),
            SafeKey {
                owner: (to_pk),
                value: amount.to_string(),
            },
        );

        Ok(xorname)
    }

    async fn get_balance_from_sk(&self, sk: SecretKey) -> Result<Coins> {
        let pk = sk.public_key();
        let xorname = xorname_from_pk(pk);
        self.get_balance_from_xorname(&xorname)
    }

    async fn safecoin_transfer_to_xorname(
        &mut self,
        from_sk: Option<SecretKey>,
        to_xorname: XorName,
        tx_id: TransactionId,
        amount: Coins,
    ) -> Result<Transaction> {
        if amount.as_nano() == 0 {
            return Err(Error::InvalidAmount(amount.to_string()));
        }

        let to_xorname_hex = xorname_to_hex(&to_xorname);

        // generate TX in destination section (to_pk)
        let mut txs_for_xorname = match self.fake_vault.txs.get(&to_xorname_hex) {
            Some(txs) => txs.clone(),
            None => BTreeMap::new(),
        };
        txs_for_xorname.insert(tx_id.to_string(), format!("Success({})", amount));
        self.fake_vault
            .txs
            .insert(to_xorname_hex.clone(), txs_for_xorname);

        if let Some(sk) = from_sk {
            // reduce balance from safecoin_transferer
            self.substract_coins(sk, amount).await?;
        }

        // credit destination
        let to_balance = self.get_balance_from_xorname(&to_xorname)?;
        match to_balance.checked_add(amount) {
            None => Err(Error::Unexpected(
                "Failed to credit destination due to overflow...maybe a millionaire's problem?!"
                    .to_string(),
            )),
            Some(new_balance_coins) => {
                self.fake_vault.coin_balances.insert(
                    to_xorname_hex,
                    SafeKey {
                        owner: self.fetch_pk_from_xorname(&to_xorname)?,
                        value: new_balance_coins.to_string(),
                    },
                );
                Ok(Transaction { id: tx_id, amount })
            }
        }
    }

    #[allow(dead_code)]
    async fn safecoin_transfer_to_pk(
        &mut self,
        from_sk: Option<SecretKey>,
        to_pk: PublicKey,
        tx_id: TransactionId,
        amount: Coins,
    ) -> Result<Transaction> {
        let to_xorname = xorname_from_pk(to_pk);
        self.safecoin_transfer_to_xorname(from_sk, to_xorname, tx_id, amount)
            .await
    }

    #[allow(dead_code)]
    async fn get_transaction(&self, tx_id: u64, pk: PublicKey, _sk: SecretKey) -> Result<String> {
        let xorname = xorname_from_pk(pk);
        let txs_for_xorname = &self.fake_vault.txs[&xorname_to_hex(&xorname)];
        let tx_state = txs_for_xorname.get(&tx_id.to_string()).ok_or_else(|| {
            Error::ContentNotFound(format!("Transaction not found with id '{}'", tx_id))
        })?;
        Ok(tx_state.to_string())
    }

    async fn files_put_published_immutable(
        &mut self,
        data: &[u8],
        dry_run: bool,
    ) -> Result<XorName> {
        // We create a XorName based on a hash of the content, not a real one as
        // it doesn't apply self-encryption, but a unique one for our fake SCL
        let vec_hash = sha3_256(&data);
        let xorname = XorName(vec_hash);

        if !dry_run {
            self.fake_vault
                .published_immutable_data
                .insert(xorname_to_hex(&xorname), data.to_vec());
        }

        Ok(xorname)
    }

    async fn files_get_published_immutable(
        &self,
        xorname: XorName,
        range: Range,
    ) -> Result<Vec<u8>> {
        let data = match self
            .fake_vault
            .published_immutable_data
            .get(&xorname_to_hex(&xorname))
        {
            Some(data) => data.clone(),
            None => {
                return Err(Error::NetDataError(
                    "No ImmutableData found at this address".to_string(),
                ))
            }
        };

        let data = match range {
            Some((start, end)) => data
                [start.unwrap_or_default() as usize..end.unwrap_or(data.len() as u64) as usize]
                .to_vec(),
            None => data.to_vec(),
        };

        Ok(data)
    }

    async fn put_seq_append_only_data(
        &mut self,
        data: Vec<(Vec<u8>, Vec<u8>)>,
        name: Option<XorName>,
        _tag: u64,
        _permissions: Option<String>,
    ) -> Result<XorName> {
        let xorname = name.unwrap_or_else(rand::random);

        self.fake_vault
            .published_seq_append_only
            .insert(xorname_to_hex(&xorname), data);

        Ok(xorname)
    }

    async fn append_seq_append_only_data(
        &mut self,
        data: Vec<(Vec<u8>, Vec<u8>)>,
        _new_version: u64,
        name: XorName,
        _tag: u64,
    ) -> Result<u64> {
        let xorname_hex = xorname_to_hex(&name);
        let mut seq_append_only = match self.fake_vault.published_seq_append_only.get(&xorname_hex)
        {
            Some(seq_append_only) => seq_append_only.clone(),
            None => {
                return Err(Error::ContentNotFound(format!(
                    "Sequenced AppendOnlyData not found at Xor name: {}",
                    xorname_hex
                )))
            }
        };

        seq_append_only.extend(data);
        self.fake_vault
            .published_seq_append_only
            .insert(xorname_hex, seq_append_only.to_vec());

        Ok((seq_append_only.len() - 1) as u64)
    }

    async fn get_latest_seq_append_only_data(
        &self,
        name: XorName,
        _tag: u64,
    ) -> Result<(u64, AppendOnlyDataRawData)> {
        let xorname_hex = xorname_to_hex(&name);
        debug!("Attempting to locate scl mock mdata: {}", xorname_hex);

        match self.fake_vault.published_seq_append_only.get(&xorname_hex) {
            Some(seq_append_only) => {
                let latest_index = seq_append_only.len() - 1;
                let last_entry = seq_append_only.get(latest_index).ok_or_else(|| {
                    Error::EmptyContent(format!(
                        "Empty Sequenced AppendOnlyData found at Xor name {}",
                        xorname_hex
                    ))
                })?;
                Ok(((seq_append_only.len() - 1) as u64, last_entry.clone()))
            }
            None => Err(Error::ContentNotFound(format!(
                "Sequenced AppendOnlyData not found at Xor name: {}",
                xorname_hex
            ))),
        }
    }

    async fn get_current_seq_append_only_data_version(
        &self,
        name: XorName,
        _tag: u64,
    ) -> Result<u64> {
        debug!("Getting seq appendable data, length for: {:?}", name);
        let xorname_hex = xorname_to_hex(&name);
        let length = match self.fake_vault.published_seq_append_only.get(&xorname_hex) {
            Some(seq_append_only) => seq_append_only.len(),
            None => {
                return Err(Error::ContentNotFound(format!(
                    "Sequenced AppendOnlyData not found at Xor name: {}",
                    xorname_hex
                )))
            }
        };

        // return the version
        Ok((length - 1) as u64)
    }

    async fn get_seq_append_only_data(
        &self,
        name: XorName,
        _tag: u64,
        version: u64,
    ) -> Result<AppendOnlyDataRawData> {
        let xorname_hex = xorname_to_hex(&name);
        match self.fake_vault.published_seq_append_only.get(&xorname_hex) {
            Some(seq_append_only) => {
                if version >= seq_append_only.len() as u64 {
                    Err(Error::VersionNotFound(format!(
                        "Invalid version ({}) for Sequenced AppendOnlyData found at Xor name {}",
                        version, name
                    )))
                } else {
                    let index = version as usize;
                    let entry = seq_append_only.get(index).ok_or_else(|| {
                        Error::EmptyContent(format!(
                            "Empty Sequenced AppendOnlyData found at Xor name {}",
                            xorname_hex
                        ))
                    })?;

                    Ok(entry.clone())
                }
            }
            None => Err(Error::ContentNotFound(format!(
                "Sequenced AppendOnlyData not found at Xor name: {}",
                xorname_hex
            ))),
        }
    }

    async fn put_seq_mutable_data(
        &mut self,
        name: Option<XorName>,
        _tag: u64,
        // _data: Option<String>,
        _permissions: Option<String>,
    ) -> Result<XorName> {
        let xorname = name.unwrap_or_else(rand::random);
        let seq_md = match self.fake_vault.mutable_data.get(&xorname_to_hex(&xorname)) {
            Some(uao) => uao.clone(),
            None => BTreeMap::new(),
        };

        self.fake_vault
            .mutable_data
            .insert(xorname_to_hex(&xorname), seq_md);

        Ok(xorname)
    }

    async fn get_seq_mdata(&self, name: XorName, tag: u64) -> Result<SeqMutableData> {
        let xorname_hex = xorname_to_hex(&name);
        debug!("attempting to locate scl mock mdata: {}", xorname_hex);

        match self.fake_vault.mutable_data.get(&xorname_hex) {
            Some(seq_md) => {
                let mut seq_md_with_vec: BTreeMap<Vec<u8>, MDataSeqValue> = BTreeMap::new();
                seq_md.iter().for_each(|(k, v)| {
                    seq_md_with_vec.insert(parse_hex(k), v.clone());
                });

                Ok(SeqMutableData::new_with_data(
                    name,
                    tag,
                    seq_md_with_vec,
                    BTreeMap::default(),
                    SafeNdPublicKey::Bls(SecretKey::random().public_key()),
                ))
            }
            None => Err(Error::ContentNotFound(format!(
                "Sequenced MutableData not found at Xor name: {}",
                xorname_hex
            ))),
        }
    }

    async fn seq_mutable_data_insert(
        &mut self,
        name: XorName,
        tag: u64,
        key: &[u8],
        value: &[u8],
    ) -> Result<()> {
        let seq_md = self.get_seq_mdata(name, tag).await?;
        let mut data = seq_md.entries().clone();

        data.insert(
            key.to_vec(),
            MDataSeqValue {
                data: value.to_vec(),
                version: 0,
            },
        );

        let mut seq_md_with_str: BTreeMap<String, MDataSeqValue> = BTreeMap::new();
        data.iter().for_each(|(k, v)| {
            seq_md_with_str.insert(vec_to_hex(k.to_vec()), v.clone());
        });
        self.fake_vault
            .mutable_data
            .insert(xorname_to_hex(&name), seq_md_with_str);

        Ok(())
    }

    async fn seq_mutable_data_get_value(
        &self,
        name: XorName,
        tag: u64,
        key: &[u8],
    ) -> Result<MDataSeqValue> {
        let seq_md = self.get_seq_mdata(name, tag).await?;
        match seq_md.get(&key.to_vec()) {
            Some(value) => Ok(value.clone()),
            None => Err(Error::EntryNotFound(format!(
                "Entry not found in Sequenced MutableData found at Xor name: {}",
                xorname_to_hex(&name)
            ))),
        }
    }

    async fn list_seq_mdata_entries(
        &self,
        name: XorName,
        tag: u64,
    ) -> Result<BTreeMap<Vec<u8>, MDataSeqValue>> {
        debug!("Listing seq_mdata_entries for: {}", name);
        let seq_md = self.get_seq_mdata(name, tag).await?;
        let mut res = BTreeMap::new();
        seq_md.entries().iter().for_each(|elem| {
            res.insert(elem.0.clone(), elem.1.clone());
        });

        Ok(res)
    }

    async fn seq_mutable_data_update(
        &mut self,
        name: XorName,
        tag: u64,
        key: &[u8],
        value: &[u8],
        _version: u64,
    ) -> Result<()> {
        let _ = self.seq_mutable_data_get_value(name, tag, key).await;
        self.seq_mutable_data_insert(name, tag, key, value).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_test_coins() {
        use std::str::FromStr;
        use threshold_crypto::SecretKey;
        use unwrap::unwrap;

        let mut mock = SafeAppFake::new();

        let sk_to = SecretKey::random();

        async_std::task::block_on(async {
            let balance = unwrap!(Coins::from_str("2.345678912"));
            unwrap!(mock.allocate_test_coins(sk_to.clone(), balance).await);
            let current_balance = unwrap!(mock.get_balance_from_sk(sk_to).await);
            assert_eq!(balance, current_balance);
        });
    }

    #[test]
    fn test_create_balance() {
        use std::str::FromStr;
        use threshold_crypto::SecretKey;
        use unwrap::unwrap;

        let mut mock = SafeAppFake::new();

        let sk = SecretKey::random();

        async_std::task::block_on(async {
            let balance = unwrap!(Coins::from_str("2.345678912"));
            unwrap!(mock.allocate_test_coins(sk.clone(), balance).await);

            let sk_to = SecretKey::random();
            let pk_to = sk_to.public_key();
            assert!(mock
                .create_balance(Some(sk), pk_to, unwrap!(Coins::from_str("1.234567891")))
                .await
                .is_ok());
        });
    }

    #[test]
    fn test_check_balance() {
        use std::str::FromStr;
        use threshold_crypto::SecretKey;
        use unwrap::unwrap;

        let mut mock = SafeAppFake::new();

        let sk = SecretKey::random();

        async_std::task::block_on(async {
            let balance = unwrap!(Coins::from_str("2.3"));
            unwrap!(mock.allocate_test_coins(sk.clone(), balance).await);
            let current_balance = unwrap!(mock.get_balance_from_sk(sk.clone()).await);
            assert_eq!(balance, current_balance);

            let sk_to = SecretKey::random();
            let pk_to = sk_to.public_key();
            let preload = unwrap!(Coins::from_str("1.234567891"));
            unwrap!(mock.create_balance(Some(sk.clone()), pk_to, preload).await);
            let current_balance = unwrap!(mock.get_balance_from_sk(sk_to).await);
            assert_eq!(preload, current_balance);

            let current_balance = unwrap!(mock.get_balance_from_sk(sk).await);
            assert_eq!(
                unwrap!(Coins::from_str("1.065432108")), /* == 2.3 - 1.234567891 - 0.000000001 (creation cost) */
                current_balance
            );
        });
    }

    #[test]
    fn test_safecoin_transfer() {
        use rand_core::RngCore;
        use std::str::FromStr;
        use threshold_crypto::SecretKey;
        use unwrap::unwrap;

        let mut mock = SafeAppFake::new();

        let sk1 = SecretKey::random();

        let sk2 = SecretKey::random();
        let pk2 = sk2.public_key();

        async_std::task::block_on(async {
            let balance1 = unwrap!(Coins::from_str("2.5"));
            let balance2 = unwrap!(Coins::from_str("5.7"));
            unwrap!(mock.allocate_test_coins(sk1.clone(), balance1).await);
            unwrap!(mock.allocate_test_coins(sk2.clone(), balance2).await);

            let curr_balance1 = unwrap!(mock.get_balance_from_sk(sk1.clone()).await);
            let curr_balance2 = unwrap!(mock.get_balance_from_sk(sk2.clone()).await);

            assert_eq!(balance1, curr_balance1);
            assert_eq!(balance2, curr_balance2);

            let mut rng = rand::thread_rng();
            let tx_id = rng.next_u64();

            let _ = unwrap!(
                mock.safecoin_transfer_to_xorname(
                    Some(sk1.clone()),
                    xorname_from_pk(pk2),
                    tx_id,
                    unwrap!(Coins::from_str("1.4"))
                )
                .await
            );
            unwrap!(mock.get_transaction(tx_id, pk2, sk2.clone()).await);

            let curr_balance1 = unwrap!(mock.get_balance_from_sk(sk1).await);
            let curr_balance2 = unwrap!(mock.get_balance_from_sk(sk2).await);

            assert_eq!(curr_balance1, unwrap!(Coins::from_str("1.1")));
            assert_eq!(curr_balance2, unwrap!(Coins::from_str("7.1")));
        });
    }
}
