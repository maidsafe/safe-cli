// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

#[cfg(feature = "app")]
use crate::{api::common::parse_hex, Error, Result};

use chrono::{DateTime, SecondsFormat, Utc};
use futures::Future;
use sn_data_types::{Error as SafeNdError, PublicKey, Token};
use std::{
    str::{self, FromStr},
    time,
};
use tokio::time::sleep;
use xor_name::XorName;

/// The conversion from coin to raw value
const COIN_TO_RAW_CONVERSION: u64 = 1_000_000_000;
// The maximum amount of safecoin that can be represented by a single `Token`
const MAX_COINS_VALUE: u64 = (u32::max_value() as u64 + 1) * COIN_TO_RAW_CONVERSION - 1;

const MAX_RETRIES: u8 = 3;

#[allow(dead_code)]
pub fn vec_to_hex(hash: Vec<u8>) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn xorname_to_hex(xorname: &XorName) -> String {
    xorname.0.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn pk_to_hex(pk: &PublicKey) -> String {
    let xorname = XorName::from(*pk);
    xorname_to_hex(&xorname)
}

pub fn pk_from_hex(hex_str: &str) -> Result<PublicKey> {
    let pk_bytes = parse_hex(&hex_str);
    let ed25519_pk = ed25519_dalek::PublicKey::from_bytes(&pk_bytes).map_err(|_| {
        Error::InvalidInput(format!("Invalid Ed25519 public key bytes: {}", hex_str))
    })?;
    Ok(PublicKey::Ed25519(ed25519_pk))
}

pub fn parse_coins_amount(amount_str: &str) -> Result<Token> {
    Token::from_str(amount_str).map_err(|err| {
        match err {
            SafeNdError::ExcessiveValue => Error::InvalidAmount(format!(
                "Invalid safecoins amount '{}', it exceeds the maximum possible value '{}'",
                amount_str, Token::from_nano(MAX_COINS_VALUE)
            )),
            SafeNdError::LossOfPrecision => {
                Error::InvalidAmount(format!("Invalid safecoins amount '{}', the minimum possible amount is one nano coin (0.000000001)", amount_str))
            }
            SafeNdError::FailedToParse(msg) => {
                Error::InvalidAmount(format!("Invalid safecoins amount '{}' ({})", amount_str, msg))
            },
            _ => Error::InvalidAmount(format!("Invalid safecoins amount '{}'", amount_str)),
        }
    })
}

pub fn systemtime_to_rfc3339(t: &time::SystemTime) -> String {
    let datetime: DateTime<Utc> = t.clone().into();
    datetime.to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn gen_timestamp_secs() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}
/*
pub async fn retry_loop2<F, Fut, T>(mut f: F) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    for _ in 0..MAX_RETRIES {
        match f().await {
            Ok(value) => return value,
            Err(_) => sleep(std::time::Duration::from_millis(200)).await,
        }
    }

    panic!("Failed all {} attempts", MAX_RETRIES)
}
*/
#[macro_export]
macro_rules! retry_loop2 {
    ($async_func:expr) => {
        loop {
            match $async_func.await {
                Ok(val) => break val,
                Err(_) => tokio::time::sleep(std::time::Duration::from_millis(200)).await,
            }
        }
    };
}
