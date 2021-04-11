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
use sn_api::NodeRpcClient;
use sn_launch_tool::{join_with, run_with};
use std::{
    collections::{HashMap, HashSet},
    fs::create_dir_all,
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

/// Helper to assign default value of ~/.safe/node/local-node/rpc
/// to cert_base if cert_base is None
fn get_node_cert_base(cert_base: Option<PathBuf>) -> Result<PathBuf> {
    match cert_base {
        Some(cert_base) => Ok(cert_base),
        None => {
            let mut path = get_node_bin_path(None)?;
            path.push("local-node");
            path.push("rpc");
            Ok(path)
        }
    }
}

/// Issues an rpc to get info related to storage (see GetRewardsInfoResult)
pub async fn node_get_rewards_info(rpc_port: u16, cert_base: Option<PathBuf>) -> Result<()> {
    let cert_base = get_node_cert_base(cert_base)?;
    let client = NodeRpcClient::new(rpc_port, &cert_base)?;
    let info = client.get_rewards_info().await?;
    println!("Reward Key: {}", &info.reward_key);
    Ok(())
}

/// Issue an rpc to set the reward key from a hex string
/// and then prints the rewards info (see node_get_rewards_info())
pub async fn node_set_reward_key(
    rpc_port: u16,
    cert_base: Option<PathBuf>,
    reward_key: String,
) -> Result<()> {
    let cert_base = get_node_cert_base(cert_base)?;
    let client = NodeRpcClient::new(rpc_port, &cert_base)?;
    let res = client.set_reward_key(reward_key.clone()).await?;
    println!("Old Reward Key: {}", &res.old_reward_key);
    println!("New Reward Key: {}", &reward_key);
    Ok(())
}

/// Issues an rpc to get info related to storage (see GetStorageInfoResult)
pub async fn node_get_storage_info(
    rpc_port: u16,
    cert_base: Option<PathBuf>,
    detailed: bool,
) -> Result<()> {
    let cert_base = get_node_cert_base(cert_base)?;
    let client = NodeRpcClient::new(rpc_port, &cert_base)?;
    let info = client.get_storage_info().await?;
    println!("Node Root: {:?}", &info.node_root);
    println!("Used space: {}", &info.used);
    println!("Total space: {}", &info.total);
    if detailed {
        println!("---------DETAILS---------");
        for (local_store_path, local_value) in info.local_stores.iter() {
            println!("'{:?}':{}", local_store_path, local_value);
        }
    }
    Ok(())
}

/// Issues an rpc to get a specified range of log lines (see GetLogsResult and GetLogsParams)
pub async fn node_get_logs(
    rpc_port: u16,
    cert_base: Option<PathBuf>,
    log_id: u64,
    start_idx: i64,
    num_lines: u64,
) -> Result<()> {
    use std::convert::TryFrom;

    // get lines
    let cert_base = get_node_cert_base(cert_base)?;
    let client = NodeRpcClient::new(rpc_port, &cert_base)?;
    let logs = client.get_logs(log_id, start_idx, num_lines).await?;

    // print result
    let num_fetched = u64::try_from(logs.lines.len()).unwrap_or(u64::MAX);
    if num_lines != num_fetched {
        println!(
            "[Info] Requested {} but received {} log lines",
            num_lines, num_fetched
        );
    }
    println!("---------LOGS {}---------", log_id);
    for line in logs.lines.iter() {
        println!("{}", line);
    }
    Ok(())
}

/// Issues several RPCs to get some general, bird's-eye information of the node
pub async fn node_get_status(rpc_port: u16, cert_base: Option<PathBuf>) -> Result<()> {
    let cert_base = get_node_cert_base(cert_base)?;
    let client = NodeRpcClient::new(rpc_port, &cert_base)?;
    let rewards_info = client.get_rewards_info().await?;
    let storage_info = client.get_storage_info().await?;
    println!("Reward Key: {}", &rewards_info.reward_key);
    println!("Used space: {}", &storage_info.used);
    println!("Total space: {}", &storage_info.total);
    Ok(())
}
