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
pub enum ErrorKind {
    GeneralError,
    AuthdAlreadyStarted,
    Unexpected,
    Unknown,
}

/// Error types used for sn_auth with support for suggestions. To create an error with suggestions
/// use `Error::from_message_with_suggestions` method. `Error::from_message` can be used if suggestions
/// don't need to be provided such as for internal errors.
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    suggestions: Option<Vec<String>>,
}

impl Error {
    pub fn from_message(kind: ErrorKind, message: String) -> Error {
        Error {
            kind,
            message,
            suggestions: None,
        }
    }

    pub fn from_message_with_suggestions(
        kind: ErrorKind,
        message: String,
        suggestions: Vec<String>,
    ) -> Error {
        Error {
            kind,
            message,
            suggestions: Some(suggestions),
        }
    }

    pub fn from_code(error_code: i32, message: String) -> Self {
        let kind = match error_code {
            1 => ErrorKind::GeneralError,
            10 => ErrorKind::AuthdAlreadyStarted,
            20 => ErrorKind::Unexpected,
            _ => ErrorKind::Unknown,
        };

        Error {
            kind,
            message,
            suggestions: None,
        }
    }

    pub fn from_code_with_suggestion(
        error_code: i32,
        message: String,
        suggestions: Vec<String>,
    ) -> Self {
        let kind = match error_code {
            1 => ErrorKind::GeneralError,
            10 => ErrorKind::AuthdAlreadyStarted,
            20 => ErrorKind::Unexpected,
            _ => ErrorKind::Unknown,
        };

        Error {
            kind,
            message,
            suggestions: Some(suggestions),
        }
    }

    pub fn code(&self) -> i32 {
        self.kind.error_code()
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut message = format!("{} - {}", self.kind.description(), self.message);
        if let Some(suggestions) = &self.suggestions {
            let mut output = vec![message, "Suggestions --- ".to_owned()];
            output.extend_from_slice(&suggestions);
            message = output.join("\n");
        }
        write!(f, "{} - {}", self.kind.description(), message)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::from_message(ErrorKind::GeneralError, error.to_string())
    }
}

impl ErrorKind {
    pub fn error_code(&self) -> i32 {
        use ErrorKind::*;
        // Don't use any of the reserved exit codes:
        // http://tldp.org/LDP/abs/html/exitcodes.html#AEN23549
        match self {
            GeneralError => 1,
            AuthdAlreadyStarted => 10,
            Unexpected => 20,
            Unknown => 1,
        }
    }

    pub fn description(&self) -> &str {
        use ErrorKind::*;
        match self {
            GeneralError => "GeneralError",
            AuthdAlreadyStarted => "AuthdAlreadyStarted",
            Unexpected => "Unexpected",
            Unknown => "Unknown",
        }
    }
}
