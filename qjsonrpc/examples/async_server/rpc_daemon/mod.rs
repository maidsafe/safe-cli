// copyright 2021 maidsafe.net limited.
//
// this safe network software is licensed to you under the general public license (gpl), version 3.
// unless required by applicable law or agreed to in writing, the safe network software distributed
// under the gpl licence is distributed on an "as is" basis, without warranties or conditions of any
// kind, either express or implied. please review the licences for the specific language governing
// permissions and limitations relating to use of the safe network software.

///! A JSON RPC over quic daemon module which allows
///! asynchronous servicing and response to qjsonrpc reqeusts.
///!
///! For incoming `JsonRpcRequest`s, the daemon converts
///! them to `Query` and buffers them to a `QueryStream`.
///!
///! The server then asynchronously fetches the `Query`
///! along with a `ResponseStream` using `QueryStream.get_next()`.
///! The server *must* then service the query, and use
///! `ResponseStream.send_oneshot()` to reply to the `Request`
///! using a `Response`
///!
///! At some later point in time, the daemon will
///! pick up the `Response` and forward it to the Client after converting
///! the `Response` into a `JsonRpcResponse`.
mod stream;

pub use stream::{QueryStream, ResponseStream};

use qjsonrpc::{
    Endpoint, Error, IncomingJsonRpcRequest, JsonRpcResponse, JsonRpcResponseStream, Result,
};
use std::{
    collections::HashMap,
    convert::TryFrom,
    path::{Path, PathBuf},
    sync::Arc,
};
use stream::{QueryContainer, ResponseContainer};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Mutex,
};
use url::Url;

/// A JSON RPC over quic daemon process.
/// Acts as an async intermediary between the
/// client and the server to abstract out
/// the `qjsonrpc` internals
pub struct RpcDaemon {
    /// stream to buffer queries to the server
    query_tx: UnboundedSender<QueryContainer>,

    /// stream to buffer responses from the server
    response_rx: UnboundedReceiver<ResponseContainer>,

    /// Maps request id to the response stream
    open_streams: Arc<Mutex<HashMap<u32, JsonRpcResponseStream>>>,
}

impl RpcDaemon {
    /// ctor
    /// Don't use this directly. Instead see `rpc_daemon()`
    fn new(
        query_tx: UnboundedSender<QueryContainer>,
        response_rx: UnboundedReceiver<ResponseContainer>,
    ) -> Self {
        let open_streams = Arc::new(Mutex::new(HashMap::new()));
        Self {
            query_tx,
            response_rx,
            open_streams,
        }
    }

    /// Runs the service on the current thread
    /// This will loop, servicing incoming requests from
    /// clients and responses from the server.
    /// The function returns when it detects that
    /// the QueryStream associated with `self` (created along with
    /// `self` using `rpc_daemon()`) and all
    /// outstanding `ResponseStream`s are dropped.
    pub async fn run<P: AsRef<Path>>(
        &mut self,
        listen_addr_raw: &str,
        cert_base_path: Option<P>,
        idle_timeout: Option<u64>,
    ) -> Result<()> {
        // use the default ~/.safe/node_rpc if no path specified
        let base_path = cert_base_path.map_or_else(
            || match dirs_next::home_dir() {
                Some(mut path) => {
                    path.push(".safe");
                    path.push("simple_server_example");
                    Ok(path)
                }
                None => Err(Error::GeneralError(
                    "Failed to obtain local project directory where to write certificate from"
                        .to_string(),
                )),
            },
            |path| {
                let mut pathbuf = PathBuf::new();
                pathbuf.push(path);
                Ok(pathbuf)
            },
        )?;

        // parse and bind the socket address
        let listen_socket_addr = Url::parse(listen_addr_raw)
            .map_err(|_| Error::GeneralError("Invalid endpoint address".to_string()))?
            .socket_addrs(|| None)
            .map_err(|_| Error::GeneralError("Invalid endpoint address".to_string()))?[0];

        let qjsonrpc_endpoint = Endpoint::new(base_path, idle_timeout)
            .map_err(|err| Error::GeneralError(format!("Failed to create endpoint: {}", err)))?;

        let mut incoming_conn = qjsonrpc_endpoint
            .bind(&listen_socket_addr)
            .map_err(|err| Error::GeneralError(format!("Failed to bind endpoint: {}", err)))?;
        println!("[rpc daemon] Listening on {}", listen_socket_addr);

        // Service Loop
        let mut done = false;
        while !done {
            tokio::select!(

                // service a new qjsonrpc connections
                Some(incoming_req) = incoming_conn.get_next() => {
                    let _ = tokio::spawn(
                        Self::handle_connection(
                            self.query_tx.clone(),
                            self.open_streams.clone(),
                            incoming_req
                        )
                    );
                },

                // service `Response` from server
                resp_container_opt = self.response_rx.recv() => {

                    match resp_container_opt {
                        // followup an exisxting connection
                        Some(resp_container) => {
                            let _ = tokio::spawn(
                                Self::handle_response(
                                    resp_container,
                                    self.open_streams.clone()
                                )
                            );

                        },
                        // All senders were dropped, time to exit
                        None => {
                            done = true;
                        }
                    }
                },
            );
        }

        Ok(())
    }

    /// Handle a `Response` from the server
    /// by serializing it to a `JsonRpcResponse`
    /// and sending it back to the original sender
    async fn handle_response(
        resp_container: ResponseContainer,
        open_streams: Arc<Mutex<HashMap<u32, JsonRpcResponseStream>>>,
    ) -> Result<()> {
        println!("[rpc daemon]: response found {:?}", resp_container);

        // retreive the stream
        let mut open_streams_lock = open_streams.lock().await;
        let stream_opt = open_streams_lock.remove(&resp_container.id());
        drop(open_streams_lock);

        // For now, it's logically impossible to reply to a stream twice
        // Since `qjsonrpc` doesn't support batching yet.
        // If this changes in the future, this will likely panic
        let mut resp_stream = stream_opt.unwrap();

        let resp = JsonRpcResponse::from(resp_container);
        resp_stream.respond(&resp).await?;
        println!("[rpc daemon] responded with {:?}", &resp);
        resp_stream.finish().await
    }

    /// Handle an incoming `JsonRpcRequest` from a new client
    /// by converting it to a `Query` and buffering it
    /// to the `QueryStream`
    async fn handle_connection(
        query_tx: UnboundedSender<QueryContainer>,
        open_streams: Arc<Mutex<HashMap<u32, JsonRpcResponseStream>>>,
        mut incoming_req: IncomingJsonRpcRequest,
    ) -> Result<()> {
        // Each stream initiated by the client constitutes a new request
        // in the current `qjsonrpc` implementation (batches not supported yet)
        println!("[rpc daemon] incoming connection");
        while let Some((jsonrpc_req, mut resp_stream)) = incoming_req.get_next().await {
            println!("[rpc daemon] req received {:?}", jsonrpc_req);

            // Try to make a query container from the request
            match QueryContainer::try_from(jsonrpc_req) {
                // case: cache the response stream and buffer the query
                Ok(container) => {
                    let mut open_streams_lock = open_streams.lock().await;
                    let _ = open_streams_lock.insert(container.id(), resp_stream);
                    drop(open_streams_lock);

                    println!("[rpc daemon] forwarding command to server {:?}", container);
                    query_tx
                        .send(container)
                        .map_err(|e| Error::GeneralError(format!("{}", e)))?;
                }

                // case: Malformed request of some sort
                Err(error_response) => {
                    println!(
                        "[rpc daemon] bad request detected, replying to client {:?}",
                        &error_response
                    );
                    resp_stream.respond(&error_response).await?;
                    resp_stream.finish().await?;
                }
            }
        }

        Ok(())
    }
}

/// Initializes a new RpcDaemon object and gives back a stream for
/// server internal queries coming from it
pub fn rpc_daemon() -> (RpcDaemon, QueryStream) {
    let (query_tx, query_rx) = unbounded_channel::<QueryContainer>();
    let (response_tx, response_rx) = unbounded_channel::<ResponseContainer>();
    let daemon = RpcDaemon::new(query_tx, response_rx);
    let query_stream = QueryStream::new(response_tx, query_rx);
    (daemon, query_stream)
}
