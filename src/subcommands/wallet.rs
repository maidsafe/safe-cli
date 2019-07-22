// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use structopt::StructOpt;

use super::helpers::{get_from_arg_or_stdin, get_secret_key};
use super::keys::create_new_key;
use super::OutputFmt;
use log::debug;
use safe_cli::{BlsKeyPair, Safe};
use unwrap::unwrap;

#[derive(StructOpt, Debug)]
pub enum WalletSubCommands {
    #[structopt(name = "insert")]
    /// Insert a spendable balance into a Wallet
    Insert {
        /// The source Wallet for funds
        source: Option<String>,
        /// The target Wallet to insert the spendable balance
        target: Option<String>,
        /// The name to give this spendable balance
        #[structopt(long = "name")]
        name: Option<String>,
        /// The Key's safe://xor-url to verify it matches/corresponds to the secret key provided. The corresponding secret key will be prompted if not provided with '--sk'.
        #[structopt(long = "keyurl")]
        keyurl: Option<String>,
        /// Set the inserted Key as the default one in the target Wallet
        #[structopt(long = "default")]
        default: bool,
        /// Pass the secret key to make the balance spendable, it will be prompted if not provided
        #[structopt(long = "sk")]
        secret: Option<String>,
    },
    #[structopt(name = "balance")]
    /// Query a Wallet's total balance
    Balance {
        /// The target Wallet to check the total balance
        target: Option<String>,
    },
    #[structopt(name = "check-tx")]
    /// Check the status of a given transaction
    CheckTx {},
    #[structopt(name = "create")]
    /// Create a new Wallet
    Create {
        /// The source Wallet for funds
        source: Option<String>,
        /// If true, do not create a spendable balance
        #[structopt(long = "no-balance")]
        no_balance: bool,
        /// The name to give the spendable balance
        #[structopt(long = "name")]
        name: Option<String>,
        /// An existing Key's safe://xor-url. If this is not supplied, a new Key will be automatically generated and inserted. The corresponding secret key will be prompted if not provided with '--sk'.
        #[structopt(long = "keyurl")]
        keyurl: Option<String>,
        /// Pass the secret key to make the balance spendable, it will be prompted if not provided
        #[structopt(long = "sk")]
        secret: Option<String>,
        /// Create a Key, allocate test-coins onto it, and add the Key to the Wallet
        #[structopt(long = "test-coins")]
        test_coins: bool,
        /// Preload the key with a balance
        #[structopt(long = "preload")]
        preload: Option<String>,
    },
    #[structopt(name = "transfer")]
    /// Transfer safecoins from one Wallet, Key or pk, to another
    Transfer {
        /// Number of safecoins to transfer
        amount: String,
        /// target Wallet
        to: String,
        /// source Wallet, or pulled from stdin if not present
        from: Option<String>,
    },
    #[structopt(name = "sweep")]
    /// Move all coins within a Wallet to a second given Wallet or Key
    Sweep {
        /// The source Wallet for funds
        #[structopt(long = "from")]
        from: String,
        /// The receiving Wallet/Key
        #[structopt(long = "to")]
        to: String,
    },
}

pub fn wallet_commander(
    cmd: Option<WalletSubCommands>,
    output_fmt: OutputFmt,
    safe: &mut Safe,
) -> Result<(), String> {
    match cmd {
        Some(WalletSubCommands::Create {
            preload,
            test_coins,
            no_balance,
            keyurl,
            name,
            source,
            secret,
        }) => {
            // create wallet
            let wallet_xorname = safe.wallet_create()?;

            if !no_balance {
                // get or create keypair
                let (xorurl, key_pair) = match keyurl {
                    Some(linked_key) => {
                        let sk = get_secret_key(&linked_key, secret)?;
                        let pk = safe.validate_sk_for_xorurl(&sk, &linked_key)?;

                        (linked_key, Some(BlsKeyPair { pk, sk }))
                    }
                    None => create_new_key(safe, test_coins, source, preload, None, output_fmt)?,
                };

                let the_name = match name {
                    Some(name_str) => name_str.to_string(),
                    None => xorurl.clone(),
                };

                // insert and set as default
                safe.wallet_insert(
                    &wallet_xorname,
                    &the_name,
                    true,
                    &unwrap!(key_pair),
                    &xorurl,
                )?;
            }

            if OutputFmt::Pretty == output_fmt {
                println!("Wallet created at: \"{}\"", &wallet_xorname);
            } else {
                println!("{}", &wallet_xorname);
            }
            Ok(())
        }
        Some(WalletSubCommands::Balance { target }) => {
            let target = get_from_arg_or_stdin(target, None)?;

            debug!("Got target location {:?}", target);
            let balance = safe.wallet_balance(&target)?;

            if OutputFmt::Pretty == output_fmt {
                println!(
                    "Wallet at \"{}\" has a total balance of {} safecoins",
                    target, balance
                );
            } else {
                println!("{}", balance);
            }

            Ok(())
        }
        Some(WalletSubCommands::Insert {
            target,
            keyurl,
            name,
            default,
            secret,
            ..
        }) => {
            let target = get_from_arg_or_stdin(target, None)?;

            let (xorurl, key_pair) = {
                let url = keyurl.unwrap_or_else(|| "".to_string());
                let sk = get_secret_key(&url, secret)?;
                let pk = safe.validate_sk_for_xorurl(&sk, &url)?;

                (url, Some(BlsKeyPair { pk, sk }))
            };

            let the_name = match name {
                Some(name_str) => name_str,
                None => xorurl.clone(),
            };

            safe.wallet_insert(&target, &the_name, default, &unwrap!(key_pair), &xorurl)?;
            if OutputFmt::Pretty == output_fmt {
                println!(
                    "Spendable balance inserted with name '{}' in Wallet located at \"{}\"",
                    the_name, target
                );
            } else {
                println!("{}", target);
            }
            Ok(())
        }
        Some(WalletSubCommands::Transfer { amount, from, to }) => {
            //TODO: if from/to start without safe://, i.e. if they are PK hex strings.
            let tx_id = safe.wallet_transfer(&amount, from, &to)?;

            if OutputFmt::Pretty == output_fmt {
                println!("Success. TX_ID: {:?}", &tx_id);
            } else {
                println!("{}", &tx_id)
            }

            Ok(())
        }
        _ => Err("Sub-command not supported yet".to_string()),
    }
}
