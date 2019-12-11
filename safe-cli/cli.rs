// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use log::debug;
use structopt::StructOpt;

use crate::operations::safe_net::connect;
use crate::shell;
use crate::subcommands::auth::auth_commander;
use crate::subcommands::cat::cat_commander;
use crate::subcommands::config::config_commander;
use crate::subcommands::dog::dog_commander;
use crate::subcommands::files::files_commander;
use crate::subcommands::keys::key_commander;
use crate::subcommands::networks::networks_commander;
use crate::subcommands::nrs::nrs_commander;
use crate::subcommands::setup::setup_commander;
use crate::subcommands::update::update_commander;
use crate::subcommands::wallet::wallet_commander;
use crate::subcommands::{OutputFmt, SubCommands};
use safe_api::{Safe, XorUrlBase};

#[derive(StructOpt, Debug)]
/// Interact with the SAFE Network
#[structopt(raw(global_settings = "&[structopt::clap::AppSettings::ColoredHelp]"))]
pub struct CmdArgs {
    /// subcommands
    #[structopt(subcommand)]
    pub cmd: Option<SubCommands>,
    // /// The account's Root Container address
    // #[structopt(long = "root", raw(global = "true"))]
    // root: bool,
    /// Output data serialisation: [json, jsoncompact, yaml]
    #[structopt(short = "o", long = "output", raw(global = "true"))]
    output_fmt: Option<OutputFmt>,
    /// Sets JSON as output serialisation format (alias of '--output json')
    #[structopt(long = "json", raw(global = "true"))]
    output_json: bool,
    // /// Increase output verbosity. (More logs!)
    // #[structopt(short = "v", long = "verbose", raw(global = "true"))]
    // verbose: bool,
    // /// Enable to query the output via SPARQL eg.
    // #[structopt(short = "q", long = "query", raw(global = "true"))]
    // query: Option<String>,
    /// Dry run of command. No data will be written. No coins spent
    #[structopt(short = "n", long = "dry-run", raw(global = "true"))]
    dry: bool,
    /// Base encoding to be used for XOR-URLs generated. Currently supported: base32z (default), base32 and base64
    #[structopt(long = "xorurl", raw(global = "true"))]
    xorurl_base: Option<XorUrlBase>,
    /// Endpoint of the Authenticator daemon where to send requests to. If not provided, https://localhost:33000 is assumed.
    #[structopt(long = "endpoint", raw(global = "true"))]
    pub endpoint: Option<String>,
}

pub fn run() -> Result<(), String> {
    let mut safe = Safe::default();
    run_with(&[], &mut safe)
}

pub fn run_with(cmd_args: &[&str], mut safe: &mut Safe) -> Result<(), String> {
    // Let's first get all the arguments passed in, either as function's args, or CLI args
    let args = if cmd_args.is_empty() {
        CmdArgs::from_args()
    } else {
        CmdArgs::from_iter_safe(cmd_args).map_err(|err| err.to_string())?
    };

    let prev_base = safe.xorurl_base;
    if let Some(base) = args.xorurl_base {
        safe.xorurl_base = base;
    }

    let output_fmt = if args.output_json {
        OutputFmt::Json
    } else {
        match args.output_fmt {
            Some(fmt) => fmt,
            None => OutputFmt::Pretty,
        }
    };

    debug!("Processing command: {:?}", args);

    let result = match args.cmd {
        Some(SubCommands::Config { cmd }) => config_commander(cmd),
        Some(SubCommands::Networks { cmd }) => networks_commander(cmd),
        Some(SubCommands::Auth { cmd }) => auth_commander(cmd, args.endpoint, &mut safe),
        Some(SubCommands::Cat(cmd)) => cat_commander(cmd, output_fmt, &mut safe),
        Some(SubCommands::Dog(cmd)) => dog_commander(cmd, output_fmt, &mut safe),
        Some(SubCommands::Keypair {}) => {
            let key_pair = safe.keypair()?;
            if OutputFmt::Pretty == output_fmt {
                println!("Key pair generated:");
            }
            println!("Public Key = {}", key_pair.pk);
            println!("Secret Key = {}", key_pair.sk);
            Ok(())
        }
        Some(SubCommands::Update {}) => {
            update_commander().map_err(|err| format!("Error performing update: {}", err))
        }
        Some(SubCommands::Keys(cmd)) => key_commander(cmd, output_fmt, &mut safe),
        Some(SubCommands::Setup(cmd)) => setup_commander(cmd, output_fmt),
        Some(other) => {
            // We treat these separatelly since we need to connect before
            // handling any of these commands
            connect(&mut safe)?;
            match other {
                SubCommands::Wallet(cmd) => wallet_commander(cmd, output_fmt, &mut safe),
                SubCommands::Files(cmd) => files_commander(cmd, output_fmt, args.dry, &mut safe),
                SubCommands::Nrs(cmd) => nrs_commander(cmd, output_fmt, args.dry, &mut safe),
                _ => Err("Unknown safe subcommand".to_string()),
            }
        }
        None => shell::shell_run(), // then enter in interactive shell
    };

    safe.xorurl_base = prev_base;
    result
}
