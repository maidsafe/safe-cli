// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::helpers::get_from_arg_or_stdin;
use super::OutputFmt;
use prettytable::{format::FormatBuilder, Table};
use safe_cli::Safe;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum NrsSubCommands {
    #[structopt(name = "add")]
    /// Add a subname to a public name
    Add {
        /// The name to give this key
        name: String,
        /// The safe:// url to map this to. Usually a FilesContainer for a website, eg.
        #[structopt(short = "-l", long = "link")]
        link: Option<String>,
        /// Set the sub name as default for this public name.
        #[structopt(long = "default")]
        default: bool,
    },
    #[structopt(name = "create")]
    /// Create a new public name
    Create {
        /// The name to give site, eg 'safenetwork'
        name: String,
        /// The safe:// url to map this to. Usually a FilesContainer for a website, eg.
        #[structopt(short = "-l", long = "link")]
        link: Option<String>,
    },
    #[structopt(name = "remove")]
    /// Remove a subname from a public name
    Remove {
        /// The name to remove
        #[structopt(long = "name")]
        name: String,
    },
}

pub fn nrs_commander(
    cmd: Option<NrsSubCommands>,
    output_fmt: OutputFmt,
    dry_run: bool,
    safe: &mut Safe,
) -> Result<(), String> {
    match cmd {
        Some(NrsSubCommands::Create { name, link }) => {
            // TODO: Where do we store/reference these?
            // Add a container for this...
            // check for subdomain?
            // sanitize name / spacing etc.
            // validate desination?
            let set_as_default = true;
            let target = get_from_arg_or_stdin(link, Some("...awaiting target link from stdin"))?;

            let (nrs_map_container_xorurl, processed_entries, _nrs_map) =
                safe.nrs_map_container_create(&name, Some(&target), set_as_default, dry_run)?;

            // Now let's just print out the content of the NrsMap
            if OutputFmt::Pretty == output_fmt {
                println!(
                    "New NrsMap, \"{}\" created at: \"{}\"",
                    &name, nrs_map_container_xorurl
                );
                let mut table = Table::new();
                let format = FormatBuilder::new()
                    .column_separator(' ')
                    .padding(0, 1)
                    .build();
                table.set_format(format);

                for (public_name, (change, name_link)) in processed_entries.iter() {
                    table.add_row(row![change, public_name, name_link]);
                }
                table.printstd();
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&(nrs_map_container_xorurl, processed_entries))
                        .unwrap_or_else(|_| "Failed to serialise output to json".to_string())
                );
            }

            Ok(())
        }
        Some(NrsSubCommands::Add { .. }) => Ok(()),
        Some(NrsSubCommands::Remove { .. }) => Ok(()),

        None => Err("Missing keys sub-command. Use --help for details.".to_string()),
    }
}
