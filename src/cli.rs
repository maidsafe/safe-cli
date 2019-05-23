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

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub enum KeySubCommands {
    #[structopt(name = "add")]
    /// Add a key to another document
    Add {
        /// The safe:// url to add
        #[structopt(long = "link")]
        link: String,
        /// The name to give this key
        #[structopt(long = "name")]
        name: String,
    },
    #[structopt(name = "create")]
    /// Create a new KeyPair
    Create {
        /// The name to give this key
        #[structopt(long = "anon")]
        anon: String,
        /// The name to give this key
        #[structopt(long = "name")]
        name: String,
        /// Preload the key with a coinbalance
        #[structopt(long = "preload")]
        preload: String,
    },
}

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub enum WalletSubCommands {
    #[structopt(name = "add")]
    /// Add a wallet to another document
    Add {
        /// The safe:// url to add
        #[structopt(long = "link")]
        link: String,
        /// The name to give this wallet
        #[structopt(long = "name")]
        name: String,
    },
    #[structopt(name = "balance")]
    /// Query a new Wallet or PublicKeys CoinBalance
    Balance {},
    #[structopt(name = "check-tx")]
    /// Check the status of a given transaction
    CheckTx {},
    #[structopt(name = "create")]
    /// Create a new Wallet/CoinBalance
    Create {},
    #[structopt(name = "sweep")]
    /// Move all coins within a wallet to a given balance
    Sweep {
        /// The source wallet for funds
        #[structopt(long = "from")]
        from: String,
        /// The receiving wallet/ballance
        #[structopt(long = "to")]
        to: String,
    },
    #[structopt(name = "transfer")]
    /// Manage files on the network
    Transfer {
        /// The safe:// url to add
        #[structopt(long = "amount")]
        amount: String,
        /// The source wallet / balance for funds
        #[structopt(long = "from")]
        from: String,
        /// The receiving wallet/ballance
        #[structopt(long = "to")]
        to: String,
    },
}
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
        /// The invitation token for creating a new SAFE Network account
        #[structopt(short = "c", long = "cat")]
        invite: String,
    },
    #[structopt(name = "keys")]
    /// Manage keys on the network
    Keys {
        /// subcommands
        #[structopt(subcommand)]
        cmd: Option<KeySubCommands>,
    },
    #[structopt(name = "wallet")]
    /// Manage wallets on the network
    Wallet {
        /// subcommands
        #[structopt(subcommand)]
        cmd: Option<WalletSubCommands>,
    },
    #[structopt(name = "safe-id")]
    /// Manage identities on the network
    SafeId {
        /// The invitation token for creating a new SAFE Network account
        #[structopt(short = "c", long = "cat")]
        invite: String,
    },
}

#[derive(StructOpt, Debug)]
/// Interact with the SAFE Network
pub struct CmdArgs {
    /// The safe:// address of target data
    #[structopt(short = "t", long = "target")]
    target: String,
    /// The account's Root Container address
    #[structopt(short = "r", long = "root")]
    root: bool,
    /// subcommands
    #[structopt(subcommand)]
    cmd: Option<SubCommands>,
    /// Output data serlialisation
    #[structopt(short = "o", long = "output")]
    output: String,
    /// Print human readable responses. (Alias to --output human-readable.)
    #[structopt(short = "hr", long = "human-readable")]
    human: bool,
    /// Increase output verbosity. (More logs!)
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
    /// Enable to query the output via SPARQL eg.
    #[structopt(short = "q", long = "query")]
    query: String,
    /// Dry run of command. No data will be written. No coins spent.
    #[structopt(long = "dry-run")]
    dry: bool,
}

pub fn run() -> Result<(), String> {
    // Let's first get all the arguments passed in
    let _args = CmdArgs::from_args();

    Ok(())
}
