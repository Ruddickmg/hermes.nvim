use agent_client_protocol::{Client, LoadSessionRequest, SessionId};
use nvim_oxi::{
    conversion::{Error, FromObject},
    lua::{Error as LuaError, Poppable, Pushable},
    Dictionary, Function, Object,
};
use std::{path::PathBuf, rc::Rc};
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{
    acp::connection::ConnectionManager, api::mcp_servers::parse_mcp_servers,
    nvim::autocommands::ResponseHandler, utilities::project,
};

/// Configuration for loading a session (second argument of the tuple)
#[derive(Debug, Clone)]
pub struct LoadSessionConfig {
    pub cwd: Option<PathBuf>,
    pub mcp_servers: Vec<agent_client_protocol::McpServer>,
}

impl LoadSessionConfig {
    fn default_with_root() -> Self {
        let current_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root = project::get_project_root(current_directory, vec![".git".to_string()]);
        Self {
            cwd: Some(root),
            mcp_servers: Vec::new(),
        }
    }
}

impl FromObject for LoadSessionConfig {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict: Dictionary = obj.try_into()?;

        let cwd: Option<PathBuf> = dict.get("cwd").and_then(|obj| {
            obj.clone()
                .try_into()
                .ok()
                .map(|s: nvim_oxi::String| PathBuf::from(s.to_string()))
        });

        let mcp_servers: Vec<agent_client_protocol::McpServer> = dict
            .get("mcpServers")
            .and_then(parse_mcp_servers)
            .unwrap_or_default();

        let current_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root = project::get_project_root(current_directory, vec![".git".to_string()]);

        Ok(Self {
            cwd: Some(cwd.unwrap_or(root)),
            mcp_servers,
        })
    }
}

impl Poppable for LoadSessionConfig {
    unsafe fn pop(lua_state: *mut nvim_oxi::lua::ffi::State) -> Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Self::from_object(obj).map_err(|e| nvim_oxi::lua::Error::RuntimeError(e.to_string()))
    }
}

impl Pushable for LoadSessionConfig {
    unsafe fn push(
        self,
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> Result<i32, nvim_oxi::lua::Error> {
        let mut dict = Dictionary::new();
        if let Some(cwd) = self.cwd {
            dict.insert("cwd", cwd.to_string_lossy().to_string());
        }
        unsafe { Object::from(dict).push(lua_state) }
    }
}

/// Tuple for two positional arguments: (session_id, config)
/// Lua: loadSession({session_id, config_table})
pub type LoadSessionArgs = (String, Option<LoadSessionConfig>);

#[instrument(level = "trace", skip_all)]
pub fn load_session<H: Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
    let function: Function<LoadSessionArgs, Result<(), LuaError>> =
        Function::from_fn(move |(session_id, maybe_config): LoadSessionArgs| {
            debug!(
                "loadSession function called with session_id: {}",
                session_id
            );

            let config = maybe_config.unwrap_or_else(LoadSessionConfig::default_with_root);

            let request = LoadSessionRequest::new(
                SessionId::from(session_id),
                config.cwd.unwrap_or_else(|| PathBuf::from(".")),
            )
            .mcp_servers(config.mcp_servers);

            connection
                .blocking_lock()
                .get_current_connection()
                .ok_or_else(|| {
                    LuaError::RuntimeError(
                        "No connection found, call the connect function".to_string(),
                    )
                })?
                .load_session(request)?;
            Ok(())
        });
    function.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to verify we can create config objects
    fn create_test_config(cwd: Option<&str>) -> LoadSessionConfig {
        if let Some(path) = cwd {
            LoadSessionConfig {
                cwd: Some(PathBuf::from(path)),
                mcp_servers: Vec::new(),
            }
        } else {
            LoadSessionConfig::default_with_root()
        }
    }

    #[test]
    fn test_config_default_has_cwd() {
        let config = LoadSessionConfig::default_with_root();
        assert!(config.cwd.is_some());
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn test_config_with_custom_cwd() {
        let config = create_test_config(Some("/test/path"));
        assert_eq!(config.cwd, Some(PathBuf::from("/test/path")));
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn test_tuple_type_alias_exists() {
        // This test just verifies the type alias compiles correctly
        // The actual functionality is tested in E2E tests
        let _: Option<LoadSessionArgs> = None;
    }
}
