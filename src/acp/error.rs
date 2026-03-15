// TODO: refine these error conversions to be more meaningful
use crate::nvim::autocommands::Commands;
use agent_client_protocol::Error as AcpError;
use nvim_oxi::{api, lua};
use std::{
    path::PathBuf,
    sync::{PoisonError, mpsc::SendError},
};

#[derive(Debug, Clone)]
pub enum Error {
    Internal(String),
    Connection(String),
    Permissions(String),
    NoListenerAttached(Commands),
    FileNotFound(PathBuf),
    InvalidLineRange { start: u32, end: u32 },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "Connection error: {}", msg),
            Error::Permissions(msg) => write!(f, "Permissions error: {}", msg),
            Error::Internal(msg) => write!(f, "Internal error: {}", msg),
            Error::FileNotFound(path) => write!(f, "File not found: {:?}", path),
            Error::InvalidLineRange { start, end } => {
                write!(f, "Invalid line range: start ({}), end ({})", start, end)
            }
            Error::NoListenerAttached(command) => {
                write!(f, "No listener attached for autocommand: {}", command)
            }
        }
    }
}

impl std::error::Error for Error {}

impl<T> From<SendError<T>> for Error {
    fn from(e: SendError<T>) -> Self {
        Error::Internal(e.to_string())
    }
}

impl From<Error> for agent_client_protocol::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::NoListenerAttached(_) => AcpError::method_not_found(),
            e => AcpError::into_internal_error(e),
        }
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

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Internal(e.to_string())
    }
}

impl From<nvim_oxi::Error> for Error {
    fn from(e: nvim_oxi::Error) -> Self {
        Error::Internal(e.to_string())
    }
}

impl From<Error> for nvim_oxi::Error {
    fn from(e: Error) -> Self {
        nvim_oxi::Error::Api(api::Error::Other(e.to_string()))
    }
}
