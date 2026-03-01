use nvim_oxi::lua;
use std::sync::{PoisonError, mpsc::SendError};

use crate::apc::connection::UserRequest;

#[derive(Debug, Clone)]
pub enum Error {
    Internal(String),
    Connection(String),
    Permissions(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "Connection error: {}", msg),
            Error::Permissions(msg) => write!(f, "Permissions error: {}", msg),
            Error::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<SendError<UserRequest>> for Error {
    fn from(e: SendError<UserRequest>) -> Self {
        Error::Internal(e.to_string())
    }
}

impl From<Error> for lua::Error {
    fn from(e: Error) -> Self {
        lua::Error::RuntimeError(e.to_string())
    }
}

impl From<agent_client_protocol::Error> for Error {
    fn from(e: agent_client_protocol::Error) -> Self {
        Error::Internal(e.to_string())
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(e: PoisonError<T>) -> Self {
        Error::Internal(e.to_string())
    }
}
