// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use crate::operations::{
    config::{read_current_network_conn_info, Config},
    node::*,
};
use anyhow::{anyhow, Result};
use log::debug;
use std::{
    collections::HashSet, convert::TryInto, iter::FromIterator, net::SocketAddr, path::PathBuf,
};
use structopt::StructOpt;

const NODES_DATA_FOLDER: &str = "baby-fleming-nodes";

const LOCAL_NODE_DIR: &str = "local-node";

#[derive(StructOpt, Debug)]
pub enum NodeSubCommands {
    /// Gets the version of `sn_node` binary
    BinVersion {
        #[structopt(long = "node_path", env = "SN_NODE_PATH")]
        node_path: Option<PathBuf>,
    },
    #[structopt(name = "install")]
    /// Install latest sn_node released version in the system
    Install {
        #[structopt(long = "node-path")]
        /// Path where to install sn_node executable (default ~/.safe/node/). The SN_NODE_PATH env var can also be used to set the path
        #[structopt(long = "node-path", env = "SN_NODE_PATH")]
        node_path: Option<PathBuf>,
    },
    #[structopt(name = "join")]
    /// Join an already running network
    Join {
        /// Network to have the node to join to
        network_name: Option<String>,
        #[structopt(long = "node-path")]
        /// Path where to run sn_node executable from (default ~/.safe/node/). The SN_NODE_PATH env var can also be used to set the path
        #[structopt(long = "node-path", env = "SN_NODE_PATH")]
        node_path: Option<PathBuf>,
        /// Vebosity level for nodes logs
        #[structopt(short = "y", parse(from_occurrences))]
        verbosity: u8,
        /// Hardcoded contacts (endpoints) to be used to bootstrap to an already running network (this overrides any value passed as 'network_name').
        #[structopt(short = "h", long = "hcc")]
        hard_coded_contacts: Vec<SocketAddr>,
    },
    #[structopt(name = "run-baby-fleming")]
    /// Run nodes to form a local single-section Safe network
    Run {
        /// Path where to run sn_node executable from (default ~/.safe/node/). The SN_NODE_PATH env var can also be used to set the path
        #[structopt(long = "node-path", env = "SN_NODE_PATH")]
        node_path: Option<PathBuf>,
        /// Vebosity level for nodes logs (default = INFO, -y = DEBUG, -yy = TRACE)
        #[structopt(short = "y", parse(from_occurrences))]
        verbosity: u8,
        /// Interval in seconds between launching each of the nodes
        #[structopt(short = "i", long, default_value = "1")]
        interval: u64,
        /// Number of nodes to be launched
        #[structopt(long = "nodes", default_value = "11")]
        num_of_nodes: u8,
        /// IP to be used to launch the local nodes.
        #[structopt(long = "ip")]
        ip: Option<String>,
        /// Start authd and log in with
        #[structopt(short = "t", long = "testing")]
        test: bool,
    },
    /// Shutdown all running nodes processes
    #[structopt(name = "killall")]
    Killall {
        /// Path of the sn_node executable used to launch the processes with (default ~/.safe/node/sn_node). The SN_NODE_PATH env var can be also used to set this path
        #[structopt(long = "node-path", env = "SN_NODE_PATH")]
        node_path: Option<PathBuf>,
    },
    #[structopt(name = "update")]
    /// Update to latest sn_node released version
    Update {
        #[structopt(long = "node-path")]
        /// Path of the sn_node executable to update (default ~/.safe/node/). The SN_NODE_PATH env var can be also used to set the path
        #[structopt(long = "node-path", env = "SN_NODE_PATH")]
        node_path: Option<PathBuf>,
    },
    /// handle info related to rewards (like reward key, etc.)
    Rewards {
        /// What port to issue node remote procedure calls on
        #[structopt(long = "rpc-port", default_value = "34000")]
        rpc_port: u16,
        /// Root dir of node rpc certification (default is ~./safe/node/local-node/rpc).
        #[structopt(long = "cert-base")]
        cert_base: Option<PathBuf>,
        /// If provided, sets a new reward key from a hex string before fetching rewards info
        #[structopt(long = "set-key")]
        set_key: Option<String>,
    },
    /// handle info related to storage (e.g. how much storage the node is offering vs using)
    Storage {
        /// What port to issue node remote procedure calls on
        #[structopt(long = "rpc-port", default_value = "34000")]
        rpc_port: u16,
        /// Root dir of node rpc certification (default is ~./safe/node/local-node/rpc).
        #[structopt(long = "cert-base")]
        cert_base: Option<PathBuf>,
        /// Set to true to also fetch a breakdown of storage usage across the various local stores
        #[structopt(long = "detailed")]
        detailed: bool,
    },
    /// handle info related to storage (e.g. how much storage the node is offering vs using)
    Logs {
        /// What port to issue node remote procedure calls on
        #[structopt(long = "rpc-port", default_value = "34000")]
        rpc_port: u16,
        /// Root dir of node rpc certification (default is ~./safe/node/local-node/rpc).
        #[structopt(long = "cert-base")]
        cert_base: Option<PathBuf>,
        /// Which log to fetch by id (options: 0 = plaintext logs)
        #[structopt(long = "log-id", default_value = "0")]
        log_id: u64,
        /// Start line index of log fetch (see from-head)
        #[structopt(long = "start-idx", default_value = "0")]
        start_idx: i64,
        /// How many log lines to fetch
        #[structopt(long = "num-lines", default_value = "10")]
        num_lines: u64,
        /// True to indicate start idx is relative to log head
        /// False to indicate start-idx is the total number of log lines - num_lines
        #[structopt(long = "from-head")]
        from_head: bool,
    },
    /// A convenience RPC that wraps other commands like "rewards" and "storage"
    /// to Get some general, bird's-eye-view info of the node
    Status {
        /// What port to issue node remote procedure calls on
        #[structopt(long = "rpc-port", default_value = "34000")]
        rpc_port: u16,
        /// Root dir of node rpc certification (default is ~./safe/node/local-node/rpc).
        #[structopt(long = "cert-base")]
        cert_base: Option<PathBuf>,
    },
}

pub async fn node_commander(cmd: Option<NodeSubCommands>) -> Result<()> {
    match cmd {
        Some(NodeSubCommands::BinVersion { node_path }) => node_version(node_path),
        Some(NodeSubCommands::Install { node_path }) => {
            // We run this command in a separate thread to overcome a conflict with
            // the self_update crate as it seems to be creating its own runtime.
            let handler = std::thread::spawn(|| node_install(node_path));
            handler
                .join()
                .map_err(|err| anyhow!("Failed to run self update: {:?}", err))?
        }
        Some(NodeSubCommands::Join {
            network_name,
            node_path,
            verbosity,
            hard_coded_contacts,
        }) => {
            let network_contacts = if hard_coded_contacts.is_empty() {
                if let Some(name) = network_name {
                    let config = Config::read()?;
                    let msg = format!("Joining the '{}' network...", name);
                    debug!("{}", msg);
                    println!("{}", msg);
                    config.get_network_info(&name).await?
                } else {
                    let (_, contacts) = read_current_network_conn_info()?;
                    contacts
                }
            } else {
                HashSet::from_iter(hard_coded_contacts)
            };

            let msg = format!("Joining network with contacts {:?} ...", network_contacts);
            debug!("{}", msg);
            println!("{}", msg);

            node_join(node_path, LOCAL_NODE_DIR, verbosity, &network_contacts)
        }
        Some(NodeSubCommands::Run {
            node_path,
            verbosity,
            interval,
            num_of_nodes,
            ip,
            test,
        }) => node_run(
            node_path,
            NODES_DATA_FOLDER,
            verbosity,
            &interval.to_string(),
            &num_of_nodes.to_string(),
            ip,
            test,
        ),
        Some(NodeSubCommands::Killall { node_path }) => node_shutdown(node_path),
        Some(NodeSubCommands::Update { node_path }) => node_update(node_path),
        Some(NodeSubCommands::Rewards {
            rpc_port,
            cert_base,
            set_key,
        }) => {
            if let Some(key_hex) = set_key {
                node_set_reward_key(rpc_port, cert_base, key_hex).await
            } else {
                node_get_rewards_info(rpc_port, cert_base).await
            }
        }
        Some(NodeSubCommands::Storage {
            rpc_port,
            cert_base,
            detailed,
        }) => node_get_storage_info(rpc_port, cert_base, detailed).await,
        Some(NodeSubCommands::Logs {
            rpc_port,
            cert_base,
            log_id,
            start_idx,
            num_lines,
            from_head,
        }) => {
            let start_idx = if from_head {
                start_idx
            } else {
                (-start_idx).saturating_sub(num_lines.try_into().unwrap_or(i64::MAX))
            };
            node_get_logs(rpc_port, cert_base, log_id, start_idx, num_lines).await
        }
        Some(NodeSubCommands::Status {
            rpc_port,
            cert_base,
        }) => node_get_status(rpc_port, cert_base).await,
        None => Err(anyhow!("Missing node subcommand")),
    }
}
