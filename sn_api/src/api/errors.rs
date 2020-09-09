// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    AuthError(String),
    AuthdClientError(String),
    AuthdError(String),
    AuthdAlreadyStarted(String),
    AuthenticatorError(String),
    ConnectionError(String),
    NetDataError(String),
    ContentNotFound(String),
    ContentError(String),
    EmptyContent(String),
    AccessDenied(String),
    VersionNotFound(String),
    EntryNotFound(String),
    EntryExists(String),
    InvalidInput(String),
    InvalidAmount(String),
    InvalidXorUrl(String),
    InvalidMediaType(String),
    NotEnoughBalance(String),
    FileSystemError(String),
    Unexpected(String),
    Unknown(String),
}

impl From<Error> for String {
    fn from(error: Error) -> String {
        error.to_string()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        let (error_type, error_msg) = match self {
            AuthError(info) => ("AuthError", info),
            AuthdClientError(info) => ("AuthdClientError", info),
            AuthdError(info) => ("AuthdError", info),
            AuthdAlreadyStarted(info) => ("AuthdAlreadyStarted", info),
            AuthenticatorError(info) => ("AuthenticatorError", info),
            ConnectionError(info) => ("ConnectionError", info),
            NetDataError(info) => ("NetDataError", info),
            ContentNotFound(info) => ("ContentNotFound", info),
            VersionNotFound(info) => ("VersionNotFound", info),
            ContentError(info) => ("ContentError", info),
            EmptyContent(info) => ("EmptyContent", info),
            AccessDenied(info) => ("AccessDenied", info),
            EntryNotFound(info) => ("EntryNotFound", info),
            EntryExists(info) => ("EntryExists", info),
            InvalidInput(info) => ("InvalidInput", info),
            InvalidAmount(info) => ("InvalidAmount", info),
            InvalidXorUrl(info) => ("InvalidXorUrl", info),
            InvalidMediaType(info) => ("InvalidMediaType", info),
            NotEnoughBalance(info) => ("NotEnoughBalance", info),
            FileSystemError(info) => ("FileSystemError", info),
            Unexpected(info) => ("Unexpected", info),
            Unknown(info) => ("Unknown", info),
        };
        let description = format!("[Error] {} - {}", error_type, error_msg);

        write!(f, "{}", description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = Error::Unknown("test error".to_string());
        let s: String = err.into();
        assert_eq!(s, "[Error] Unknown - test error");
    }
}
