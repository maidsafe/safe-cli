// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use log::debug;
use prettytable::Table;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, create_dir_all, remove_file},
    path::PathBuf,
};
const CONFIG_FILENAME: &str = "config.json";
const CONFIG_NETWORKS_DIRNAME: &str = "networks";

#[derive(Deserialize, Debug, Serialize, Default)]
pub struct ConfigSettings {
    pub networks: BTreeMap<String, String>,
    // pub contacts: BTreeMap<String, String>,
}

pub fn read_config_settings() -> Result<(ConfigSettings, PathBuf), String> {
    let file_path = config_file_path()?;
    let file = match fs::File::open(&file_path) {
        Ok(file) => file,
        Err(error) => {
            return Err(format!(
                "Error reading config file from '{}': {}",
                file_path.display(),
                error
            ));
        }
    };
    let settings: ConfigSettings = serde_json::from_reader(file).map_err(|err| {
        format!(
            "Format of the config file is not valid and couldn't be parsed: {:?}",
            err
        )
    })?;
    debug!(
        "Config settings retrieved from {}: {:?}",
        file_path.display(),
        settings
    );
    Ok((settings, file_path))
}

pub fn write_config_settings(file_path: &PathBuf, settings: ConfigSettings) -> Result<(), String> {
    let serialised_settings = serde_json::to_string(&settings)
        .map_err(|err| format!("Failed to add config to file: {}", err))?;

    fs::write(&file_path, serialised_settings.as_bytes())
        .map_err(|err| format!("Unable to write config in {}: {}", file_path.display(), err))?;

    debug!(
        "Config settings at {} updated with: {:?}",
        file_path.display(),
        settings
    );

    Ok(())
}

pub fn add_network_to_config(
    network_name: &str,
    config_location: Option<String>,
) -> Result<(), String> {
    let location = match config_location {
        Some(location) => location,
        None => {
            // Cache current network connection info
            let (_, conn_info) = read_current_network_conn_info()?;
            let cache_path = cache_conn_info(network_name, &conn_info)?;
            println!(
                "Caching current network connection information into: {}",
                cache_path.display()
            );
            cache_path.display().to_string()
        }
    };

    let (mut settings, file_path) = read_config_settings()?;
    settings
        .networks
        .insert(network_name.to_string(), location.clone());
    write_config_settings(&file_path, settings)?;
    debug!("Network {} - {} added to settings", network_name, location);
    println!(
        "Network '{}' was added to the list. Connection information is located at '{}'",
        network_name, location
    );
    Ok(())
}

pub fn remove_network_from_config(network_name: &str) -> Result<(), String> {
    let (mut settings, file_path) = read_config_settings()?;
    match settings.networks.remove(network_name) {
        Some(location) => {
            write_config_settings(&file_path, settings)?;
            debug!("Network {} removed from settings", network_name);
            println!("Network '{}' was removed from the list", network_name);
            let mut config_local_path = get_cli_config_path()?;
            config_local_path.push(CONFIG_NETWORKS_DIRNAME);
            if PathBuf::from(&location).starts_with(config_local_path) {
                println!(
                    "Removing cached network connection information from {}",
                    location
                );
                remove_file(&location).map_err(|err| {
                    format!(
                        "Failed to remove cached network connection information from {}: {}",
                        location, err
                    )
                })?;
            }
        }
        None => println!(
            "No network with name '{}' was found in config",
            network_name
        ),
    }

    Ok(())
}

pub fn read_current_network_conn_info() -> Result<(PathBuf, Vec<u8>), String> {
    let (_, file_path) = get_current_network_conn_info_path()?;
    let current_conn_info = fs::read(&file_path).map_err(|err| {
        format!(
            "There doesn't seem to be a any network setup in your system. Unable to read current network connection information from '{}': {}",
            file_path.display(), err
        )
    })?;
    Ok((file_path, current_conn_info))
}

pub fn write_current_network_conn_info(conn_info: &[u8]) -> Result<(), String> {
    let (base_path, file_path) = get_current_network_conn_info_path()?;

    if !base_path.exists() {
        println!(
            "Creating '{}' folder for network connection info",
            base_path.display()
        );
        create_dir_all(&base_path).map_err(|err| {
            format!(
                "Couldn't create folder for network connection info: {}",
                err
            )
        })?;
    }

    fs::write(&file_path, conn_info).map_err(|err| {
        format!(
            "Unable to write network connection info in {}: {}",
            base_path.display(),
            err
        )
    })
}

pub fn config_file_path() -> Result<PathBuf, String> {
    let config_local_path = get_cli_config_path()?;
    let file_path = config_local_path.join(CONFIG_FILENAME);
    if !config_local_path.exists() {
        println!(
            "Creating '{}' folder for config file",
            config_local_path.display()
        );
        create_dir_all(config_local_path)
            .map_err(|err| format!("Couldn't create project's local config folder: {}", err))?;
    }

    if !file_path.exists() {
        let empty_settings = ConfigSettings::default();
        write_config_settings(&file_path, empty_settings).map_err(|err| {
            format!(
                "Unable to create config in {}: {}",
                file_path.display(),
                err
            )
        })?;
    }

    Ok(file_path)
}

pub fn cache_conn_info(network_name: &str, conn_info: &[u8]) -> Result<PathBuf, String> {
    let mut file_path = get_cli_config_path()?;
    file_path.push(CONFIG_NETWORKS_DIRNAME);
    if !file_path.exists() {
        println!(
            "Creating '{}' folder for networks connection info cache",
            file_path.display()
        );
        create_dir_all(&file_path).map_err(|err| {
            format!(
                "Couldn't create folder for networks information cache: {}",
                err
            )
        })?;
    }

    file_path.push(format!("{}_node_connection_info.config", network_name));
    fs::write(&file_path, conn_info).map_err(|err| {
        format!(
            "Unable to cache connection information in {}: {}",
            file_path.display(),
            err
        )
    })?;

    Ok(file_path)
}

pub fn print_networks_settings() -> Result<(), String> {
    let mut table = Table::new();
    table.add_row(row![bFg->"Networks"]);
    table.add_row(row![bFg->"Network name", bFg->"Connection info location"]);

    let (settings, _) = read_config_settings()?;
    settings
        .networks
        .iter()
        .for_each(|(network_name, config_location)| {
            table.add_row(row![network_name, config_location,]);
        });
    table.printstd();
    Ok(())
}

pub fn retrieve_conn_info(name: &str, location: &str) -> Result<Vec<u8>, String> {
    println!(
        "Fetching '{}' network connection information from '{}' ...",
        name, location
    );
    if is_remote_location(location) {
        #[cfg(feature = "self-update")]
        {
            // Fetch info from an HTTP/s location
            let mut resp = reqwest::get(location).map_err(|err| {
                format!(
                    "Failed to fetch connection information for network '{}' from '{}': {}",
                    name, location, err
                )
            })?;

            let conn_info = resp.text().map_err(|err| {
                format!(
                    "Failed to fetch connection information for network '{}' from '{}': {}",
                    name, location, err
                )
            })?;
            Ok(conn_info.as_bytes().to_vec())
        }
        #[cfg(not(feature = "self-update"))]
        Err("Self updates are disabled".to_string())
    } else {
        // Fetch it from a local file then
        let conn_info = fs::read(location).map_err(|err| {
            format!(
                "Unable to read connection information from '{}': {}",
                location, err
            )
        })?;
        Ok(conn_info)
    }
}

#[inline]
fn is_remote_location(location: &str) -> bool {
    location.starts_with("http")
}

fn get_current_network_conn_info_path() -> Result<(PathBuf, PathBuf), String> {
    let mut node_data_path =
        dirs_next::home_dir().ok_or_else(|| "Failed to obtain user's home path".to_string())?;

    node_data_path.push(".safe");
    node_data_path.push("node");

    Ok((
        node_data_path.clone(),
        node_data_path.join("node_connection_info.config"),
    ))
}

fn get_cli_config_path() -> Result<PathBuf, String> {
    let mut project_data_path =
        dirs_next::home_dir().ok_or_else(|| "Couldn't find user's home directory".to_string())?;
    project_data_path.push(".safe");
    project_data_path.push("cli");

    Ok(project_data_path)
}
