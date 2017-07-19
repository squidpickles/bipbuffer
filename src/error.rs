use std::fmt;
use std::error;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorKind {
    NoSpace,
    Empty,
}

impl ErrorKind {
    fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::NoSpace => "no space",
            ErrorKind::Empty => "empty",
        }
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Error {
        Error { kind: kind }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { kind: kind }
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

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}
