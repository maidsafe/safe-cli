// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use super::{helpers::gen_timestamp_nanos, xorurl::SafeContentType, Safe, SafeApp};
use crate::{
    xorurl::{XorUrl, XorUrlEncoder},
    Error, Result,
};
use log::debug;

// Type tag to use for the Sequence
// Note this may not be needed when new Sequence data type is supported in vaults
const PUBLIC_SEQUENCE_TYPE_TAG: u64 = 1_200;

impl Safe {
    /// Create a Public Sequence on the network
    ///
    /// ## Example
    /// ```
    /// # use safe_api::Safe;
    /// # let mut safe = Safe::default();
    /// # safe.connect("", Some("fake-credentials")).unwrap();
    /// # async_std::task::block_on(async {
    ///     let data = b"First in the sequence";
    ///     let xorurl = safe.sequence_create(Some(data)).await.unwrap();
    ///     let received_data = safe.sequence_get(&xorurl).await.unwrap();
    ///     assert_eq!(received_data, (0, data.to_vec()));
    /// # });
    /// ```
    pub async fn sequence_create(&mut self, data: Option<&[u8]>) -> Result<XorUrl> {
        let seq_data = match data {
            Some(data) => {
                let now = gen_timestamp_nanos();
                vec![(now.into_bytes().to_vec(), data.to_vec())]
            }
            None => vec![],
        };
        let xorname = self
            .safe_app
            .put_seq_append_only_data(seq_data, None, PUBLIC_SEQUENCE_TYPE_TAG, None)
            .await?;

        XorUrlEncoder::encode_append_only_data(
            xorname,
            PUBLIC_SEQUENCE_TYPE_TAG,
            SafeContentType::Raw,
            self.xorurl_base,
        )
    }

    /// Get data from a Public Sequence on the network
    ///
    /// ## Example
    /// ```
    /// # use safe_api::Safe;
    /// # let mut safe = Safe::default();
    /// # safe.connect("", Some("fake-credentials")).unwrap();
    /// # async_std::task::block_on(async {
    ///     let data = b"First in the sequence";
    ///     let xorurl = safe.sequence_create(Some(data)).await.unwrap();
    ///     let received_data = safe.sequence_get(&xorurl).await.unwrap();
    ///     assert_eq!(received_data, (0, data.to_vec()));
    /// # });
    /// ```
    pub async fn sequence_get(&self, url: &str) -> Result<(u64, Vec<u8>)> {
        debug!("Getting Public Sequence data from: {:?}", url);
        let (xorurl_encoder, _) = self.parse_and_resolve_url(url).await?;

        // Check if the URL specifies a specific version of the content or simply the latest available
        let data = match xorurl_encoder.content_version() {
            None => {
                self.safe_app
                    .get_latest_seq_append_only_data(
                        xorurl_encoder.xorname(),
                        xorurl_encoder.type_tag(),
                    )
                    .await
            }
            Some(content_version) => {
                let (key, value) = self
                    .safe_app
                    .get_seq_append_only_data(
                        xorurl_encoder.xorname(),
                        xorurl_encoder.type_tag(),
                        content_version,
                    )
                    .await
                    .map_err(|err| {
                        if let Error::VersionNotFound(_) = err {
                            Error::VersionNotFound(format!(
                                "Version '{}' is invalid for the Sequence found at \"{}\"",
                                content_version, url,
                            ))
                        } else {
                            err
                        }
                    })?;
                Ok((content_version, (key, value)))
            }
        };

        match data {
            Ok((version, (_key, value))) => {
                debug!("Sequence retrieved... v{}", &version);
                Ok((version, value.to_vec()))
            }
            Err(Error::EmptyContent(_)) => Err(Error::EmptyContent(format!(
                "Sequence found at \"{}\" was empty",
                url
            ))),
            Err(Error::ContentNotFound(_)) => Err(Error::ContentNotFound(
                "No Sequence found at this address".to_string(),
            )),
            Err(Error::VersionNotFound(msg)) => Err(Error::VersionNotFound(msg)),
            Err(err) => Err(Error::NetDataError(format!(
                "Failed to get current version: {}",
                err
            ))),
        }
    }

    /// Append data to a Public Sequence on the network
    ///
    /// ## Example
    /// ```
    /// # use safe_api::Safe;
    /// # let mut safe = Safe::default();
    /// # safe.connect("", Some("fake-credentials")).unwrap();
    /// # async_std::task::block_on(async {
    ///     let data1 = b"First in the sequence";
    ///     let xorurl = safe.sequence_create(Some(data1)).await.unwrap();
    ///     let data2 = b"Second in the sequence";
    ///     let new_version = safe.sequence_append(&xorurl, data2).await.unwrap();
    ///     let received_data = safe.sequence_get(&xorurl).await.unwrap();
    ///     assert_eq!(received_data, (new_version, data2.to_vec()));
    /// # });
    /// ```
    pub async fn sequence_append(&mut self, url: &str, data: &[u8]) -> Result<u64> {
        let xorurl_encoder = Safe::parse_url(url)?;
        if xorurl_encoder.content_version().is_some() {
            return Err(Error::InvalidInput(format!(
                "The target URL cannot cannot contain a version: {}",
                url
            )));
        };

        let (mut xorurl_encoder, _) = self.parse_and_resolve_url(url).await?;

        // If the FilesContainer URL was resolved from an NRS name we need to remove
        // the version from it so we can fetch latest version of it
        xorurl_encoder.set_content_version(None);
        let (current_version, _): (u64, Vec<u8>) =
            self.sequence_get(&xorurl_encoder.to_string()?).await?;

        let now = gen_timestamp_nanos();
        let seq_data = vec![(now.into_bytes().to_vec(), data.to_vec())];

        let xorname = xorurl_encoder.xorname();
        let type_tag = xorurl_encoder.type_tag();
        let new_version = self
            .safe_app
            .append_seq_append_only_data(seq_data, current_version + 1, xorname, type_tag)
            .await?;
        Ok(new_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::app::test_helpers::new_safe_instance;

    #[tokio::test]
    async fn test_sequence_create_empty() -> Result<()> {
        let mut safe = new_safe_instance()?;
        let xorurl = safe.sequence_create(None).await?;
        match safe.sequence_get(&xorurl).await {
            Ok(_) => Err(Error::Unexpected(
                "Unexpectedly fetched Sequence".to_string(),
            )),
            Err(Error::EmptyContent(msg)) => {
                assert_eq!(msg, format!("Sequence found at \"{}\" was empty", xorurl));
                Ok(())
            }
            other => Err(Error::Unexpected(format!(
                "Error returned is not the expected one: {:?}",
                other
            ))),
        }
    }

    #[tokio::test]
    async fn test_sequence_append() -> Result<()> {
        let mut safe = new_safe_instance()?;
        let data1 = b"First in the sequence";
        let xorurl = safe.sequence_create(Some(data1)).await?;
        let data2 = b"Second in the sequence";
        let new_version = safe.sequence_append(&xorurl, data2).await?;
        let received_data = safe.sequence_get(&format!("{}?v=1", xorurl)).await?;
        assert_eq!(received_data, (new_version, data2.to_vec()));
        Ok(())
    }
}
