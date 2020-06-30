// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use super::{xorurl::SafeContentType, Safe, SafeApp};
use crate::{
    xorurl::{SafeDataType, XorUrl, XorUrlEncoder},
    Error, Result,
};
use log::debug;
use safe_nd::XorName;

impl Safe {
    /// Create a Public Sequence on the network
    ///
    /// ## Example
    /// ```
    /// # use safe_api::Safe;
    /// # let mut safe = Safe::default();
    /// # async_std::task::block_on(async {
    /// #   safe.connect("", Some("fake-credentials")).await.unwrap();
    ///     let data = b"First in the sequence";
    ///     let xorurl = safe.sequence_create(data, None, 20_000, false).await.unwrap();
    ///     let received_data = safe.sequence_get(&xorurl).await.unwrap();
    ///     assert_eq!(received_data, (0, data.to_vec()));
    /// # });
    /// ```
    pub async fn sequence_create(
        &mut self,
        data: &[u8],
        name: Option<XorName>,
        type_tag: u64,
        private: bool,
    ) -> Result<XorUrl> {
        let xorname = self
            .safe_app
            .store_sequence_data(data, name, type_tag, None, private)
            .await?;

        XorUrlEncoder::encode_sequence_data(
            xorname,
            type_tag,
            SafeContentType::Raw,
            self.xorurl_base,
            private,
        )
    }

    /// Get data from a Public Sequence on the network
    ///
    /// ## Example
    /// ```
    /// # use safe_api::Safe;
    /// # let mut safe = Safe::default();
    /// # async_std::task::block_on(async {
    /// #   safe.connect("", Some("fake-credentials")).await.unwrap();
    ///     let data = b"First in the sequence";
    ///     let xorurl = safe.sequence_create(data, None, 20_000, false).await.unwrap();
    ///     let received_data = safe.sequence_get(&xorurl).await.unwrap();
    ///     assert_eq!(received_data, (0, data.to_vec()));
    /// # });
    /// ```
    pub async fn sequence_get(&self, url: &str) -> Result<(u64, Vec<u8>)> {
        debug!("Getting Public Sequence data from: {:?}", url);
        let (xorurl_encoder, _) = self.parse_and_resolve_url(url).await?;

        self.fetch_sequence(&xorurl_encoder).await
    }

    /// Fetch a Sequence from a XorUrlEncoder without performing any type of URL resolution
    pub(crate) async fn fetch_sequence(
        &self,
        xorurl_encoder: &XorUrlEncoder,
    ) -> Result<(u64, Vec<u8>)> {
        let is_private = xorurl_encoder.data_type() == SafeDataType::PrivateSequence;
        let data = match xorurl_encoder.content_version() {
            Some(version) => {
                // We fetch a specific entry since the URL specifies a specific version
                let data = self
                    .safe_app
                    .sequence_get_entry(
                        xorurl_encoder.xorname(),
                        xorurl_encoder.type_tag(),
                        version,
                        is_private,
                    )
                    .await
                    .map_err(|err| {
                        if let Error::VersionNotFound(_) = err {
                            Error::VersionNotFound(format!(
                                "Version '{}' is invalid for the Sequence found at \"{}\"",
                                version, xorurl_encoder,
                            ))
                        } else {
                            err
                        }
                    })?;
                Ok((version, data))
            }
            None => {
                // ...then get last entry in the Sequence
                self.safe_app
                    .sequence_get_last_entry(
                        xorurl_encoder.xorname(),
                        xorurl_encoder.type_tag(),
                        is_private,
                    )
                    .await
            }
        };

        match data {
            Ok((version, value)) => {
                debug!("Sequence retrieved... v{}", &version);
                Ok((version, value))
            }
            Err(Error::EmptyContent(_)) => Err(Error::EmptyContent(format!(
                "Sequence found at \"{}\" was empty",
                xorurl_encoder
            ))),
            Err(Error::ContentNotFound(_)) => Err(Error::ContentNotFound(
                "No Sequence found at this address".to_string(),
            )),
            other => other,
        }
    }

    /// Append data to a Public Sequence on the network
    ///
    /// ## Example
    /// ```
    /// # use safe_api::Safe;
    /// # let mut safe = Safe::default();
    /// # async_std::task::block_on(async {
    /// #   safe.connect("", Some("fake-credentials")).await.unwrap();
    ///     let data1 = b"First in the sequence";
    ///     let xorurl = safe.sequence_create(data1, None, 20_000, false).await.unwrap();
    ///     let data2 = b"Second in the sequence";
    ///     safe.sequence_append(&xorurl, data2).await.unwrap();
    ///     let received_data = safe.sequence_get(&xorurl).await.unwrap();
    ///     assert_eq!(received_data, (1, data2.to_vec()));
    /// # });
    /// ```
    pub async fn sequence_append(&mut self, url: &str, data: &[u8]) -> Result<()> {
        let xorurl_encoder = Safe::parse_url(url)?;
        if xorurl_encoder.content_version().is_some() {
            return Err(Error::InvalidInput(format!(
                "The target URL cannot cannot contain a version: {}",
                url
            )));
        };

        let (xorurl_encoder, _) = self.parse_and_resolve_url(url).await?;

        let xorname = xorurl_encoder.xorname();
        let type_tag = xorurl_encoder.type_tag();
        let is_private = xorurl_encoder.data_type() == SafeDataType::PrivateSequence;
        self.safe_app
            .sequence_append(data, xorname, type_tag, is_private)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::app::test_helpers::new_safe_instance;

    #[tokio::test]
    async fn test_sequence_create() -> Result<()> {
        let mut safe = new_safe_instance().await?;
        let initial_data = b"initial data";

        let xorurl = safe
            .sequence_create(initial_data, None, 25_000, false)
            .await?;
        let xorurl_priv = safe
            .sequence_create(initial_data, None, 25_000, true)
            .await?;

        let received_data = safe.sequence_get(&xorurl).await?;
        let received_data_priv = safe.sequence_get(&xorurl_priv).await?;
        assert_eq!(received_data, (0, initial_data.to_vec()));
        assert_eq!(received_data_priv, (0, initial_data.to_vec()));
        Ok(())
    }

    #[tokio::test]
    async fn test_sequence_append() -> Result<()> {
        let mut safe = new_safe_instance().await?;
        let data_v0 = b"First in the sequence";
        let data_v1 = b"Second in the sequence";

        let xorurl = safe.sequence_create(data_v0, None, 25_000, false).await?;
        safe.sequence_append(&xorurl, data_v1).await?;

        let xorurl_priv = safe.sequence_create(data_v0, None, 25_000, true).await?;
        safe.sequence_append(&xorurl_priv, data_v1).await?;

        let received_data_v0 = safe.sequence_get(&format!("{}?v=0", xorurl)).await?;
        let received_data_v1 = safe.sequence_get(&xorurl).await?;
        assert_eq!(received_data_v0, (0, data_v0.to_vec()));
        assert_eq!(received_data_v1, (1, data_v1.to_vec()));

        let received_data_v0_priv = safe.sequence_get(&format!("{}?v=0", xorurl)).await?;
        let received_data_v1_priv = safe.sequence_get(&xorurl).await?;
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
            .sequence_create(data_v0, None, 25_000, false)
            .await?;
        client1.sequence_append(&xorurl, data_v1).await?;

        let client2 = new_safe_instance().await?;
        let received_data_v0 = client2.sequence_get(&format!("{}?v=0", xorurl)).await?;
        let received_data_v1 = client2.sequence_get(&xorurl).await?;
        assert_eq!(received_data_v0, (0, data_v0.to_vec()));
        assert_eq!(received_data_v1, (1, data_v1.to_vec()));
        Ok(())
    }

    #[tokio::test]
    #[cfg(not(feature = "scl-mock"))]
    async fn test_sequence_append_concurrently_from_second_client() -> Result<()> {
        let mut client1 = new_safe_instance().await?;
        let mut client2 = new_safe_instance().await?;
        let data_v0 = b"First from client1";
        let data_v1 = b"First from client2";

        let xorurl = client1
            .sequence_create(data_v0, None, 25_000, false)
            .await?;
        client2.sequence_append(&xorurl, data_v1).await?;

        let received_client1 = client1.sequence_get(&xorurl).await?;
        let received_client2 = client2.sequence_get(&xorurl).await?;

        // client1 sees only data_v0 as version 0 since it's using its own replica
        // it didn't see the append from client2 even it was merged on the network
        assert_eq!(received_client1, (0, data_v0.to_vec()));

        // client2 sees data_v1 as version 1 since it fetched v0 before appending
        assert_eq!(received_client2, (1, data_v1.to_vec()));

        // a third client should see all versions now since it'll fetch from the
        // replicas on the network which have merged all appends from client1 and client2,
        let client3 = new_safe_instance().await?;
        let received_v0 = client3.sequence_get(&format!("{}?v=0", xorurl)).await?;
        assert_eq!((0, data_v0.to_vec()), received_v0);

        let received_v1 = client3.sequence_get(&xorurl).await?;
        assert_eq!((1, data_v1.to_vec()), received_v1);

        Ok(())
    }
}
