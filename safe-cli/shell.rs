// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::cli;
use crate::operations::auth_daemon::*;
use crate::subcommands::auth::{auth_commander, AuthSubCommands};
use crate::subcommands::SubCommands;
use safe_api::{AuthReq, Safe, SafeAuthdClient};
use shrust::{Shell, ShellIO};
use std::io::{stdout, Write};
use structopt::StructOpt;

pub fn shell_run() -> Result<(), String> {
    let safe = Safe::default();
    let safe_authd_client = SafeAuthdClient::new(None);
    let mut shell = Shell::new((safe, safe_authd_client));
    shell.set_default(|io, _, cmd| {
        writeln!(
            io,
            "Command '{}' is unknown or not supported yet in interactive mode",
            cmd
        )?;
        writeln!(io, "Type 'help' for a list of currently supported top level commands")?;
        writeln!(io, "Pass '--help' flag to any top level command for a complete list of supported subcommands and arguments")?;
        Ok(())
    });
    shell.new_command(
        "auth",
        "Authorise the SAFE CLI and interact with a remote Authenticator daemon",
        0,
        |io, (safe, safe_authd_client), args| {
            // Let's create an args array to mimic the one we'd receive commands were passed from outside shell
            let mut mimic_cli_args = vec!["safe", "auth"];
            mimic_cli_args.extend(args.iter());

            // We can now pass this args array to the CLI structopt parser
            match cli::CmdArgs::from_iter_safe(mimic_cli_args) {
                Ok(cmd_args) => {
                    match cmd_args.cmd {
                        Some(SubCommands::Auth { cmd }) => {
                            if let Some(AuthSubCommands::Subscribe { notifs_endpoint }) = cmd {
                                match authd_subscribe(safe_authd_client, notifs_endpoint, &prompt_to_allow_auth) {
                                    Ok(()) => {
                                        writeln!(io, "Keep this shell session open to receive the notifications")?;
                                        Ok(())
                                    },
                                    Err(err) => {
                                        writeln!(io, "{}", err)?;
                                        Ok(())
                                    }
                                }
                            } else {
                                match auth_commander(cmd, cmd_args.endpoint, safe) {
                                    Ok(()) => Ok(()),
                                    Err(err) => {
                                        writeln!(io, "{}", err)?;
                                        Ok(())
                                    }
                                }
                            }
                        },
                        _other => {
                            writeln!(io, "Unexpected error. Command not valid")?;
                            Ok(())
                        }
                    }
                },
                Err(err) => {
                    writeln!(io, "{}", err)?;
                    Ok(())
                }
            }
        },
    );
    shell.new_command(
        "cat",
        "Read data on the SAFE Network",
        0,
        |io, (safe, _safe_authd_client), args| call_cli("cat", args, safe, io),
    );
    shell.new_command(
        "dog",
        "Inspect data on the SAFE Network providing only metadata information about the content",
        0,
        |io, (safe, _safe_authd_client), args| call_cli("dog", args, safe, io),
    );
    shell.new_command(
        "files",
        "Manage files on the SAFE Network",
        0,
        |io, (safe, _safe_authd_client), args| call_cli("files", args, safe, io),
    );
    shell.new_command(
        "keypair",
        "Generate a key pair without creating and/or storing a SafeKey on the network",
        0,
        |io, (safe, _safe_authd_client), args| call_cli("keypair", args, safe, io),
    );
    shell.new_command(
        "nrs",
        "Manage public names on the SAFE Network",
        0,
        |io, (safe, _safe_authd_client), args| call_cli("nrs", args, safe, io),
    );
    shell.new_command(
        "keys",
        "Manage keys on the SAFE Network",
        0,
        |io, (safe, _safe_authd_client), args| call_cli("keys", args, safe, io),
    );
    shell.new_command(
        "wallet",
        "Manage wallets on the SAFE Network",
        0,
        |io, (safe, _safe_authd_client), args| call_cli("wallet", args, safe, io),
    );
    shell.new_command(
        "update",
        "Update the application to the latest available version",
        0,
        |io, (safe, _safe_authd_client), args| call_cli("update", args, safe, io),
    );

    println!();
    println!("Welcome to SAFE CLI interactive shell!");
    println!("Type 'help' for a list of supported commands");
    println!("Pass '--help' flag to any top level command for a complete list of supported subcommands and arguments");
    println!("Type 'quit' to exit this shell. Enjoy it!");
    println!();

    // Run the shell loop to process user commands
    shell.run_loop(&mut ShellIO::default());

    Ok(())
}

fn call_cli(
    subcommand: &str,
    args: &[&str],
    safe: &mut Safe,
    io: &mut shrust::ShellIO,
) -> Result<(), shrust::ExecError> {
    // Let's create an args array to mimic the one we'd receive when passed to CLI
    let mut mimic_cli_args = vec!["safe", subcommand];
    mimic_cli_args.extend(args.iter());

    // We can now pass this args array to the CLI
    match cli::run_with(&mimic_cli_args, safe) {
        Ok(()) => Ok(()),
        Err(err) => {
            writeln!(io, "{}", err)?;
            Ok(())
        }
    }
}

fn prompt_to_allow_auth(auth_req: AuthReq) -> Option<bool> {
    println!();
    println!("A new application authorisation request was received:");
    let req_id = auth_req.req_id;
    pretty_print_auth_reqs(vec![auth_req], None);

    println!("You can use \"auth allow\"/\"auth deny\" commands to allow/deny the request respectively, e.g.: auth allow {}", req_id);
    println!("Press Enter to continue");
    let _ = stdout().flush();
    None
}
