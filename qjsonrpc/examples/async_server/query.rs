// copyright 2021 maidsafe.net limited.
//
// this safe network software is licensed to you under the general public license (gpl), version 3.
// unless required by applicable law or agreed to in writing, the safe network software distributed
// under the gpl licence is distributed on an "as is" basis, without warranties or conditions of any
// kind, either express or implied. please review the licences for the specific language governing
// permissions and limitations relating to use of the safe network software.

///! A module for outward-facing Query types and constants
///! that the server and client can use directly.
///! Also includes enumeration of supported methods
use serde::{Deserialize, Serialize};

/// method string used to request a ping
pub const METHOD_PING: &str = "ping";

/// method string used to ask for an echo of the arguments
pub const METHOD_ECHO: &str = "echo";

/// method string used to request a remote shutdown
pub const METHOD_SHUTDOWN: &str = "shutdown";

/// Outward-facing query types
/// for use by client and server while communicating
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Query {
    /// Ping the node
    Ping,

    /// Ask the node to echo a number back to you
    Echo(u32),

    /// Shutdown the node
    Shutdown,
}
