//! The shared application/service-level error used by generated repository
//! and service traits. See `docs/05-rest-client-cli.md` for the REST-layer
//! mapping of these variants onto the stable error envelope (DCL-0004).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The request was structurally or semantically invalid, e.g. a missing
    /// required precondition.
    InvalidArgument(String),
    /// No resource exists for the given identifier.
    NotFound,
    /// The expected revision did not match the current resource revision.
    Conflict,
    /// An unexpected internal failure.
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            Error::NotFound => write!(f, "not found"),
            Error::Conflict => write!(f, "conflict"),
            Error::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}
