// copyright 2021 maidsafe.net limited.
//
// this safe network software is licensed to you under the general public license (gpl), version 3.
// unless required by applicable law or agreed to in writing, the safe network software distributed
// under the gpl licence is distributed on an "as is" basis, without warranties or conditions of any
// kind, either express or implied. please review the licences for the specific language governing
// permissions and limitations relating to use of the safe network software.

///! A module for stream types returned by the daemon server.
///! `QueryStream` is a buffered stream for incoming queries
///! that have been pre-parsed by the daemon. `ResponseStream`
///! is another buffered stream returned by `QueryStream`
///! along with each query. It must be used by a
///! server process to buffer responses to a query.
///!
///! Finally, this also provides the Container<T> type
///! which is used internally by the rpc daemon to keep track
///! of metadata like identifiers for each query/response in the
///! stream.
///!
///! NOTE: Currently `qjsonrpc` doesn't support Notification types
///! (e.g. a request where no response is required), so
///! `ResponseStream.send_oneshot()` *must* be called
///! to consume `ResponseStream` or the thread will panic when
///! `ResponseStream` is dropped.
use crate::{query::*, response::*};
use log::error;
use qjsonrpc::{JsonRpcRequest, JsonRpcResponse, Result};
use serde_json::json;
use std::convert::{From, TryFrom};
use tokio::sync::mpsc;

// ====================================
//             Query Stream
// ====================================

/// An outward facing stream which yields queries
/// parsed from JsonRpcRequests and an associated `ResponseStream`
pub struct QueryStream {
    /// Buffered stream for outgoing responses
    response_tx: mpsc::UnboundedSender<ResponseContainer>,

    /// Buffered stream for incoming queries
    query_rx: mpsc::UnboundedReceiver<QueryContainer>,
}

impl QueryStream {
    /// Ctor
    pub fn new(
        response_tx: mpsc::UnboundedSender<ResponseContainer>,
        query_rx: mpsc::UnboundedReceiver<QueryContainer>,
    ) -> Self {
        Self {
            response_tx,
            query_rx,
        }
    }

    /// Returns tuple (query, ResponseStream) or None if all senders
    /// are dropped.
    pub async fn get_next(&mut self) -> Option<(Query, ResponseStream)> {
        let (query, id) = self.query_rx.recv().await?.into_tuple();
        let resp_stream = ResponseStream::new(self.response_tx.clone(), id);
        Some((query, resp_stream))
    }
}

// =======================================
//             Response Stream
// =======================================

/// An associated type for query stream used to respond to
/// queries once they've been converted to an internal
/// server representation.
///
/// The function `send_oneshot()` *must* be called by the server
/// before the response stream is dropped as the current `qjsonrpc`
/// implementation doesn't support requests with no response.
pub struct ResponseStream {
    /// channel to buffer the output stream
    response_tx: mpsc::UnboundedSender<ResponseContainer>,

    /// id of the request we're responding to
    id: u32,

    /// ensure that one_shot() was called before `drop()`
    was_consumed: bool,
}

impl ResponseStream {
    /// Ctor
    fn new(response_tx: mpsc::UnboundedSender<ResponseContainer>, id: u32) -> Self {
        let was_consumed = false;
        Self {
            response_tx,
            id,
            was_consumed,
        }
    }

    /// Send a response along the pipeline
    /// and consumes the stream in the process.
    ///
    /// Implementation Note:
    /// `response_tx.send()` can error, but it should
    /// never do so here logically (hence the `assert!`).
    /// If send does `Error` we have one of two cases.
    ///     a) `RpcDaemon.run()` returned already.
    ///         This case isn't possible because it only exits
    ///         when all senders (like this stream) are dropped
    ///     b) You somehow got a `ResponseStream` that wasn't
    ///        constructed yielded from a `QueryStream` (also not possible)
    pub fn send_oneshot(mut self, res: Result<Response>) {
        self.was_consumed = true;
        let container = ResponseContainer::new(res, self.id);
        let r = self.response_tx.send(container);
        assert!(r.is_ok());
    }
}

impl Drop for ResponseStream {
    /// If a response stream was yielded from `get_next()`
    /// the server is not allowed to ignore it
    /// as per the JSON RPC 2.0 spec
    fn drop(&mut self) {
        assert!(self.was_consumed);
    }
}

// ========================================
//              Container Type
// ========================================

pub type QueryContainer = Container<Query>;
pub type ResponseContainer = Container<Result<Response>>;

/// A container type used internally by the QueryStream
/// and Response Stream which tags messages passing through
/// with metadata so that the server service and the client
/// need not worry about them directly.
///
/// Implementation Note on the id field:
/// We don't allow `id` to be an `Option` because Null id requests
/// (e.g. Notification in JSON RPC lingo) are not
/// supported yet by `qjsonrpc`.
#[derive(Debug)]
pub struct Container<T> {
    /// The wrapped data
    val: T,

    /// The id of the query/response
    id: u32,
}

impl<T> Container<T> {
    /// Ctor
    pub fn new(val: T, id: u32) -> Self {
        Self { val, id }
    }

    /// Get the id
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get a reference to the value stored inside
    pub fn val(&self) -> &T {
        &self.val
    }

    /// Consume this container to get back (val,id)
    pub fn into_tuple(self) -> (T, u32) {
        (self.val, self.id)
    }
}

impl From<ResponseContainer> for JsonRpcResponse {
    /// Convert a ResponseContainer to a jsonrpc response
    fn from(container: ResponseContainer) -> Self {
        match container.val() {
            Ok(resp) => JsonRpcResponse::result(json!(resp), container.id()),
            Err(e) => {
                let msg = format!("{}", e);
                error!("{}", msg);
                JsonRpcResponse::error(msg, JSONRPC_ASYNC_SERVER_ERROR, Some(container.id()))
            }
        }
    }
}

impl TryFrom<JsonRpcRequest> for QueryContainer {
    type Error = JsonRpcResponse;

    /// Convert JsonRpcRequest to a query container
    /// or return a JsonRpcResponse to use as the error response
    fn try_from(request: JsonRpcRequest) -> std::result::Result<Self, Self::Error> {
        match request.method.as_str() {
            METHOD_PING => Ok(QueryContainer::new(Query::Ping, request.id)),
            METHOD_SHUTDOWN => Ok(QueryContainer::new(Query::Shutdown, request.id)),
            other => {
                let msg = format!("Method '{}' not supported or unknown by the server", other);
                error!("{}", msg);
                Err(JsonRpcResponse::error(
                    msg,
                    JSONRPC_METHOD_NOT_FOUND,
                    Some(request.id),
                ))
            }
        }
    }
}
