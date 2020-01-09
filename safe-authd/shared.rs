// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use safe_api::{AuthReq, SafeAuthenticator};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct IncomingAuthReq {
    pub timestamp: SystemTime,
    pub auth_req: AuthReq,
    pub tx: mpsc::Sender<bool>,
    pub notified: bool,
}

// List of authorisation requests indexed by their request id
pub type AuthReqsList = BTreeMap<u32, IncomingAuthReq>;

// A thread-safe queue to keep the list of authorisation requests
pub type SharedAuthReqsHandle = Arc<Mutex<AuthReqsList>>;

// A thread-safe handle to keep the SafeAuthenticator instance
pub type SharedSafeAuthenticatorHandle = Arc<Mutex<SafeAuthenticator>>;

// A thread-safe handle to keep the list of notifications subscribers' endpoints,
// we also keep the certificates' base path which is needed to create the communication channel
pub type SharedNotifEndpointsHandle = Arc<Mutex<BTreeMap<String, Option<String>>>>;

pub fn lock_safe_authenticator<F, R>(
    safe_auth_handle: SharedSafeAuthenticatorHandle,
    mut f: F,
) -> Result<R, String>
where
    F: FnMut(&mut SafeAuthenticator) -> Result<R, String>,
{
    match safe_auth_handle.lock() {
        Err(err) => Err(format!(
            "Unexpectedly failed to obtain lock of the authenticator lib instance: {}",
            err
        )),
        Ok(mut locked_auth) => {
            let safe_authenticator: &mut SafeAuthenticator = &mut *(locked_auth);
            f(safe_authenticator)
        }
    }
}

pub fn lock_auth_reqs_list<F, R>(
    auth_reqs_handle: SharedAuthReqsHandle,
    mut f: F,
) -> Result<R, String>
where
    F: FnMut(&mut AuthReqsList) -> Result<R, String>,
{
    match auth_reqs_handle.lock() {
        Err(err) => Err(format!(
            "Unexpectedly failed to obtain lock of pending auth reqs list: {}",
            err
        )),
        Ok(mut locked_list) => {
            let auth_reqs_list: &mut AuthReqsList = &mut *(locked_list);
            f(auth_reqs_list)
        }
    }
}

pub fn lock_notif_endpoints_list<F, R>(
    notif_endpoints_handle: SharedNotifEndpointsHandle,
    mut f: F,
) -> Result<R, String>
where
    F: FnMut(&mut BTreeMap<String, Option<String>>) -> Result<R, String>,
{
    match notif_endpoints_handle.lock() {
        Err(err) => Err(format!(
            "Unexpectedly failed to obtain lock of list of notif subscribers: {}",
            err
        )),
        Ok(mut locked_list) => {
            let notif_endpoints_list: &mut BTreeMap<String, Option<String>> = &mut *(locked_list);
            f(notif_endpoints_list)
        }
    }
}

pub fn remove_auth_req_from_list(auth_reqs_handle: SharedAuthReqsHandle, req_id: u32) {
    let _ = lock_auth_reqs_list(auth_reqs_handle, |auth_reqs_list| {
        auth_reqs_list.remove(&req_id);
        Ok(())
    });
}

pub fn remove_notif_endpoint_from_list(
    notif_endpoints_handle: SharedNotifEndpointsHandle,
    url: &str,
) {
    let _ = lock_notif_endpoints_list(notif_endpoints_handle, |notif_endpoints_list| {
        notif_endpoints_list.remove(url);
        Ok(())
    });
}
