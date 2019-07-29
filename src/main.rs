// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

mod api;
mod cli;
mod subcommands;

use cli::run;
use env_logger;
use log::{debug, error};
use std::process;
#[macro_use]
extern crate prettytable;

#[macro_use]
extern crate human_panic;

#[macro_use]
extern crate self_update;

#[macro_use]
extern crate validator_derive;

fn main() {
    setup_panic!();
    env_logger::init();
    debug!("Starting SAFE CLI...");

    if let Err(e) = update() {
        error!("safe_cli error: {}", e);
        process::exit(1);
    }
    if let Err(e) = run() {
        error!("safe_cli error: {}", e);
        process::exit(1);
    }
}

fn update() -> Result<(), Box<::std::error::Error>> {
    let target = self_update::get_target()?;
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner("jacderida")
        .repo_name("safe-cli")
        .with_target(&target)
        .build()?
        .fetch()?;
    println!("found releases:");
    println!("{:#?}\n", releases);
    Ok(())
}
