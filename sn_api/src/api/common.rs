// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use super::{constants::SN_AUTHD_CONNECTION_IDLE_TIMEOUT, Error, Result};
use log::info;
use qjsonrpc::ClientEndpoint;
use serde::de::DeserializeOwned;
use tokio::runtime;

pub mod auth_types {
    use crate::api::ipc::req::IpcReq;
    use serde::{Deserialize, Serialize};

    pub type SafeAuthReq = IpcReq;
    pub type SafeAuthReqId = u32;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct AuthedApp {
        /// The App ID. It must be unique.
        pub id: String,
        /// The application friendly-name.
        pub name: String,
        /// The application provider/vendor (e.g. MaidSafe)
        pub vendor: String,
    }

    // Type of the list of authorised applications in a Safe
    pub type AuthedAppsList = Vec<AuthedApp>;
}

pub fn parse_hex(hex_str: &str) -> Vec<u8> {
    let mut hex_bytes = hex_str
        .as_bytes()
        .iter()
        .filter_map(|b| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        })
        .fuse();

    let mut bytes = Vec::new();
    while let (Some(h), Some(l)) = (hex_bytes.next(), hex_bytes.next()) {
        bytes.push(h << 4 | l)
    }
    bytes
}

pub fn ed_sk_from_hex(hex_str: &str) -> Result<ed25519_dalek::SecretKey> {
    let sk_bytes = parse_hex(&hex_str);
    ed25519_dalek::SecretKey::from_bytes(&sk_bytes).map_err(|_| {
        Error::InvalidInput("Failed to deserialize provided Ed25519 secret key".to_string())
    })
}

// Get hex string of a SecretKey
pub fn sk_to_hex(sk: sn_data_types::SecretKey) -> String {
    match sk {
        sn_data_types::SecretKey::Ed25519(sk) => {
            sk.to_bytes().iter().map(|b| format!("{:02x}", b)).collect()
        }
        //SecretKey::Bls(sk) => sk.inner().reveal(), // FIXME: it includes bls in the text
        sn_data_types::SecretKey::BlsShare(sk) => sk.inner().reveal(), // FIXME: it includes bls in the text
    }
}

/// Convert byte vector to hex
#[allow(dead_code)]
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// Send a request to authd using JSON-RPC over QUIC
pub async fn send_authd_request<T>(
    dest_endpoint: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<T>
where
    T: DeserializeOwned,
{
    info!(
        "Sending '{}' request to SAFE Authenticator on {} ...",
        method, dest_endpoint
    );

    match dirs_next::home_dir() {
        None => Err(Error::AuthdClientError(
            "Failed to obtain local project directory where to read certificate from".to_string(),
        )),
        Some(mut paths) => {
            paths.push(".safe");
            paths.push("authd");

            // disble lint to map string to the proper error variant
            #[allow(clippy::redundant_closure)]
            send_qjsonrpc_request(
                &paths,
                Some(SN_AUTHD_CONNECTION_IDLE_TIMEOUT),
                dest_endpoint,
                method,
                params,
            )
            .await
            .map_err(|msg| Error::AuthdClientError(msg))
        }
    }
}

// Send a request to authd using JSON-RPC over QUIC
pub async fn send_qjsonrpc_request<T, P>(
    cert_base_path: P,
    idle_timeout: Option<u64>,
    dest_endpoint: &str,
    method: &str,
    params: serde_json::Value,
) -> std::result::Result<T, String>
where
    T: DeserializeOwned,
    P: AsRef<std::path::Path>,
{
    // build a client
    let qjsonrpc_client = ClientEndpoint::new(
        cert_base_path.as_ref().to_str().unwrap(),
        idle_timeout,
        false,
    )
    .map_err(|err| format!("Failed to create client endpoint: {}", err))?;

    // We try to obtain current runtime or create a new one if there is none
    let runtime = match runtime::Handle::try_current() {
        Ok(r) => r,
        Err(_) => runtime::Runtime::new()
            .map_err(|err| format!("Failed to create runtime: {}", err))?
            .handle()
            .clone(),
    };

    // bind
    let mut outgoing_conn = {
        let _ = runtime.enter();
        qjsonrpc_client
            .bind()
            .map_err(|err| format!("Failed to bind endpoint: {}", err))?
    };

    outgoing_conn
        // connect
        .connect(dest_endpoint, None)
        .await
        .map_err(|err| format!("Failed to establish connection with authd: {}", err))?
        //send and wait for response
        .send(method, params)
        .await
        .map_err(|err| match err {
            qjsonrpc::Error::RemoteEndpointError(msg) => msg,
            other => other.to_string(),
        })
}
