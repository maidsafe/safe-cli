// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

pub use sn_data_types::register::{Entry, EntryHash};

use super::safeurl::{SafeContentType, SafeDataType, SafeUrl, XorUrl};
use crate::{Error, Result, Safe};
use log::debug;
use std::collections::BTreeSet;
use xor_name::XorName;

impl Safe {
    /// Create a Public Register on the network
    ///
    /// ## Example
    /// ```
    /// # use sn_api::Safe;
    /// # let mut safe = Safe::default();
    /// # async_std::task::block_on(async {
    /// #   safe.connect("", Some("fake-credentials")).await.unwrap();
    ///     let data = b"First in the sequence";
    ///     let xorurl = safe.register_create(data, None, 20_000, false).await.unwrap();
    ///     let received_data = safe.register_read(&xorurl).await.unwrap();
    ///     assert_eq!(received_data, (0, data.to_vec()));
    /// # });
    /// ```
    pub async fn register_create(
        &mut self,
        data: &[u8],
        name: Option<XorName>,
        type_tag: u64,
        private: bool,
    ) -> Result<(XorUrl, EntryHash)> {
        let (xorname, hash) = self
            .safe_client
            .store_register(data, name, type_tag, None, private)
            .await?;

        let xorurl = SafeUrl::encode_register_url(
            xorname,
            type_tag,
            SafeContentType::Raw,
            self.xorurl_base,
            private,
        )?;

        Ok((xorurl, hash))
    }

    /// Get data from a Public Register on the network
    ///
    /// ## Example
    /// ```
    /// # use sn_api::Safe;
    /// # let mut safe = Safe::default();
    /// # async_std::task::block_on(async {
    /// #   safe.connect("", Some("fake-credentials")).await.unwrap();
    ///     let data = b"First in the sequence";
    ///     let xorurl = safe.register_create(data, None, 20_000, false).await.unwrap();
    ///     let received_data = safe.register_read(&xorurl).await.unwrap();
    ///     assert_eq!(received_data, (0, data.to_vec()));
    /// # });
    /// ```
    pub async fn register_read(&mut self, url: &str) -> Result<BTreeSet<(EntryHash, Entry)>> {
        debug!("Getting Public Register data from: {:?}", url);
        let (safeurl, _) = self.parse_and_resolve_url(url).await?;

        self.fetch_register_value(&safeurl).await
    }

    /// Fetch a Register from a SafeUrl without performing any type of URL resolution
    pub(crate) async fn fetch_register_value(
        &mut self,
        safeurl: &SafeUrl,
    ) -> Result<BTreeSet<(EntryHash, Entry)>> {
        let is_private = safeurl.data_type() == SafeDataType::PrivateRegister;
        let data = match safeurl.content_hash() {
            Some(hash) => {
                // We fetch a specific entry since the URL specifies a specific hash
                let data = self
                    .safe_client
                    .get_register_entry(safeurl.xorname(), safeurl.type_tag(), hash, is_private)
                    .await?;

                Ok(vec![(hash, data)].into_iter().collect())
            }
            None => {
                // ...then get last entry from the Register
                self.safe_client
                    .read_register(safeurl.xorname(), safeurl.type_tag(), is_private)
                    .await
            }
        };

        match data {
            Ok(data) => {
                debug!("Register retrieved...");
                Ok(data)
            }
            Err(Error::EmptyContent(_)) => Err(Error::EmptyContent(format!(
                "Register found at \"{}\" was empty",
                safeurl
            ))),
            Err(Error::ContentNotFound(_)) => Err(Error::ContentNotFound(
                "No Register found at this address".to_string(),
            )),
            other => other,
        }
    }

    /// Write data to a Public Register on the network
    ///
    /// ## Example
    /// ```
    /// # use sn_api::Safe;
    /// # let mut safe = Safe::default();
    /// # async_std::task::block_on(async {
    /// #   safe.connect("", Some("fake-credentials")).await.unwrap();
    ///     let data1 = b"First in the sequence";
    ///     let xorurl = safe.register_create(data1, None, 20_000, false).await.unwrap();
    ///     let data2 = b"Second in the sequence";
    ///     safe.append_to_sequence(&xorurl, data2).await.unwrap();
    ///     let received_data = safe.register_read(&xorurl).await.unwrap();
    ///     assert_eq!(received_data, (1, data2.to_vec()));
    /// # });
    /// ```
    pub async fn write_to_register(&mut self, url: &str, data: &[u8]) -> Result<EntryHash> {
        let safeurl = Safe::parse_url(url)?;
        if safeurl.content_hash().is_some() {
            // TODO: perhaps we can allow this instad, and that's how an
            // application can specify the parent entry in the Register.
            return Err(Error::InvalidInput(format!(
                "The target URL cannot contain a content hash: {}",
                url
            )));
        };

        let (safeurl, _) = self.parse_and_resolve_url(url).await?;

        let xorname = safeurl.xorname();
        let type_tag = safeurl.type_tag();
        let is_private = safeurl.data_type() == SafeDataType::PrivateRegister;

        // write the data to the Register
        self.safe_client
            .write_to_register(data, xorname, type_tag, is_private, BTreeSet::new())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::{api::app::test_helpers::new_safe_instance, retry_loop};
    use anyhow::Result;
    /*
    #[tokio::test]
    async fn test_register_create() -> Result<()> {
        let mut safe = new_safe_instance().await?;
        let initial_data = b"initial data";

        let xorurl = safe
            .register_create(initial_data, None, 25_000, false)
            .await?;
        let xorurl_priv = safe
            .register_create(initial_data, None, 25_000, true)
            .await?;

        let received_data = retry_loop!(safe.register_read(&xorurl));
        let received_data_priv = retry_loop!(safe.register_read(&xorurl_priv));
        assert_eq!(received_data, (0, initial_data.to_vec()));
        assert_eq!(received_data_priv, (0, initial_data.to_vec()));
        Ok(())
    }

    #[tokio::test]
    async fn test_sequence_append() -> Result<()> {
        let mut safe = new_safe_instance().await?;
        let data_v0 = b"First in the sequence";
        let data_v1 = b"Second in the sequence";

        let xorurl = safe.register_create(data_v0, None, 25_000, false).await?;
        let xorurl_priv = safe.register_create(data_v0, None, 25_000, true).await?;

        let _ = retry_loop!(safe.register_read(&xorurl));
        safe.append_to_sequence(&xorurl, data_v1).await?;

        let _ = retry_loop!(safe.register_read(&xorurl_priv));
        safe.append_to_sequence(&xorurl_priv, data_v1).await?;

        let received_data_v0 = safe.register_read(&format!("{}?v=0", xorurl)).await?;
        let received_data_v1 = safe.register_read(&xorurl).await?;
        assert_eq!(received_data_v0, (0, data_v0.to_vec()));
        assert_eq!(received_data_v1, (1, data_v1.to_vec()));

        let received_data_v0_priv = safe.register_read(&format!("{}?v=0", xorurl)).await?;
        let received_data_v1_priv = safe.register_read(&xorurl).await?;
        assert_eq!(received_data_v0_priv, (0, data_v0.to_vec()));
        assert_eq!(received_data_v1_priv, (1, data_v1.to_vec()));
        Ok(())
    }

    #[tokio::test]
    async fn test_sequence_read_from_second_client() -> Result<()> {
        let mut client1 = new_safe_instance().await?;
        let data_v0 = b"First in the sequence";
        let data_v1 = b"Second in the sequence";

        let xorurl = client1
            .register_create(data_v0, None, 25_000, false)
            .await?;
        let _ = retry_loop!(client1.register_read(&xorurl));
        client1.append_to_sequence(&xorurl, data_v1).await?;

        let mut client2 = new_safe_instance().await?;
        let received_data_v0 = client2.register_read(&format!("{}?v=0", xorurl)).await?;
        let received_data_v1 = client2.register_read(&xorurl).await?;
        assert_eq!(received_data_v0, (0, data_v0.to_vec()));
        assert_eq!(received_data_v1, (1, data_v1.to_vec()));
        Ok(())
    }

    #[tokio::test]
    async fn test_sequence_append_concurrently_from_second_client() -> Result<()> {
        let mut client1 = new_safe_instance().await?;
        let mut client2 = new_safe_instance().await?;
        // this tests assumes the same credentials/keypair have been set for all tests,
        // so both instances should be using the same keypair to sign the messages
        assert_eq!(
            client1.get_my_keypair().await?,
            client1.get_my_keypair().await?
        );

        let data_v0 = b"First from client1";
        let data_v1 = b"First from client2";

        let xorurl = client1
            .register_create(data_v0, None, 25_000, false)
            .await?;

        let received_client1 = retry_loop!(client1.register_read(&xorurl));

        client2.append_to_sequence(&xorurl, data_v1).await?;

        let received_client2 = retry_loop!(client2.register_read(&xorurl));

        // client1 sees only data_v0 as version 0 since it's using its own replica
        // it didn't see the append from client2 even it was merged on the network
        assert_eq!(received_client1, (0, data_v0.to_vec()));

        // client2 sees data_v1 as version 1 since it fetched v0 before appending
        assert_eq!(received_client2, (1, data_v1.to_vec()));

        // a third client should see all versions now since it'll fetch from the
        // replicas on the network which have merged all appends from client1 and client2,
        let mut client3 = new_safe_instance().await?;
        let received_v0 = client3.register_read(&format!("{}?v=0", xorurl)).await?;
        assert_eq!((0, data_v0.to_vec()), received_v0);

        let received_v1 = client3.register_read(&xorurl).await?;
        assert_eq!((1, data_v1.to_vec()), received_v1);

        Ok(())
    }
    */
}
