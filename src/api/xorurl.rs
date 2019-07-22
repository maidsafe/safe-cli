// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::{Error, ResultReturn};
use multibase::{decode, encode, Base};
use rand::rngs::OsRng;
use rand_core::RngCore;
use safe_nd::{XorName, XOR_NAME_LEN};
use url::Url;

const SAFE_URL_PROTOCOL: &str = "safe://";

// The XOR-URL type
pub type XorUrl = String;

#[derive(Debug, Clone, PartialEq)]
pub enum SafeContentType {
    Unknown = 0x00,
    CoinBalance = 0x01,
    Wallet = 0x02,
    SeqMutableData = 0x03,
    UnseqMutableData = 0x04,
    FilesContainer = 0x05,
    UnpublishedFilesContainer = 0x06,
    ImmutableData = 0x07,
    UnpublishedImmutableData = 0x08,
    ResolvableMapContainer = 0x09,
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
    content_type: SafeContentType,
    path: String,
}

impl XorUrlEncoder {
    pub fn new(xorname: XorName, type_tag: u64, content_type: SafeContentType) -> Self {
        Self {
            version: 1,
            xorname,
            type_tag,
            content_type,
            path: "".to_string(),
        }
    }

    // An static encoder function for convinience in some cases
    pub fn encode(
        xorname: XorName,
        type_tag: u64,
        content_type: SafeContentType,
        base: &str,
    ) -> ResultReturn<String> {
        let xorurl_encoder = XorUrlEncoder::new(xorname, type_tag, content_type);
        xorurl_encoder.to_string(base)
    }

    pub fn from_url(xorurl: &str) -> ResultReturn<Self> {
        let parsing_url = Url::parse(&xorurl).map_err(|err| {
            Error::InvalidXorUrl(format!("Problem parsing the XOR-URL: {:?}", err))
        })?;

        if parsing_url.scheme() != "safe" {
            return Err(Error::InvalidXorUrl(
                "Only \"safe://\" URLs may be used.".to_string(),
            ));
        }

        // Get path and normalise it to use '/' (instead of '\' as on Windows)
        let mut path = str::replace(parsing_url.path(), "\\", "/");
        if path == "/" {
            path = "".to_string();
        }

        let cid_str = parsing_url
            .host_str()
            .unwrap_or_else(|| "Failed parsing the XOR-URL");

        let decoded_xorurl = decode(&cid_str)
            .map_err(|err| Error::InvalidXorUrl(format!("Failed to decode XOR-URL: {:?}", err)))?
            .1;
        if decoded_xorurl.len() > 42 {
            return Err(Error::InvalidXorUrl(format!(
                "Invalid XOR-URL, encoded string too long: {} bytes",
                decoded_xorurl.len()
            )));
        }

        let u8_version: u8 = decoded_xorurl[0];
        let version: u64 = u64::from(u8_version);
        if version != 1 {
            return Err(Error::InvalidXorUrl(format!(
                "Invalid XOR-URL encoding version: {}",
                version
            )));
        }
        let content_type = match decoded_xorurl[1] {
            0 => SafeContentType::Unknown,
            1 => SafeContentType::CoinBalance,
            2 => SafeContentType::Wallet,
            3 => SafeContentType::SeqMutableData,
            4 => SafeContentType::UnseqMutableData,
            5 => SafeContentType::FilesContainer,
            6 => SafeContentType::UnpublishedFilesContainer,
            7 => SafeContentType::ImmutableData,
            8 => SafeContentType::UnpublishedImmutableData,
            9 => SafeContentType::ResolvableMapContainer,
            _other => SafeContentType::Unknown,
        };
        let xorname_offset = 2; // offset where to find the xorname bytes
        let type_tag_offset = xorname_offset + XOR_NAME_LEN; // offset where to find the type tag bytes

        let mut xorname = XorName::default();
        xorname
            .0
            .copy_from_slice(&decoded_xorurl[xorname_offset..type_tag_offset]);

        let type_tag_bytes_len = decoded_xorurl.len() - type_tag_offset;

        let mut tag_type_bytes = [0; 8];
        tag_type_bytes[8 - type_tag_bytes_len..]
            .copy_from_slice(&decoded_xorurl[type_tag_offset..]);
        let type_tag: u64 = u64::from_be_bytes(tag_type_bytes);

        Ok(Self {
            version,
            xorname,
            type_tag,
            content_type,
            path: path.to_string(),
        })
    }

    #[allow(dead_code)]
    pub fn version(&self) -> u64 {
        self.version
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

    pub fn to_string(&self, base: &str) -> ResultReturn<String> {
        // Temporary CID format (var length from 34 to 42 bytes):
        // 1 byte for version
        // 1 byte for content_type
        // 32 bytes for xorname
        // and up to 8 bytes for type_tag
        let mut cid_vec: Vec<u8> = vec![
            0x1,                             // version = 1
            self.content_type.clone() as u8, // content type
        ];

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
        0xa6323c4d4a32,
        SafeContentType::ImmutableData,
        &"base32".to_string()
    ));
    let base32_xorurl = "safe://bedtcmrtgq2tmnzyheydcmrtgq2tmnzyheydcmrtgq2tmnzyheydcmvggi6e2srs";
    assert_eq!(xorurl, base32_xorurl);
}

#[test]
fn test_xorurl_base32z_encoding() {
    use unwrap::unwrap;
    let xorname = XorName(*b"12345678901234567890123456789012");
    let xorurl = unwrap!(XorUrlEncoder::encode(
        xorname,
        0,
        SafeContentType::ImmutableData,
        &"base32z".to_string()
    ));
    let base32z_xorurl = "safe://hoqcj1gc4dkptz8yhuycj1gc4dkptz8yhuycj1gc4dkptz8yhuycj1";
    assert_eq!(xorurl, base32z_xorurl);
}

#[test]
fn test_xorurl_base64_encoding() {
    use unwrap::unwrap;
    let xorname = XorName(*b"12345678901234567890123456789012");
    let xorurl = unwrap!(XorUrlEncoder::encode(
        xorname,
        4584545,
        SafeContentType::FilesContainer,
        &"base64".to_string()
    ));
    let base64_xorurl = "safe://mBBTEyMzQ1Njc4OTAxMjM0NTY3ODkwMTIzNDU2Nzg5MDEyRfRh";
    assert_eq!(xorurl, base64_xorurl);
    let xorurl_encoder = unwrap!(XorUrlEncoder::from_url(&base64_xorurl));
    assert_eq!(base64_xorurl, unwrap!(xorurl_encoder.to_string("base64")));
    assert_eq!("", xorurl_encoder.path());
    assert_eq!(1, xorurl_encoder.version());
    assert_eq!(xorname, xorurl_encoder.xorname());
    assert_eq!(4584545, xorurl_encoder.type_tag());
    assert_eq!(
        SafeContentType::FilesContainer,
        xorurl_encoder.content_type()
    );
}

#[test]
fn test_xorurl_default_base_encoding() {
    use unwrap::unwrap;
    let xorname = XorName(*b"12345678901234567890123456789012");
    let base32z_xorurl = "safe://hoqcj1gc4dkptz8yhuycj1gc4dkptz8yhuycj1gc4dkptz8yhuycj1";
    let xorurl = unwrap!(XorUrlEncoder::encode(
        xorname,
        0,
        SafeContentType::ImmutableData,
        &"".to_string() // forces it to use the default
    ));
    assert_eq!(xorurl, base32z_xorurl);
}

#[test]
fn test_xorurl_decoding() {
    let xorname = XorName(*b"12345678901234567890123456789012");
    let type_tag: u64 = 0x0eef;
    let xorurl_encoder = XorUrlEncoder::new(xorname, type_tag, SafeContentType::ImmutableData);
    assert_eq!("", xorurl_encoder.path());
    assert_eq!(1, xorurl_encoder.version());
    assert_eq!(xorname, xorurl_encoder.xorname());
    assert_eq!(type_tag, xorurl_encoder.type_tag());
    assert_eq!(
        SafeContentType::ImmutableData,
        xorurl_encoder.content_type()
    );
}

#[test]
fn test_xorurl_decoding_with_path() {
    use unwrap::unwrap;
    let xorname = XorName(*b"12345678901234567890123456789012");
    let type_tag: u64 = 0x0eef;
    let xorurl = unwrap!(XorUrlEncoder::encode(
        xorname,
        type_tag,
        SafeContentType::ImmutableData,
        "base32z"
    ));

    let xorurl_with_path = format!("{}/subfolder/file", xorurl);
    let xorurl_encoder_with_path = unwrap!(XorUrlEncoder::from_url(&xorurl_with_path));
    assert_eq!(
        xorurl_with_path,
        unwrap!(xorurl_encoder_with_path.to_string("base32z"))
    );
    assert_eq!("/subfolder/file", xorurl_encoder_with_path.path());
    assert_eq!(1, xorurl_encoder_with_path.version());
    assert_eq!(xorname, xorurl_encoder_with_path.xorname());
    assert_eq!(type_tag, xorurl_encoder_with_path.type_tag());
    assert_eq!(
        SafeContentType::ImmutableData,
        xorurl_encoder_with_path.content_type()
    );
}
