// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use super::quic_client::quic_send;
use super::Error;
use rand::{self, Rng};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

type Result<T> = std::result::Result<T, Error>;

// Version of the JSON-RPC used in the requests
const SAFE_AUTHD_JSONRPC_VERSION: &str = "2.0";

// JSON-RPC error codes as defined at https://www.jsonrpc.org/specification#response_object
const JSONRPC_PARSE_ERROR: isize = -32700;
const JSONRPC_INVALID_REQUEST: isize = -32600;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRpcReq {
    jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u32,
}

#[derive(Deserialize, Serialize, Debug)]
struct JsonRpcRes<'a> {
    jsonrpc: &'a str,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError<'a>>,
    id: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcError<'a> {
    code: isize,
    message: &'a str,
    data: &'a str,
}

// It parses the request bytes an returns a JsonRpcReq, or a
// serialised JSON-RPC error response ready to send back to the origin
pub fn parse_jsonrpc_request(req: Vec<u8>) -> std::result::Result<JsonRpcReq, String> {
    let req_payload = match String::from_utf8(req) {
        Ok(payload) => payload,
        Err(err) => {
            let err_str = jsonrpc_serialised_error(
                "Request payload is a malformed UTF-8 string",
                &err.to_string(),
                JSONRPC_PARSE_ERROR,
                None,
            )?;
            return Err(err_str);
        }
    };

    let jsonrpc_req: JsonRpcReq = match serde_json::from_str(&req_payload) {
        Ok(jsonrpc) => jsonrpc,
        Err(err) => {
            let err_str = jsonrpc_serialised_error(
                "Failed to deserialise request payload as a JSON-RPC message",
                &err.to_string(),
                JSONRPC_INVALID_REQUEST,
                None,
            )?;
            return Err(err_str);
        }
    };

    Ok(jsonrpc_req)
}

// Generates a serialised JSON-RPC error response
pub fn jsonrpc_serialised_error(
    message: &str,
    data: &str,
    code: isize,
    id: Option<u32>,
) -> std::result::Result<String, String> {
    let jsonrpc_err = JsonRpcRes {
        jsonrpc: SAFE_AUTHD_JSONRPC_VERSION,
        result: None,
        error: Some(JsonRpcError {
            code,
            message,
            data,
        }),
        id,
    };
    let serialised_err_res = serde_json::to_string(&jsonrpc_err)
        .map_err(|err| format!("Failed to serialise authd error response: {:?}", err))?;

    Ok(serialised_err_res)
}

// Generates a serialised JSON-RPC response containing the result, ready to be send back to the origin
pub fn jsonrpc_serialised_result(
    result: serde_json::Value,
    id: u32,
) -> std::result::Result<String, String> {
    let jsonrpc_res = JsonRpcRes {
        jsonrpc: SAFE_AUTHD_JSONRPC_VERSION,
        result: Some(result),
        error: None,
        id: Some(id),
    };
    let serialised_res = serde_json::to_string(&jsonrpc_res)
        .map_err(|err| format!("Failed to serialise authd response: {:?}", err))?;

    Ok(serialised_res)
}

// Generates a JSON-RPC request and sends it to the provided endpoint URL using QUIC
pub fn jsonrpc_send<T>(
    endpoint: &str,
    method: &str,
    params: serde_json::Value,
    cert_base_path: Option<&str>,
    timeout: Option<u64>,
) -> Result<T>
where
    T: DeserializeOwned,
{
    let jsonrpc_req = JsonRpcReq {
        jsonrpc: SAFE_AUTHD_JSONRPC_VERSION.to_string(),
        method: method.to_string(),
        params,
        id: rand::thread_rng().gen_range(0, std::u32::MAX) + 1,
    };

    let serialised_req = serde_json::to_string(&jsonrpc_req)
        .map_err(|err| Error::ClientError(format!("Failed to serialise authd request: {}", err)))?;

    // Send request over QUIC, and await for JSON-RPC response
    let received_bytes = quic_send(
        &endpoint,
        &serialised_req,
        false,
        None,
        cert_base_path,
        false,
        timeout,
    )
    .map_err(Error::ClientError)?;

    let res_payload = std::str::from_utf8(received_bytes.as_slice())
        .map_err(|err| Error::ClientError(format!("Failed to decode response data: {}", err)))?;

    match serde_json::from_str(&res_payload) {
        Ok(JsonRpcRes {
            jsonrpc,
            result: Some(r),
            ..
        }) => {
            if jsonrpc != SAFE_AUTHD_JSONRPC_VERSION {
                Err(Error::ClientError(format!(
                    "JSON-RPC version {} not supported, only version {} is supported",
                    jsonrpc, SAFE_AUTHD_JSONRPC_VERSION
                )))
            } else {
                let result = serde_json::from_value(r).map_err(|err| {
                    Error::ClientError(format!("Failed to decode response result: {}", err))
                })?;

                Ok(result)
            }
        }
        Ok(JsonRpcRes {
            error: Some(err), ..
        }) => Err(Error::ServerError(err.message.to_string())),
        Ok(JsonRpcRes {
            result: None,
            error: None,
            ..
        }) => Err(Error::ClientError(
            "Received an invalid JSON-RPC response from authd".to_string(),
        )),
        Err(err) => Err(Error::ClientError(format!(
            "Failed to parse authd response: {}",
            err
        ))),
    }
}
