//! Errors used in the Bip-Buffer implementation

use std::error;
use std::fmt;

/// The error type
#[derive(Clone, Copy, Debug)]
pub struct Error {
    /// Contains the type of error
    kind: ErrorKind,
}

/// Specific error types
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    /// No space is available for writing to the buffer; data must be marked read by calling
    /// [`decommit()`](struct.BipBuffer.html#method.decommit)
    NoSpace,
}

impl ErrorKind {
    fn as_str(self) -> &'static str {
        match self {
            ErrorKind::NoSpace => "no space",
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { kind }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.kind.as_str())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        self.kind.as_str()
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}
