use agent_client_protocol::ListSessionsRequest;
use nvim_oxi::{
    conversion::{Error, FromObject},
    lua::{Error as LuaError, Poppable, Pushable},
    Dictionary, Function, Object,
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use tracing::{debug, instrument};

use crate::acp::connection::ConnectionManager;

/// Configuration for listing sessions (optional argument)
#[derive(Debug, Clone, Default)]
pub struct ListSessionsConfig {
    pub cwd: Option<PathBuf>,
    pub cursor: Option<String>,
}

impl FromObject for ListSessionsConfig {
    fn from_object(obj: Object) -> Result<Self, Error> {
        if obj.is_nil() {
            return Ok(Self::default());
        }

        let dict: Dictionary = obj.try_into()?;

        let cwd: Option<PathBuf> = dict.get("cwd").and_then(|obj| {
            obj.clone()
                .try_into()
                .ok()
                .map(|s: nvim_oxi::String| PathBuf::from(s.to_string()))
        });

        let cursor: Option<String> = dict.get("cursor").and_then(|obj| {
            obj.clone()
                .try_into()
                .ok()
                .map(|s: nvim_oxi::String| s.to_string())
        });

        Ok(Self { cwd, cursor })
    }
}

impl Poppable for ListSessionsConfig {
    unsafe fn pop(lua_state: *mut nvim_oxi::lua::ffi::State) -> Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Self::from_object(obj).map_err(|e| nvim_oxi::lua::Error::RuntimeError(e.to_string()))
    }
}

impl Pushable for ListSessionsConfig {
    unsafe fn push(
        self,
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> Result<i32, nvim_oxi::lua::Error> {
        let mut dict = Dictionary::new();
        if let Some(cwd) = self.cwd {
            dict.insert("cwd", cwd.to_string_lossy().to_string());
        }
        if let Some(cursor) = self.cursor {
            dict.insert("cursor", cursor);
        }
        unsafe { Object::from(dict).push(lua_state) }
    }
}

#[instrument(level = "trace", skip_all)]
pub fn list_sessions(connection: Rc<RefCell<ConnectionManager>>) -> Object {
    let function: Function<Option<ListSessionsConfig>, Result<(), LuaError>> =
        Function::from_fn(move |maybe_config: Option<ListSessionsConfig>| {
            debug!("listSessions function called");

            let config = maybe_config.unwrap_or_default();

            let mut request = ListSessionsRequest::new();

            if let Some(cwd) = config.cwd {
                request = request.cwd(cwd);
            }

            if let Some(cursor) = config.cursor {
                request = request.cursor(cursor);
            }

            connection
                .borrow()
                .get_current_connection()
                .ok_or_else(|| {
                    LuaError::RuntimeError(
                        "No connection found, call the connect function".to_string(),
                    )
                })?
                .list_sessions(request)?;
            Ok(())
        });
    function.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_object_nil() {
        let obj = Object::nil();
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert!(config.cwd.is_none());
        assert!(config.cursor.is_none());
    }

    #[test]
    fn test_from_object_empty_dict() {
        let dict = Dictionary::new();
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert!(config.cwd.is_none());
        assert!(config.cursor.is_none());
    }

    #[test]
    fn test_from_object_with_cwd() {
        let mut dict = Dictionary::new();
        dict.insert("cwd", "/path/to/project");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cwd, Some(PathBuf::from("/path/to/project")));
        assert!(config.cursor.is_none());
    }

    #[test]
    fn test_from_object_with_cursor() {
        let mut dict = Dictionary::new();
        dict.insert("cursor", "abc123");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert!(config.cwd.is_none());
        assert_eq!(config.cursor, Some("abc123".to_string()));
    }

    #[test]
    fn test_from_object_with_both() {
        let mut dict = Dictionary::new();
        dict.insert("cwd", "/path/to/project");
        dict.insert("cursor", "xyz789");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cwd, Some(PathBuf::from("/path/to/project")));
        assert_eq!(config.cursor, Some("xyz789".to_string()));
    }
}
