// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use sn_entropy::Chunk;
use std::path::PathBuf;
use structopt::StructOpt;
use walkdir::WalkDir;

fn main() {
    let args = CmdArgs::from_args();
    let CmdArgs { ref input, .. } = args;
    WalkDir::new(input)
        .into_iter()
        .filter(|chunk| {
            if let Ok(c) = chunk {
                !c.path().is_dir()
            } else {
                false
            }
        })
        .for_each(|chunk| match chunk {
            Ok(ref chunk) => {
                let value =
                    Chunk::try_new(chunk.path()).and_then(|mut chunk| chunk.calculate_entropy());
                match value {
                    Ok(v) => println!("PATH: {}, entropy: {}", chunk.path().to_string_lossy(), v),
                    Err(e) => println!(
                        "Error calculating entropy for {}: {}",
                        chunk.path().to_string_lossy(),
                        e
                    ),
                }
            }
            Err(err) => println!(
                "Error calculating entropy for file {}: {}",
                input.to_string_lossy(),
                err.to_string()
            ),
        });
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "sn_entropy",
    about = "Calculates entropy of a file or a directory of files."
)]
pub struct CmdArgs {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}
