// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::helpers::{get_from_arg_or_stdin, serialise_output};
use super::OutputFmt;
use log::debug;
use pretty_hex;
use prettytable::Table;
use safe_api::{Safe, SafeData};
use std::io::{self, Write};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct CatCommands {
    /// The safe:// location to retrieve
    location: Option<String>,
    /// Renders file output as hex
    #[structopt(short = "x", long = "hexdump")]
    hexdump: bool,
}

pub fn cat_commander(
    cmd: CatCommands,
    output_fmt: OutputFmt,
    safe: &mut Safe,
) -> Result<(), String> {
    let url = get_from_arg_or_stdin(cmd.location, None)?;
    debug!("Running cat for: {:?}", &url);

    let content = safe.fetch(&url)?;
    match &content {
        SafeData::FilesContainer {
            version, files_map, ..
        } => {
            // Render FilesContainer
            if OutputFmt::Pretty == output_fmt {
                println!(
                    "Files of FilesContainer (version {}) at \"{}\":",
                    version, url
                );
                let mut table = Table::new();
                table.add_row(
                    row![bFg->"Name", bFg->"Size", bFg->"Created", bFg->"Modified", bFg->"Link"],
                );
                files_map.iter().for_each(|(name, file_item)| {
                    table.add_row(row![
                        name,
                        file_item["size"],
                        file_item["created"],
                        file_item["modified"],
                        file_item["link"],
                    ]);
                });
                table.printstd();
            } else {
                println!("{}", serialise_output(&(url, files_map), output_fmt));
            }
        }
        SafeData::PublishedImmutableData { data, .. } => {
            if cmd.hexdump {
                // Render hex representation of ImmutableData file
                println!("{}", pretty_hex::pretty_hex(data).to_string());
            } else {
                // Render ImmutableData file
                io::stdout().write_all(data).map_err(|err| {
                    format!("Failed to print out the content of the file: {}", err)
                })?
            }
        }
        SafeData::Wallet { balances, .. } => {
            // Render Wallet
            if OutputFmt::Pretty == output_fmt {
                println!("Spendable balances of Wallet at \"{}\":", url);
                let mut table = Table::new();
                table.add_row(row![bFg->"Default", bFg->"Friendly Name", bFg->"SafeKey URL"]);
                balances.iter().for_each(|(name, (default, balance))| {
                    let def = if *default { "*" } else { "" };
                    table.add_row(row![def, name, balance.xorurl]);
                });
                table.printstd();
            } else {
                println!("{}", serialise_output(&(url, balances), output_fmt));
            }
        }
        SafeData::SafeKey { .. } => {
            println!("No content to show since the URL targets a SafeKey. Use the 'dog' command to obtain additional information about the targeted SafeKey.");
        }
    }

    Ok(())
}
