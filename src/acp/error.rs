// TODO: refine these error conversions to be more meaningful
use crate::nvim::autocommands::Commands;
use agent_client_protocol::Error as AcpError;
use nvim_oxi::{api, lua};
use std::sync::{PoisonError, mpsc::SendError};

#[derive(Debug, Clone)]
pub enum Error {
    Internal(String),
    Connection(String),
    Permissions(String),
    NoListenerAttached(Commands),
    InvalidInput(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connection(msg) => write!(f, "Connection error: {}", msg),
            Error::Permissions(msg) => write!(f, "Permissions error: {}", msg),
            Error::Internal(msg) => write!(f, "Internal error: {}", msg),
            Error::InvalidInput(input) => write!(f, "Invalid input provided: {}", input),
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

impl From<nvim_oxi::conversion::Error> for Error {
    fn from(e: nvim_oxi::conversion::Error) -> Self {
        Error::InvalidInput(e.to_string())
    }
}

impl From<Error> for agent_client_protocol::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::NoListenerAttached(_) => AcpError::method_not_found(),
            Error::InvalidInput(_) => AcpError::invalid_params(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::sync::mpsc::channel;

    #[test]
    fn test_error_display_connection() {
        let err = Error::Connection("connection failed".to_string());
        assert_eq!(format!("{}", err), "Connection error: connection failed");
    }

    #[test]
    fn test_error_display_permissions() {
        let err = Error::Permissions("not allowed".to_string());
        assert_eq!(format!("{}", err), "Permissions error: not allowed");
    }

    #[test]
    fn test_error_display_internal() {
        let err = Error::Internal("something went wrong".to_string());
        assert_eq!(format!("{}", err), "Internal error: something went wrong");
    }

    #[test]
    fn test_error_display_invalid_input() {
        let err = Error::InvalidInput("bad data".to_string());
        assert_eq!(format!("{}", err), "Invalid input provided: bad data");
    }

    #[test]
    fn test_error_display_no_listener_contains_message() {
        let err = Error::NoListenerAttached(Commands::ToolCall);
        let display = format!("{}", err);
        assert!(display.contains("No listener attached"));
    }

    #[test]
    fn test_error_display_no_listener_contains_command() {
        let err = Error::NoListenerAttached(Commands::ToolCall);
        let display = format!("{}", err);
        assert!(display.contains("ToolCall"));
    }

    #[test]
    fn test_from_send_error() {
        let (sender, _receiver) = channel::<String>();
        drop(_receiver);
        let result = sender.send("test".to_string());
        if let Err(send_err) = result {
            let error: Error = send_err.into();
            assert!(matches!(error, Error::Internal(_)));
        }
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_result: Result<serde_json::Value, _> = serde_json::from_str("invalid json");
        if let Err(json_err) = json_result {
            let error: Error = json_err.into();
            assert!(matches!(error, Error::Internal(_)));
        }
    }

    #[test]
    fn test_from_error_to_acp_error_no_listener() {
        let err = Error::NoListenerAttached(Commands::PermissionRequest);
        let acp_err: AcpError = err.into();
        // Just verify it converts successfully (exact message content may vary)
        let _ = acp_err.to_string();
    }

    #[test]
    fn test_from_error_to_acp_error_invalid_input() {
        let err = Error::InvalidInput("bad".to_string());
        let acp_err: AcpError = err.into();
        // Just verify it converts successfully
        let _ = acp_err.to_string();
    }

    #[test]
    fn test_from_error_to_acp_error_internal() {
        let err = Error::Internal("test".to_string());
        let acp_err: AcpError = err.into();
        // Just verify it converts successfully
        let _ = acp_err.to_string();
    }

    #[test]
    fn test_from_error_to_lua_error_contains_message() {
        let err = Error::Internal("test error".to_string());
        let lua_err: lua::Error = err.into();
        match lua_err {
            lua::Error::RuntimeError(msg) => {
                assert!(msg.contains("test error"));
            }
            _ => panic!("Expected RuntimeError"),
        }
    }

    #[test]
    fn test_from_acp_error_to_error() {
        let acp_err = AcpError::internal_error();
        let err: Error = acp_err.into();
        assert!(matches!(err, Error::Internal(_)));
    }
}
