// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::helpers::get_host_and_path;
use super::{Error, ResultReturn};
use multibase::{decode, encode, Base};
use rand::rngs::OsRng;
use rand_core::RngCore;
use safe_nd::{XorName, XOR_NAME_LEN};

const SAFE_URL_PROTOCOL: &str = "safe://";
const XOR_URL_VERSION_1: u64 = 0x1; // TODO: consider using 16 bits
const XOR_URL_STR_MAX_LENGTH: usize = 44;
const XOR_NAME_BYTES_OFFSET: usize = 4; // offset where to find the XoR name bytes

// The XOR-URL type
pub type XorUrl = String;

// We encode the content type that a XOR-URL is targetting, this allows the consumer/user to
// treat the content in particular ways when the content requires it.
// TODO: support MIME types
#[derive(Debug, Clone, PartialEq)]
pub enum SafeContentType {
    Raw = 0x0000,
    Wallet = 0x0001,
    FilesContainer = 0x0002,
    NrsMapContainer = 0x0003,
}

// We also encode the native SAFE data type where the content is being stored on the SAFE Network,
// this allows us to fetch the targetted data using the corresponding API, regardless of the
// data that is being held which is identified by the SafeContentType instead.
#[derive(Debug, Clone, PartialEq)]
pub enum SafeDataType {
    CoinBalance = 0x00,
    PublishedImmutableData = 0x01,
    UnpublishedImmutableData = 0x02,
    SeqMutableData = 0x03,
    UnseqMutableData = 0x04,
    PublishedSeqAppendOnlyData = 0x05,
    PublishedUnseqAppendOnlyData = 0x06,
    UnpublishedSeqAppendOnlyData = 0x07,
    UnpublishedUnseqAppendOnlyData = 0x08,
}

impl std::fmt::Display for SafeDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn create_random_xorname() -> XorName {
    let mut os_rng = OsRng::new().unwrap();
    let mut xorname = XorName::default();
    os_rng.fill_bytes(&mut xorname.0);
    xorname
}

#[derive(Debug)]
pub struct XorUrlEncoder {
    version: u64, // currently only v1 supported
    xorname: XorName,
    type_tag: u64,
    data_type: SafeDataType,
    content_type: SafeContentType,
    path: String,
}

impl XorUrlEncoder {
    pub fn new(
        xorname: XorName,
        type_tag: u64,
        data_type: SafeDataType,
        content_type: SafeContentType,
        path: Option<&str>,
    ) -> Self {
        Self {
            version: XOR_URL_VERSION_1,
            xorname,
            type_tag,
            data_type,
            content_type,
            path: path.unwrap_or_else(|| "").to_string(),
        }
    }

    // A non-member encoder function for convinience in some cases
    pub fn encode(
        xorname: XorName,
        type_tag: u64,
        data_type: SafeDataType,
        content_type: SafeContentType,
        path: Option<&str>,
        base: &str,
    ) -> ResultReturn<String> {
        let xorurl_encoder = XorUrlEncoder::new(xorname, type_tag, data_type, content_type, path);
        xorurl_encoder.to_string(base)
    }

    pub fn from_url(xorurl: &str) -> ResultReturn<Self> {
        let (cid_str, path) = get_host_and_path(&xorurl)?;

        let (_base, xorurl_bytes): (Base, Vec<u8>) = decode(&cid_str)
            .map_err(|err| Error::InvalidXorUrl(format!("Failed to decode XOR-URL: {:?}", err)))?;

        // let's do a sanity check before analysing byte by byte
        if xorurl_bytes.len() > XOR_URL_STR_MAX_LENGTH {
            return Err(Error::InvalidXorUrl(format!(
                "Invalid XOR-URL, encoded string too long: {} bytes",
                xorurl_bytes.len()
            )));
        }

        // let's make sure we support the XOR_URL version
        let u8_version: u8 = xorurl_bytes[0];
        let version: u64 = u64::from(u8_version);
        if version != XOR_URL_VERSION_1 {
            return Err(Error::InvalidXorUrl(format!(
                "Invalid or unsupported XOR-URL encoding version: {}",
                version
            )));
        }

        let mut content_type_bytes = [0; 2];
        content_type_bytes[0..].copy_from_slice(&xorurl_bytes[1..3]);
        let content_type = match u16::from_be_bytes(content_type_bytes) {
            0 => SafeContentType::Raw,
            1 => SafeContentType::Wallet,
            2 => SafeContentType::FilesContainer,
            3 => SafeContentType::NrsMapContainer,
            other => {
                return Err(Error::InvalidXorUrl(format!(
                    "Invalid content type encoded in the XOR-URL string: {}",
                    other
                )))
            }
        };

        let data_type = match xorurl_bytes[3] {
            0 => SafeDataType::CoinBalance,
            1 => SafeDataType::PublishedImmutableData,
            2 => SafeDataType::UnpublishedImmutableData,
            3 => SafeDataType::SeqMutableData,
            4 => SafeDataType::UnseqMutableData,
            5 => SafeDataType::PublishedSeqAppendOnlyData,
            6 => SafeDataType::PublishedUnseqAppendOnlyData,
            7 => SafeDataType::UnpublishedSeqAppendOnlyData,
            8 => SafeDataType::UnpublishedUnseqAppendOnlyData,
            other => {
                return Err(Error::InvalidXorUrl(format!(
                    "Invalid SAFE data type encoded in the XOR-URL string: {}",
                    other
                )))
            }
        };

        let type_tag_offset = XOR_NAME_BYTES_OFFSET + XOR_NAME_LEN; // offset where to find the type tag bytes

        let mut xorname = XorName::default();
        xorname
            .0
            .copy_from_slice(&xorurl_bytes[XOR_NAME_BYTES_OFFSET..type_tag_offset]);

        let type_tag_bytes_len = xorurl_bytes.len() - type_tag_offset;

        let mut type_tag_bytes = [0; 8];
        type_tag_bytes[8 - type_tag_bytes_len..].copy_from_slice(&xorurl_bytes[type_tag_offset..]);
        let type_tag: u64 = u64::from_be_bytes(type_tag_bytes);

        Ok(Self {
            version,
            xorname,
            type_tag,
            data_type,
            content_type,
            path: path.to_string(),
        })
    }

    #[allow(dead_code)]
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn data_type(&self) -> SafeDataType {
        self.data_type.clone()
    }

    pub fn content_type(&self) -> SafeContentType {
        self.content_type.clone()
    }

    pub fn xorname(&self) -> XorName {
        self.xorname
    }

    pub fn type_tag(&self) -> u64 {
        self.type_tag
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    // XOR-URL encoding format (var length from 36 to 44 bytes):
    // 1 byte for version
    // 2 bytes for content type (enough to start including some MIME types also)
    // 1 byte for SAFE native data type
    // 32 bytes for XoR Name
    // and up to 8 bytes for type_tag
    pub fn to_string(&self, base: &str) -> ResultReturn<String> {
        // let's set the first byte with the XOR-URL format version
        let mut cid_vec: Vec<u8> = vec![XOR_URL_VERSION_1 as u8];

        // add the content type bytes
        let content_type: u16 = self.content_type.clone() as u16;
        cid_vec.extend_from_slice(&content_type.to_be_bytes());

        // push the SAFE data type byte
        cid_vec.push(self.data_type.clone() as u8);

        // add the xorname 32 bytes
        cid_vec.extend_from_slice(&self.xorname.0);

        // let's get non-zero bytes only from th type_tag
        let start_byte: usize = (self.type_tag.leading_zeros() / 8) as usize;
        // add the non-zero bytes of type_tag
        cid_vec.extend_from_slice(&self.type_tag.to_be_bytes()[start_byte..]);

        let base_encoding = match base {
            "base32z" => Base::Base32z,
            "base32" => Base::Base32,
            "base64" => Base::Base64,
            base => {
                if !base.is_empty() {
                    println!(
                        "Base encoding '{}' not supported for XOR-URL. Using default 'base32z'.",
                        base
                    );
                }
                Base::Base32z
            }
        };
        let cid_str = encode(base_encoding, cid_vec);
        Ok(format!("{}{}{}", SAFE_URL_PROTOCOL, cid_str, self.path))
    }
}

#[test]
fn test_xorurl_base32_encoding() {
    use unwrap::unwrap;
    let xorname = XorName(*b"12345678901234567890123456789012");
    let xorurl = unwrap!(XorUrlEncoder::encode(
        xorname,
        0xa632_3c4d_4a32,
        SafeDataType::PublishedImmutableData,
        SafeContentType::Raw,
        None,
        "base32"
    ));
    let base32_xorurl =
        "safe://biaaaatcmrtgq2tmnzyheydcmrtgq2tmnzyheydcmrtgq2tmnzyheydcmvggi6e2srs";
    assert_eq!(xorurl, base32_xorurl);
}

#[test]
fn test_xorurl_base32z_encoding() {
    use unwrap::unwrap;
    let xorname = XorName(*b"12345678901234567890123456789012");
    let xorurl = unwrap!(XorUrlEncoder::encode(
        xorname,
        0,
        SafeDataType::PublishedImmutableData,
        SafeContentType::Raw,
        None,
        "base32z"
    ));
    let base32z_xorurl = "safe://hbyyyyncj1gc4dkptz8yhuycj1gc4dkptz8yhuycj1gc4dkptz8yhuycj1";
    assert_eq!(xorurl, base32z_xorurl);
}

#[test]
fn test_xorurl_base64_encoding() {
    use unwrap::unwrap;
    let xorname = XorName(*b"12345678901234567890123456789012");
    let xorurl = unwrap!(XorUrlEncoder::encode(
        xorname,
        4_584_545,
        SafeDataType::PublishedSeqAppendOnlyData,
        SafeContentType::FilesContainer,
        None,
        "base64"
    ));
    let base64_xorurl = "safe://mQACBTEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDEyRfRh";
    assert_eq!(xorurl, base64_xorurl);
    let xorurl_encoder = unwrap!(XorUrlEncoder::from_url(&base64_xorurl));
    assert_eq!(base64_xorurl, unwrap!(xorurl_encoder.to_string("base64")));
    assert_eq!("", xorurl_encoder.path());
    assert_eq!(XOR_URL_VERSION_1, xorurl_encoder.version());
    assert_eq!(xorname, xorurl_encoder.xorname());
    assert_eq!(4_584_545, xorurl_encoder.type_tag());
    assert_eq!(
        SafeDataType::PublishedSeqAppendOnlyData,
        xorurl_encoder.data_type()
    );
    assert_eq!(
        SafeContentType::FilesContainer,
        xorurl_encoder.content_type()
    );
}

#[test]
fn test_xorurl_default_base_encoding() {
    use unwrap::unwrap;
    let xorname = XorName(*b"12345678901234567890123456789012");
    let base32z_xorurl = "safe://hbyyyyncj1gc4dkptz8yhuycj1gc4dkptz8yhuycj1gc4dkptz8yhuycj1";
    let xorurl = unwrap!(XorUrlEncoder::encode(
        xorname,
        0,
        SafeDataType::PublishedImmutableData,
        SafeContentType::Raw,
        None,
        "" // forces it to use the default
    ));
    assert_eq!(xorurl, base32z_xorurl);
}

#[test]
fn test_xorurl_decoding() {
    let xorname = XorName(*b"12345678901234567890123456789012");
    let type_tag: u64 = 0x0eef;
    let xorurl_encoder = XorUrlEncoder::new(
        xorname,
        type_tag,
        SafeDataType::PublishedImmutableData,
        SafeContentType::Raw,
        None,
    );
    assert_eq!("", xorurl_encoder.path());
    assert_eq!(XOR_URL_VERSION_1, xorurl_encoder.version());
    assert_eq!(xorname, xorurl_encoder.xorname());
    assert_eq!(type_tag, xorurl_encoder.type_tag());
    assert_eq!(
        SafeDataType::PublishedImmutableData,
        xorurl_encoder.data_type()
    );
    assert_eq!(SafeContentType::Raw, xorurl_encoder.content_type());
}

#[test]
fn test_xorurl_decoding_with_path() {
    use unwrap::unwrap;
    let xorname = XorName(*b"12345678901234567890123456789012");
    let type_tag: u64 = 0x0eef;
    let xorurl = unwrap!(XorUrlEncoder::encode(
        xorname,
        type_tag,
        SafeDataType::PublishedSeqAppendOnlyData,
        SafeContentType::Wallet,
        None,
        "base32z"
    ));

    let xorurl_with_path = format!("{}/subfolder/file", xorurl);
    let xorurl_encoder_with_path = unwrap!(XorUrlEncoder::from_url(&xorurl_with_path));
    assert_eq!(
        xorurl_with_path,
        unwrap!(xorurl_encoder_with_path.to_string("base32z"))
    );
    assert_eq!("/subfolder/file", xorurl_encoder_with_path.path());
    assert_eq!(XOR_URL_VERSION_1, xorurl_encoder_with_path.version());
    assert_eq!(xorname, xorurl_encoder_with_path.xorname());
    assert_eq!(type_tag, xorurl_encoder_with_path.type_tag());
    assert_eq!(
        SafeDataType::PublishedSeqAppendOnlyData,
        xorurl_encoder_with_path.data_type()
    );
    assert_eq!(
        SafeContentType::Wallet,
        xorurl_encoder_with_path.content_type()
    );
}
