// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::shared::{lock_safe_authenticator, SharedSafeAuthenticatorHandle};

pub fn process_req(
    args: &[&str],
    safe_auth_handle: SharedSafeAuthenticatorHandle,
) -> Result<String, String> {
    if args.len() != 3 {
        Err("Incorrect number of arguments for 'create' action".to_string())
    } else {
        println!("Creating an account in SAFE...");
        let passphrase = urlencoding::decode(args[0])
            .map_err(|_| "The passphrase couldn't be decoded from the request".to_string())?;
        let password = urlencoding::decode(args[1])
            .map_err(|_| "The password couldn't be decoded from the request".to_string())?;
        let sk = args[2];

        lock_safe_authenticator(
            safe_auth_handle,
            |safe_authenticator| match safe_authenticator.create_acc(sk, &passphrase, &password) {
                Ok(_) => {
                    let msg = "Account created successfully";
                    println!("{}", msg);
                    Ok(msg.to_string())
                }
                Err(err) => {
                    println!("Error occurred when trying to create SAFE account: {}", err);
                    Err(err.to_string())
                }
            },
        )
    }
}
