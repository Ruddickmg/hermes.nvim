use agent_client_protocol::ListSessionsRequest;
use nvim_oxi::{
    Dictionary, Function, Object,
    conversion::{Error, FromObject},
    lua::{Error as LuaError, Poppable, Pushable},
};
use tokio::sync::Mutex;
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};
use tracing::{debug, error, instrument};

use crate::{PluginState, acp::connection::ConnectionManager};

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
pub fn list_sessions(connection: Rc<RefCell<ConnectionManager>>, state: Arc<Mutex<PluginState>>) -> Object {
    let function: Function<Option<ListSessionsConfig>, Result<(), LuaError>> =
        Function::from_fn(move |maybe_config: Option<ListSessionsConfig>| {
            debug!("listSessions function called");
            let plugin_state = state.blocking_lock();
            let agent_info = plugin_state.agent_info.clone();
            drop(plugin_state);

            if !agent_info.can_list_sessions() {
                return Ok(())
            }

            let config = maybe_config.unwrap_or_default();

            let mut request = ListSessionsRequest::new();

            if let Some(cwd) = config.cwd {
                request = request.cwd(cwd);
            }

            if let Some(cursor) = config.cursor {
                request = request.cursor(cursor);
            }

            let conn = match connection.borrow().get_current_connection() {
                Some(c) => c,
                None => {
                    error!("No connection found, call the connect function");
                    return Ok(());
                }
            };

            if let Err(e) = conn.list_sessions(request) {
                error!("Error listing sessions: {:?}", e);
            }
            Ok(())
        });
    function.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    // Helper to create a Dictionary with cwd field
    fn create_cwd_dict(path: &str) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("cwd", path);
        dict
    }

    // Helper to create a Dictionary with cursor field
    fn create_cursor_dict(cursor: &str) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("cursor", cursor);
        dict
    }

    // Helper to create a Dictionary with both fields
    fn create_full_dict(path: &str, cursor: &str) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("cwd", path);
        dict.insert("cursor", cursor);
        dict
    }

    // Strategy for generating valid path strings
    fn arb_path() -> impl Strategy<Value = String> {
        prop_oneof!(
            Just("/home/user/project".to_string()),
            Just("/var/www/hermes".to_string()),
            Just("C:\\Users\\name\\project".to_string()),
            Just(".".to_string()),
            Just("./relative".to_string()),
            Just("../parent".to_string()),
            Just("/".to_string()),
            Just("/path/with spaces".to_string()),
            "[/a-zA-Z0-9._-]{1,100}".prop_map(|s| format!("/{}", s))
        )
    }

    // Strategy for generating cursor strings (alphanumeric, often base64-like)
    fn arb_cursor() -> impl Strategy<Value = String> {
        prop_oneof!(
            Just("abc123".to_string()),
            Just("xyz789".to_string()),
            Just("MTIzYWJj".to_string()),
            Just("".to_string()),
            "[a-zA-Z0-9_-]{0,50}".prop_map(|s| s.to_string())
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        #[test]
        fn test_from_object_with_various_cwd_paths(path in arb_path()) {
            // Property: Any valid path string should be parsed without panicking
            let dict = create_cwd_dict(&path);
            let obj = Object::from(dict);
            let result = ListSessionsConfig::from_object(obj);
            prop_assert!(result.is_ok(), "Should successfully parse any path string");

            let config = result.unwrap();
            prop_assert_eq!(config.cwd, Some(PathBuf::from(&path)));
        }

        #[test]
        fn test_from_object_with_various_cursors(cursor in arb_cursor()) {
            // Property: Any cursor string (including empty) should be parsed
            let dict = create_cursor_dict(&cursor);
            let obj = Object::from(dict);
            let result = ListSessionsConfig::from_object(obj);
            prop_assert!(result.is_ok(), "Should successfully parse any cursor string");

            let config = result.unwrap();
            prop_assert_eq!(config.cursor, Some(cursor));
        }

        #[test]
        fn test_from_object_with_both_fields(path in arb_path(), cursor in arb_cursor()) {
            // Property: Combined cwd and cursor should both be parsed
            let dict = create_full_dict(&path, &cursor);
            let obj = Object::from(dict);
            let result = ListSessionsConfig::from_object(obj);
            prop_assert!(result.is_ok(), "Should successfully parse combined fields");

            let config = result.unwrap();
            prop_assert_eq!(config.cwd, Some(PathBuf::from(&path)));
            prop_assert_eq!(config.cursor, Some(cursor));
        }
    }

    // Test nil input - should create default config
    #[test]
    fn test_from_object_nil_creates_default_config() {
        let obj = Object::nil();
        let config = ListSessionsConfig::from_object(obj).unwrap();
        // Single assertion checking both fields are None
        assert!(
            config.cwd.is_none() && config.cursor.is_none(),
            "Nil input should create default config with None fields"
        );
    }

    // Test empty dictionary - should create default config
    #[test]
    fn test_from_object_empty_dict_creates_default_config() {
        let dict = Dictionary::new();
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        // Single assertion checking both fields are None
        assert!(
            config.cwd.is_none() && config.cursor.is_none(),
            "Empty dict should create default config with None fields"
        );
    }

    // Test cwd field parsing
    #[test]
    fn test_from_object_with_absolute_cwd() {
        let dict = create_cwd_dict("/home/user/project");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cwd, Some(PathBuf::from("/home/user/project")));
    }

    #[test]
    fn test_from_object_with_relative_cwd() {
        let dict = create_cwd_dict("./relative/path");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cwd, Some(PathBuf::from("./relative/path")));
    }

    #[test]
    fn test_from_object_with_windows_cwd() {
        let dict = create_cwd_dict("C:\\Users\\name\\project");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cwd, Some(PathBuf::from("C:\\Users\\name\\project")));
    }

    // Test cursor field parsing
    #[test]
    fn test_from_object_with_alphanumeric_cursor() {
        let dict = create_cursor_dict("abc123");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cursor, Some("abc123".to_string()));
    }

    #[test]
    fn test_from_object_with_base64_like_cursor() {
        let dict = create_cursor_dict("MTIzYWJj");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cursor, Some("MTIzYWJj".to_string()));
    }

    #[test]
    fn test_from_object_with_empty_cursor() {
        let dict = create_cursor_dict("");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cursor, Some("".to_string()));
    }

    // Test combined fields - single assertion for both fields
    #[test]
    fn test_from_object_with_both_fields_parses_correctly() {
        let dict = create_full_dict("/path/to/project", "xyz789");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        // Single assertion verifying both fields parsed correctly
        assert_eq!(
            (config.cwd, config.cursor),
            (
                Some(PathBuf::from("/path/to/project")),
                Some("xyz789".to_string())
            )
        );
    }

    // Test unknown fields are ignored (only known fields should be parsed)
    #[test]
    fn test_from_object_ignores_unknown_fields() {
        let mut dict = Dictionary::new();
        dict.insert("cwd", "/valid/path");
        dict.insert("unknown", "should_be_ignored");
        dict.insert("cursor", "valid_cursor");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cwd, Some(PathBuf::from("/valid/path")));
    }

    #[test]
    fn test_from_object_with_only_unknown_fields_is_default() {
        let mut dict = Dictionary::new();
        dict.insert("unknown", "ignored");
        dict.insert("another", "also_ignored");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        // Single assertion checking both fields are None
        assert!(
            config.cwd.is_none() && config.cursor.is_none(),
            "Unknown fields should result in default config"
        );
    }

    // Test error path - invalid object type
    #[test]
    fn test_from_object_with_invalid_type_returns_error() {
        // Array is not a valid input for ListSessionsConfig
        let arr = nvim_oxi::Array::from_iter(vec![Object::from("test")]);
        let obj = Object::from(arr);
        let result = ListSessionsConfig::from_object(obj);
        assert!(
            result.is_err(),
            "Should error on non-dictionary/non-nil input"
        );
    }

    // Test path edge cases
    #[test]
    fn test_from_object_with_special_characters_in_path() {
        let mut dict = Dictionary::new();
        dict.insert("cwd", "/path/with-unicode-日本語");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cwd, Some(PathBuf::from("/path/with-unicode-日本語")));
    }

    #[test]
    fn test_from_object_with_special_characters_in_cursor() {
        let mut dict = Dictionary::new();
        dict.insert("cursor", "cursor-with-chars_123.test");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(
            config.cursor,
            Some("cursor-with-chars_123.test".to_string())
        );
    }

    #[test]
    fn test_from_object_with_very_long_cursor() {
        let long_cursor = "a".repeat(1000);
        let mut dict = Dictionary::new();
        dict.insert("cursor", long_cursor.clone());
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(config.cursor, Some(long_cursor));
    }

    #[test]
    fn test_from_object_with_nested_path() {
        let mut dict = Dictionary::new();
        dict.insert("cwd", "/very/nested/deep/path/to/project");
        let obj = Object::from(dict);
        let config = ListSessionsConfig::from_object(obj).unwrap();
        assert_eq!(
            config.cwd,
            Some(PathBuf::from("/very/nested/deep/path/to/project"))
        );
    }
}
