// copyright 2021 maidsafe.net limited.
//
// this safe network software is licensed to you under the general public license (gpl), version 3.
// unless required by applicable law or agreed to in writing, the safe network software distributed
// under the gpl licence is distributed on an "as is" basis, without warranties or conditions of any
// kind, either express or implied. please review the licences for the specific language governing
// permissions and limitations relating to use of the safe network software.

/**
 * A module for outward-facing response types and constants
 * that the server and client can use directly.
 * Also includes enumeration of error codes.
 */
use serde::{Deserialize, Serialize};

// JSON-RPC error codes as defined at https://www.jsonrpc.org/specification#response_object

pub const JSONRPC_METHOD_NOT_FOUND: isize = -32601;
pub const JSONRPC_ASYNC_SERVER_ERROR: isize = -320099;

/// Outward-facing succesful responses types
/// for use by client and server while communicating
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Response {
    /// Acknowledge a shutdown
    AckShutdown,

    /// Acknowledge a ping
    AckPing,
}
