// copyright 2021 maidsafe.net limited.
//
// this safe network software is licensed to you under the general public license (gpl), version 3.
// unless required by applicable law or agreed to in writing, the safe network software distributed
// under the gpl licence is distributed on an "as is" basis, without warranties or conditions of any
// kind, either express or implied. please review the licences for the specific language governing
// permissions and limitations relating to use of the safe network software.

mod query;
mod response;
mod rpc_daemon;

pub use query::*;
pub use response::*;

use qjsonrpc::{ClientEndpoint, Error, Result};
use rpc_daemon::rpc_daemon;
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use tempfile::tempdir;

///  Sets up a minimal client, a fake server process, and an async qjsonrpc interface in the middle.
/// The client pings the server by first sending a 'ping' request to the rpc interface service,
/// which forwards the ping to the server itself.
/// The server tells the interface to respond with an ACK, which the rpc interface forwards to the client.
/// A similar flow is then used to send a shutdown signal. When the QueryStream is dropped,
/// the rpc service knows it's time to shut down, and does so automatically too by returning
/// from run().
///
/// The rpc interface service allows us to asynchronously receive
/// buffer requests and responses from clients and the server respectively.
/// Neither the server nor the client ever needs to use the qjsonrpc API
/// directly and can instead focus on working with `Query` and `Response`
/// as if there was no networking involved
#[tokio::main]
async fn main() -> Result<()> {
    // init shared resources
    let listen = "https://localhost:33001";
    let cert_base_dir = Arc::new(tempdir()?);
    let (mut rpc_daemon, mut query_stream) = rpc_daemon();

    // client task
    let cert_base_dir1 = cert_base_dir.clone();
    let client_task = async move {
        let client = ClientEndpoint::new(cert_base_dir1.path(), Some(10000u64), false)?;
        let mut out_conn = client.bind()?;
        println!("[client] bound");

        // try ping
        let mut out_jsonrpc_req = out_conn.connect(listen, None).await?;
        println!("[client] connected to {}", listen);
        let ack = out_jsonrpc_req
            .send::<Response>(METHOD_PING, json!(null))
            .await?;
        println!("[client] sent ping to and received response {:?}", ack);

        // try remote shutdown
        let mut out_jsonrpc_req = out_conn.connect(listen, None).await?;
        println!("[client] connected to {}", listen);
        let ack = out_jsonrpc_req
            .send::<Response>(METHOD_SHUTDOWN, json!(null))
            .await?;
        println!("[client] shutdown sent and received response {:?}", ack);

        let res: Result<()> = Ok(());
        res
    };

    // the manager task (note this will run until query_stream is dropped)
    let cert_base_dir2 = cert_base_dir.clone();
    let rpc_daemon_task = async move {
        rpc_daemon
            .run(listen, Some(PathBuf::from(cert_base_dir2.path())), None)
            .await
    };

    // dirt-simple server focuses only on servicing queries
    let fake_node_task = async move {
        let mut done = false;
        while let Some((query, resp_stream)) = query_stream.get_next().await {
            println!("[server]: query {:?} received", &query);
            let resp = match &query {
                Query::Ping => Ok(Response::AckPing),
                Query::Shutdown => {
                    done = true;
                    Ok(Response::AckShutdown)
                }
            };

            println!("[server] sending response {:?}", &resp);
            resp_stream.send_oneshot(resp);
            if done {
                break;
            }
        }

        Ok(())
    };

    // join all
    tokio::try_join!(rpc_daemon_task, client_task, fake_node_task)
        .and_then(|_| Ok(()))
        .map_err(|e| Error::GeneralError(e.to_string()))
}
