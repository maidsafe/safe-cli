// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

//! sn_node provides the interface to Safe routing.  The resulting executable is the node
//! for the Safe network.

use crate::{
    api::common::{parse_hex, send_qjsonrpc_request},
    Error, Result,
};
use rand::{distributions::Standard, thread_rng, Rng};
use serde::de::DeserializeOwned;
use serde_json::{json, map::Map, Value};
use sn_data_types::{Keypair, PublicKey, SecretKey};
use sn_node_rpc_data_types as rpc_types;
use std::{
    fs,
    path::{Path, PathBuf},
};

// endpoint config parameters
const RPC_IP_ADDR_STR: &str = "https://localhost";
const RPC_SECRET_KEY_FILENAME: &str = "rpc_secret_key";
const CONNECTION_IDLE_TIMEOUT_MS: u64 = 10_000;

// message constants
const PASSPHRASE_SIZE: usize = 256;
const CREDENTIALS_FIELDNAME: &str = "credentials";
const PAYLOAD_FIELDNAME: &str = "payload";

/// Constitutes one client to one node
pub struct NodeRpcClient {
    /// where the node is listening on
    dest_endpoint: String,

    /// Used to sign messages to the node
    keypair: Keypair,

    /// where the ca base path is
    cert_base_path: PathBuf,
}

impl NodeRpcClient {
    /// Constructs a new client to connect to a node
    pub fn new<P>(rpc_port: u16, cert_base: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let dest_endpoint = format!("{}:{}", RPC_IP_ADDR_STR, rpc_port);

        // load key
        let sk_path = cert_base.as_ref().join(RPC_SECRET_KEY_FILENAME);
        let keypair = match load_sk(sk_path)? {
            SecretKey::Ed25519(sk) => Ok(Keypair::from(sk)),
            _ => Err(Error::NodeRpcClientError(
                "Secret key must be ed25519 format".to_string(),
            )),
        }?;
        Ok(Self {
            dest_endpoint,
            keypair,
            cert_base_path: cert_base.as_ref().to_owned(),
        })
    }

    /// Get the node's rewards public key and oher info
    pub async fn get_rewards_info(&self) -> Result<rpc_types::GetRewardsInfoResult> {
        self.send_node_rpc::<rpc_types::GetRewardsInfoResult>(
            rpc_types::METHOD_GET_REWARDS_INFO,
            json!(null),
        )
        .await
    }

    /// Sets the node reward key from a hex string before returning the reawrds info
    pub async fn set_reward_key(
        &self,
        reward_key: String,
    ) -> Result<rpc_types::SetRewardKeyResult> {
        let reward_key = PublicKey::Ed25519(
            ed25519_dalek::PublicKey::from_bytes(parse_hex(&reward_key).as_slice())
                .map_err(|e| Error::InvalidInput(e.to_string()))?,
        );
        let params = rpc_types::SetRewardKeyParams { reward_key };
        self.send_node_rpc::<rpc_types::SetRewardKeyResult>(
            rpc_types::METHOD_SET_REWARD_KEY,
            json!(params),
        )
        .await
    }

    /// Get stats and info on the storage offered by the node
    pub async fn get_storage_info(&self) -> Result<rpc_types::GetStorageInfoResult> {
        self.send_node_rpc::<rpc_types::GetStorageInfoResult>(
            rpc_types::METHOD_GET_STORAGE_INFO,
            json!(null),
        )
        .await
    }

    /// Get log lines from the node by id. Start idx >= 0 starts from head of the log,
    /// while start_idx < 0 implies a start index starting at the index denoted
    /// by the total number of log lines minus the start idx. E.g. start_idx = -1, num_lines=1,
    /// implies get the last log entry.
    pub async fn get_logs(
        &self,
        log_id: u64,
        start_idx: i64,
        num_lines: u64,
    ) -> Result<rpc_types::GetLogsResult> {
        let params = rpc_types::GetLogsParams {
            log_id,
            start_idx,
            num_lines,
        };
        self.send_node_rpc::<rpc_types::GetLogsResult>(rpc_types::METHOD_GET_LOGS, json!(params))
            .await
    }

    /// send an rpc to a node using JSON RPC over QUIC
    async fn send_node_rpc<T>(&self, method: &str, payload: serde_json::Value) -> Result<T>
    where
        T: DeserializeOwned,
    {
        // generate a passphrase and use it to sign the rpc
        let passphrase: Vec<u8> = thread_rng()
            .sample_iter(&Standard)
            .take(PASSPHRASE_SIZE)
            .collect();
        let signature = self.keypair.sign(passphrase.as_slice());
        let creds = rpc_types::Credentials {
            passphrase,
            signature,
        };

        // map the params and send
        let mut params_map = Map::new();
        params_map.insert(CREDENTIALS_FIELDNAME.to_string(), json!(creds));
        params_map.insert(PAYLOAD_FIELDNAME.to_string(), payload);

        // disble lint to map string to the proper error variant
        #[allow(clippy::redundant_closure)]
        send_qjsonrpc_request(
            self.cert_base_path.to_str().unwrap(),
            Some(CONNECTION_IDLE_TIMEOUT_MS),
            &self.dest_endpoint,
            method,
            Value::Object(params_map),
        )
        .await
        .map_err(|msg| Error::NodeRpcClientError(msg))
    }
}

/// Helper loads sk from disk. Some(sk) if file found, Some(None) if
/// rpc_base_dir/RPC_SK_FILENAME doesn't exists, Err(e) for hard
/// errors
fn load_sk<P: AsRef<Path>>(sk_path: P) -> Result<SecretKey> {
    // ensure exists
    if !sk_path.as_ref().is_file() {
        return Err(Error::NodeRpcClientError(
            "Unable to locate signing key for node RPC".to_string(),
        ));
    }

    // parse from file
    let sk_hex = String::from_utf8(
        fs::read(sk_path.as_ref()).map_err(|e| Error::NodeRpcClientError(e.to_string()))?,
    )
    .map_err(|e| Error::NodeRpcClientError(e.to_string()))?;

    Ok(SecretKey::Ed25519(
        ed25519_dalek::SecretKey::from_bytes(parse_hex(&sk_hex).as_slice()).map_err(|_| {
            Error::NodeRpcClientError("Unable to deserialize signing key from file.".to_string())
        })?,
    ))
}
