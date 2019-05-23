// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

// use crate::cli_helpers::*;

// use log::{debug, warn};
// use std::env;
use structopt::StructOpt;

use crate::commands::{keys, pns, safe_id, wallet};

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub enum SubCommands {
    #[structopt(name = "container")]
    /// Create a new SAFE Network account with the credentials provided
    Container {
        /// The invitation token for creating a new SAFE Network account
        #[structopt(short = "c", long = "create")]
        invite: String,
    },
    #[structopt(name = "auth")]
    /// Authorise an application by providing the authorisation request URI or string
    Auth {
        /// The authorisation request URI or string
        #[structopt(short = "r", long = "req")]
        req: String,
    },
    #[structopt(name = "cat")]
    /// Read data on the network.
    Cat {
        /// The invitation token for creating a new SAFE Network account
        #[structopt(short = "c", long = "cat")]
        invite: String,
    },
    #[structopt(name = "files")]
    /// Manage files on the network
    Files {
        /// The invitation token for creating a new SAFE Network account
        #[structopt(short = "c", long = "cat")]
        invite: String,
    },
    #[structopt(name = "pns")]
    /// Manage public names on the network
    Pns {
        /// subcommands
        #[structopt(subcommand)]
        cmd: Option<pns::PnsSubCommands>,
    },
    #[structopt(name = "keys")]
    /// Manage keys on the network
    Keys {
        /// subcommands
        #[structopt(subcommand)]
        cmd: Option<keys::KeysSubCommands>,
    },
    #[structopt(name = "wallet")]
    /// Manage wallets on the network
    Wallet {
        /// subcommands
        #[structopt(subcommand)]
        cmd: Option<wallet::WalletSubCommands>,
    },
    #[structopt(name = "safe-id")]
    /// Manage identities on the network
    SafeId {
        /// subcommands
        #[structopt(subcommand)]
        cmd: Option<safe_id::SafeIdSubCommands>,
    },
}
