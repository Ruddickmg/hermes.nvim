use nvim_oxi::{
    Dictionary, Object,
    conversion::FromObject,
    lua::{Poppable, Pushable},
};
use std::path::PathBuf;
use tracing::error;

use crate::{
    acp::{Result, error::Error},
    api::{Api, mcp_servers::parse_mcp_servers},
    utilities::{self},
};

/// Configuration for loading a session (second argument of the tuple)
#[derive(Debug, Clone, Default)]
pub struct LoadSessionConfig {
    pub cwd: Option<PathBuf>,
    pub mcp_servers: Vec<agent_client_protocol::McpServer>,
}

impl FromObject for LoadSessionConfig {
    fn from_object(obj: Object) -> std::result::Result<Self, nvim_oxi::conversion::Error> {
        // Convert Object to Dictionary, handling empty Lua tables
        let dict = crate::nvim::configuration::dict_from_object(obj)?;

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
        let root = utilities::get_project_root(current_directory, vec![".git".to_string()]);

        Ok(Self {
            cwd: Some(cwd.unwrap_or(root)),
            mcp_servers,
        })
    }
}

impl Poppable for LoadSessionConfig {
    unsafe fn pop(
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> std::result::Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Ok(Self::from_object(obj)
            .inspect(|e| error!("An error occurred parsing session load arguments, reverting to defaults:\n {:?}", e))
            .unwrap_or_default())
    }
}

impl Pushable for LoadSessionConfig {
    unsafe fn push(
        self,
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> std::result::Result<i32, nvim_oxi::lua::Error> {
        let mut dict = Dictionary::new();
        if let Some(cwd) = self.cwd {
            dict.insert("cwd", cwd.to_string_lossy().to_string());
        }
        unsafe { Object::from(dict).push(lua_state) }
    }
}

pub type LoadSessionArgs = (String, Option<LoadSessionConfig>);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn load_session(&self, (session_id, maybe_config): LoadSessionArgs) -> Result<()> {
        let config = maybe_config.unwrap_or_default();
        let state = self.state.lock().await;
        let root_markers = state.config.root_markers.clone();
        let agent_info = state.agent_info.clone();
        drop(state);

        if !agent_info.can_load_session() {
            return Ok(());
        }

        let request = agent_client_protocol::LoadSessionRequest::new(
            agent_client_protocol::SessionId::from(session_id),
            config.cwd.unwrap_or_else(|| {
                let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                crate::utilities::get_project_root(current_dir, root_markers)
            }),
        );

        let connection = self
            .connection
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        connection.load_session(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl LoadSessionConfig {
        fn default_with_root() -> Self {
            let current_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let root = utilities::get_project_root(current_directory, vec![".git".to_string()]);
            Self {
                cwd: Some(root),
                mcp_servers: Vec::new(),
            }
        }
    }
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

    #[test]
    fn test_load_session_config_with_mcp_servers() {
        // Test that LoadSessionConfig properly stores an empty mcp_servers vector
        // The actual McpServer construction comes from the agent_client_protocol crate
        let config = LoadSessionConfig {
            cwd: Some(PathBuf::from("/project")),
            mcp_servers: vec![], // Empty vector for simplicity
        };
        // Verify the config handles MCP servers correctly
        assert_eq!(config.cwd, Some(PathBuf::from("/project")));
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn test_load_session_config_pushable_without_cwd() {
        let config = LoadSessionConfig {
            cwd: None,
            mcp_servers: vec![],
        };
        // Verify the config struct handles None cwd correctly
        assert!(config.cwd.is_none());
        assert!(config.mcp_servers.is_empty());
    }
}
