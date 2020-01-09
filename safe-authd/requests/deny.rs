// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use crate::shared::{lock_auth_reqs_list, SharedAuthReqsHandle};
use serde_json::{json, Value};

pub fn process_req(params: Value, auth_reqs_handle: SharedAuthReqsHandle) -> Result<Value, String> {
    if let Value::String(auth_req_id) = params {
        println!("Denying authorisation request...");
        let req_id = match auth_req_id.parse::<u32>() {
            Ok(id) => id,
            Err(err) => return Err(err.to_string()),
        };
        lock_auth_reqs_list(auth_reqs_handle, |auth_reqs_list| {
            match auth_reqs_list.remove(&req_id) {
                Some(mut auth_req) => match auth_req.tx.try_send(false) {
                    Ok(_) => {
                        let msg = format!(
                            "Authorisation request ({}) denied successfully",
                            auth_req_id
                        );
                        println!("{}", msg);
                        Ok(json!(msg))
                    }
                    Err(_) => {
                        let msg = format!("Authorisation request '{}' was already denied since the response couldn't be sent to the requesting application", auth_req_id);
                        println!("{}", msg);
                        Err(msg)
                    }
                },
                None => {
                    let msg = format!(
                        "No pending authorisation request found with id '{}'",
                        auth_req_id
                    );
                    println!("{}", msg);
                    Err(msg)
                }
            }
        })
    } else {
        Err(format!("Incorrect params for 'deny' method: {:?}", params))
    }
}
