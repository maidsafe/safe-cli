// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

mod auth;
mod constants;
mod errors;
mod fetch;
mod files;
mod helpers;
mod keys;
mod nrs;
#[cfg(not(feature = "scl-mock"))]
mod safe_client_libs;
#[cfg(feature = "scl-mock")]
mod scl_mock;
mod wallet;
mod xorurl;

pub use errors::{Error, ResultReturn};
pub use fetch::SafeData;
pub use files::{FilesMap, ProcessedFiles};
pub use keys::BlsKeyPair;
pub use nrs::{NrsMap, ProcessedEntries};
pub use safe_nd::{XorName, XOR_NAME_LEN};
pub use xorurl::{XorUrl, XorUrlEncoder};

#[cfg(not(feature = "scl-mock"))]
use safe_client_libs::SafeApp;

#[cfg(feature = "scl-mock")]
use scl_mock::SafeApp;

pub trait SafeAuthApi {
    fn auth_app(
        &mut self,
        app_id: &str,
        app_name: &str,
        app_vendor: &str,
        port: Option<u16>,
    ) -> ResultReturn<String>;

    fn connect(&mut self, app_id: &str, auth_credentials: Option<&str>) -> ResultReturn<()>;
}

pub trait SafeFetchApi {
    fn fetch(&self, xorurl: &str) -> ResultReturn<SafeData>;
}

pub trait SafeFilesApi {
    fn files_container_create(
        &mut self,
        location: &str,
        dest: Option<String>,
        recursive: bool,
        dry_run: bool,
    ) -> ResultReturn<(XorUrl, ProcessedFiles, FilesMap)>;

    fn files_container_get_latest(&self, xorurl: &str) -> ResultReturn<(u64, FilesMap, String)>;

    fn files_container_sync(
        &mut self,
        location: &str,
        xorurl: &str,
        recursive: bool,
        delete: bool,
        dry_run: bool,
    ) -> ResultReturn<(u64, ProcessedFiles, FilesMap)>;

    fn files_put_published_immutable(&mut self, data: &[u8]) -> ResultReturn<XorUrl>;

    fn files_get_published_immutable(&self, xorurl: &str) -> ResultReturn<Vec<u8>>;
}

pub trait SafeKeysApi {
    fn keypair(&self) -> ResultReturn<BlsKeyPair>;

    // Create a Key on the network and return its XOR-URL.
    fn keys_create(
        &mut self,
        from: Option<String>,
        preload_amount: Option<String>,
        pk: Option<String>,
    ) -> ResultReturn<(XorUrl, Option<BlsKeyPair>)>;

    // Create a Key on the network, allocates testcoins onto it, and return the Key's XOR-URL
    // This is avilable only when testing with mock-network
    // #[cfg(feature = "mock-network")]
    fn keys_create_preload_test_coins(
        &mut self,
        preload_amount: String,
        pk: Option<String>,
    ) -> ResultReturn<(XorUrl, Option<BlsKeyPair>)>;

    // Check Key's balance from the network from a given SecretKey string
    fn keys_balance_from_sk(&self, sk: &str) -> ResultReturn<String>;

    // Check Key's balance from the network from a given XOR-URL and secret key string.
    // The difference between this and 'keys_balance_from_sk' function is that this will additionally
    // check that the XOR-URL corresponds to the public key derived from the provided secret key
    fn keys_balance_from_xorurl(&self, xorurl: &str, sk: &str) -> ResultReturn<String>;

    // Check that the XOR-URL corresponds to the public key derived from the provided secret key
    fn validate_sk_for_xorurl(&self, sk: &str, xorurl: &str) -> ResultReturn<String>;
}

pub trait SafeNrsApi {
    fn nrs_map_container_create(
        &mut self,
        name: &str,
        destination: Option<&str>,
        default: bool,
        _dry_run: bool,
    ) -> ResultReturn<(XorUrl, ProcessedEntries, NrsMap)>;

    fn nrs_map_container_get_latest(&self, xorurl: &str) -> ResultReturn<(u64, NrsMap, String)>;
}

pub trait SafeWalletApi {
    fn wallet_create(&mut self) -> ResultReturn<XorUrl>;

    fn wallet_insert(
        &mut self,
        wallet_xorurl: &str,
        name: &str,
        default: bool,
        key_pair: &BlsKeyPair,
        key_xorurl: &str,
    ) -> ResultReturn<()>;

    fn wallet_balance(&mut self, xorurl: &str) -> ResultReturn<String>;

    fn wallet_transfer(
        &mut self,
        amount: &str,
        from: Option<XorUrl>,
        to: &str,
    ) -> ResultReturn<u64>;
}

pub struct Safe {
    safe_app: SafeApp,
    xorurl_base: String,
}

impl Safe {
    pub fn new(xorurl_base: String) -> Self {
        Self {
            safe_app: SafeApp::new(),
            xorurl_base,
        }
    }
}
