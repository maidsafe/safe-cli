// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

#[cfg(feature = "self-update")]
use super::helpers::download_from_s3_and_install_bin;
use anyhow::{anyhow, bail, Context, Result};
use log::debug;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sn_launch_tool::{join_with, run_with};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, create_dir_all, read_dir},
    io::{self, Write},
    net::SocketAddr,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

#[cfg(not(target_os = "windows"))]
const SN_NODE_EXECUTABLE: &str = "sn_node";

#[cfg(target_os = "windows")]
const SN_NODE_EXECUTABLE: &str = "sn_node.exe";

const DEFAULT_MAX_CAPACITY: u64 = 2 * 1024 * 1024 * 1024;

fn run_safe_cmd(
    args: &[&str],
    envs: Option<HashMap<String, String>>,
    ignore_errors: bool,
    verbosity: u8,
) -> Result<()> {
    let env: HashMap<String, String> = envs.unwrap_or_else(HashMap::default);

    let msg = format!("Running 'safe' with args {:?} ...", args);
    if verbosity > 1 {
        println!("{}", msg);
    }
    debug!("{}", msg);

    let _child = Command::new("safe")
        .args(args)
        .envs(&env)
        .stdout(Stdio::inherit())
        .stderr(if ignore_errors {
            Stdio::null()
        } else {
            Stdio::inherit()
        })
        .spawn()
        .with_context(|| format!("Failed to run 'safe' with args '{:?}'", args))?;

    Ok(())
}

/// Tries to print the version of the node binary pointed to
pub fn node_version(node_path: Option<PathBuf>) -> Result<()> {
    let bin_path = get_node_bin_path(node_path)?.join(SN_NODE_EXECUTABLE);
    let path_str = bin_path.display().to_string();

    if !bin_path.as_path().is_file() {
        return Err(anyhow!(format!(
            "node executable not found at '{}'.",
            path_str
        )));
    }

    let output = Command::new(&path_str)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| {
            anyhow!(format!(
                "Failed to execute node from '{}': {}",
                path_str, err
            ))
        })?;

    if output.status.success() {
        io::stdout()
            .write_all(&output.stdout)
            .map_err(|err| anyhow!(format!("failed to write to stdout: {}", err)))
    } else {
        Err(anyhow!(
            "Failed to get node version nodes when invoking executable from '{}': {}",
            path_str,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[cfg(not(feature = "self-update"))]
pub fn node_install(_vault_path: Option<PathBuf>) -> Result<()> {
    anyhow!("Self updates are disabled")
}

#[cfg(feature = "self-update")]
pub fn node_install(node_path: Option<PathBuf>) -> Result<()> {
    let target_path = get_node_bin_path(node_path)?;
    let _ = download_from_s3_and_install_bin(
        target_path,
        "sn-node",
        "sn_node",
        SN_NODE_EXECUTABLE,
        if cfg!(target_os = "linux") {
            Some("x86_64-unknown-linux-musl")
        } else {
            None
        },
    )?;
    Ok(())
}

pub fn node_run(
    node_path: Option<PathBuf>,
    nodes_dir: &str,
    verbosity: u8,
    interval: &str,
    num_of_nodes: &str,
    ip: Option<String>,
    test: bool,
) -> Result<()> {
    let node_path = get_node_bin_path(node_path)?;

    let arg_node_path = node_path.join(SN_NODE_EXECUTABLE).display().to_string();
    debug!("Running node from {}", arg_node_path);

    let nodes_dir = node_path.join(nodes_dir);
    if !nodes_dir.exists() {
        println!("Creating '{}' folder", nodes_dir.display());
        create_dir_all(nodes_dir.clone())
            .context("Couldn't create target path to store nodes' generated data")?;
    }
    let arg_nodes_dir = nodes_dir.display().to_string();
    println!("Storing nodes' generated data at {}", arg_nodes_dir);

    // Let's create an args array to pass to the network launcher tool
    let mut sn_launch_tool_args = vec![
        "sn_launch_tool",
        "-v",
        "--node-path",
        &arg_node_path,
        "--nodes-dir",
        &arg_nodes_dir,
        "--interval",
        interval,
        "--num-nodes",
        num_of_nodes,
    ];

    let interval_as_int = &interval.parse::<u64>().unwrap();

    let mut verbosity_arg = String::from("-");
    if verbosity > 0 {
        let v = "y".repeat(verbosity as usize);
        println!("V: {}", v);
        verbosity_arg.push_str(&v);
        sn_launch_tool_args.push(&verbosity_arg);
    }

    if let Some(ref launch_ip) = ip {
        sn_launch_tool_args.push("--ip");
        sn_launch_tool_args.push(launch_ip);
    } else {
        sn_launch_tool_args.push("--local");
    }

    debug!(
        "Running network launch tool with args: {:?}",
        sn_launch_tool_args
    );

    // We can now call the tool with the args
    println!("Launching local Safe network...");
    run_with(Some(&sn_launch_tool_args)).map_err(|err| anyhow!(err))?;

    let interval_duration = Duration::from_secs(interval_as_int * 15);
    thread::sleep(interval_duration);

    let ignore_errors = true;
    let report_errors = false;

    if test {
        println!("Setting up authenticator against local Safe network...");

        // stop authd
        let stop_auth_args = vec!["auth", "stop"];
        run_safe_cmd(&stop_auth_args, None, ignore_errors, verbosity)?;

        let between_command_interval = Duration::from_secs(interval_as_int * 5);
        thread::sleep(between_command_interval);

        // start authd
        let start_auth_args = vec!["auth", "start"];
        run_safe_cmd(&start_auth_args, None, report_errors, verbosity)?;

        thread::sleep(between_command_interval);

        // Q: can we assume network is correct here? Or do we need to do networks switch?
        let passphrase: String = thread_rng().sample_iter(&Alphanumeric).take(15).collect();
        let password: String = thread_rng().sample_iter(&Alphanumeric).take(15).collect();

        // setup env for create / unlock
        let mut env = HashMap::new();
        env.insert("SAFE_AUTH_PASSPHRASE".to_string(), passphrase);
        env.insert("SAFE_AUTH_PASSWORD".to_string(), password);

        // create a Safe
        let create = vec!["auth", "create", "--test-coins"];

        run_safe_cmd(&create, Some(env.clone()), report_errors, verbosity)?;
        thread::sleep(between_command_interval);

        // unlock the Safe
        let unlock = vec!["auth", "unlock", "--self-auth"];
        run_safe_cmd(&unlock, Some(env), report_errors, verbosity)?;
    }

    Ok(())
}

pub fn node_join(
    node_path: Option<PathBuf>,
    node_data_dir: &str,
    verbosity: u8,
    contacts: &HashSet<SocketAddr>,
    max_capacity: Option<u64>,
) -> Result<()> {
    let node_path = get_node_bin_path(node_path)?;

    let arg_node_path = node_path.join(SN_NODE_EXECUTABLE).display().to_string();
    debug!("Running node from {}", arg_node_path);

    let node_data_dir = node_path.join(node_data_dir);
    if !node_data_dir.exists() {
        println!("Creating '{}' folder", node_data_dir.display());
        create_dir_all(node_data_dir.clone())
            .context("Couldn't create target path to store nodes' generated data")?;
    }
    let arg_nodes_dir = node_data_dir.display().to_string();
    println!("Storing nodes' generated data at {}", arg_nodes_dir);

    // Let's create an args array to pass to the network launcher tool
    let mut sn_launch_tool_args = vec![
        "sn_launch_tool-join",
        "-v",
        "--node-path",
        &arg_node_path,
        "--nodes-dir",
        &arg_nodes_dir,
    ];

    let max_capacity_string;
    if let Some(mc) = max_capacity {
        sn_launch_tool_args.push("--max-capacity");
        max_capacity_string = format!("{}", mc);
        sn_launch_tool_args.push(&max_capacity_string);
    }

    let mut verbosity_arg = String::from("-");
    if verbosity > 0 {
        let v = "y".repeat(verbosity as usize);
        println!("V: {}", v);
        verbosity_arg.push_str(&v);
        sn_launch_tool_args.push(&verbosity_arg);
    }

    sn_launch_tool_args.push("--hard-coded-contacts");
    let contacts_list = contacts
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>();

    for peer in &contacts_list {
        sn_launch_tool_args.push(peer);
    }

    debug!(
        "Running network launch tool with args: {:?}",
        sn_launch_tool_args
    );

    // We can now call the tool with the args
    println!("Starting a node to join a Safe network...");
    join_with(Some(&sn_launch_tool_args)).map_err(|err| anyhow!(err))?;
    Ok(())
}

pub fn node_shutdown(node_path: Option<PathBuf>) -> Result<()> {
    let node_exec_name = match node_path {
        Some(ref path) => {
            let filepath = path.as_path();
            if filepath.is_file() {
                match filepath.file_name() {
                    Some(filename) => match filename.to_str() {
                        Some(name) => name,
                        None => bail!("Node path provided ({}) contains invalid unicode chars", filepath.display()),
                    }
                    None => bail!("Node path provided ({}) is invalid as it doens't include the executable filename", filepath.display()),
                }
            } else {
                bail!("Node path provided ({}) is invalid as it doens't include the executable filename", filepath.display())
            }
        }
        None => SN_NODE_EXECUTABLE,
    };

    debug!(
        "Killing all running nodes launched with {}...",
        node_exec_name
    );
    kill_nodes(node_exec_name)
}

fn get_node_bin_path(node_path: Option<PathBuf>) -> Result<PathBuf> {
    match node_path {
        Some(p) => Ok(p),
        None => {
            let mut path = dirs_next::home_dir()
                .ok_or_else(|| anyhow!("Failed to obtain user's home path"))?;

            path.push(".safe");
            path.push("node");
            Ok(path)
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn kill_nodes(exec_name: &str) -> Result<()> {
    let output = Command::new("killall")
        .arg(exec_name)
        .output()
        .with_context(|| {
            format!(
                "Error when atempting to stop nodes ({}) processes",
                exec_name
            )
        })?;

    if output.status.success() {
        println!(
            "Success, all processes instances of {} were stopped!",
            exec_name
        );
        Ok(())
    } else {
        Err(anyhow!(
            "Failed to stop nodes ({}) processes: {}",
            exec_name,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn is_running(exec_name: &str) -> Result<bool> {
    let output = Command::new("pgrep")
        .arg(exec_name)
        .output()
        .with_context(|| format!("Error when running command `pgrep {}`", exec_name))?;
    Ok(output.status.success())
}

#[cfg(target_os = "windows")]
fn kill_nodes(exec_name: &str) -> Result<()> {
    let output = Command::new("taskkill")
        .args(&["/F", "/IM", exec_name])
        .output()
        .with_context(|| {
            format!(
                "Error when atempting to stop nodes ({}) processes",
                exec_name
            )
        })?;

    if output.status.success() {
        println!(
            "Success, all processes instances of {} were stopped!",
            exec_name
        );
        Ok(())
    } else {
        Err(anyhow!(
            "Failed to stop nodes ({}) processes: {}",
            exec_name,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub fn node_update(node_path: Option<PathBuf>) -> Result<()> {
    let node_path = get_node_bin_path(node_path)?;

    let arg_node_path = node_path.join(SN_NODE_EXECUTABLE).display().to_string();
    debug!("Updating node at {}", arg_node_path);

    let child = Command::new(&arg_node_path)
        .args(vec!["--update-only"])
        .spawn()
        .with_context(|| format!("Failed to update node at '{}'", arg_node_path))?;

    let output = child
        .wait_with_output()
        .with_context(|| format!("Failed to update node at '{}'", arg_node_path))?;

    if output.status.success() {
        io::stdout()
            .write_all(&output.stdout)
            .context("Failed to output stdout")?;
        Ok(())
    } else {
        Err(anyhow!(
            "Failed when invoking node executable from '{}':\n{}",
            arg_node_path,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn parse_storage(s: u64) -> String {
    if s > 1024 * 1024 * 1024 {
        format!(
            "{:.2} GB",
            (s as f64) / (1024.0 * 1024.0 * 1024.0)
        )
    } else if s > 1024 * 1024 {
        format!(
            "{:.2} MB",
            (s as f64) / (1024.0 * 1024.0)
        )
    } else if s > 1024 {
        format!(
            "{:.2} KB",
            (s as f64) / 1024.0
        )
    } else {
        format!("{} Bytes", s)
    }
}

pub fn node_status(node_path: Option<PathBuf>, local_node_dir: &str, max_capacity: Option<u64>) -> Result<()> {
    let running = is_running(SN_NODE_EXECUTABLE)?;

    let chunks_dir = get_node_bin_path(node_path)?
        .join(local_node_dir)
        .join("chunks");

    let mut total_used_space = 0;
    if chunks_dir.is_dir() {
        for entry in read_dir(chunks_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // `path` is a unique store
                let used_space_path = path.join("used_space");
                if used_space_path.is_file() {
                    let contents = fs::read(used_space_path)?;
                    let used_space = bincode::deserialize::<u64>(&contents)?;
                    total_used_space += used_space;
                } else {
                    return Err(anyhow!(
                        "used_space file not found for store {:?}",
                        path.file_name()
                    ));
                }
            }
        }
    } else {
        return Err(anyhow!("Chunks directory not found"));
    }

    println!("Status: {}", if running { "Running" } else { "Stopped" });
    let used_string = parse_storage(total_used_space);
    println!("Total storage used: {}", used_string);

    let max_capacity_string = match max_capacity {
        Some(mc) => parse_storage(mc),
        None => parse_storage(DEFAULT_MAX_CAPACITY),
    };

    println!("Storage Limit: {}", max_capacity_string);

    Ok(())
}
