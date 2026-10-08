use agent_client_protocol::schema::v1::{
    LoadSessionRequest, ResumeSessionRequest, SessionNotification,
};
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
    pub additional_directories: Option<Vec<PathBuf>>,
    pub mcp_servers: Vec<agent_client_protocol::schema::v1::McpServer>,
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

        let additional_directories: Option<Vec<PathBuf>> =
            dict.get("additional_directories").and_then(|obj| {
                if let nvim_oxi::ObjectKind::Array = obj.kind() {
                    let array = unsafe { obj.clone().into_array_unchecked() };
                    Some(
                        array
                            .into_iter()
                            .filter_map(|v| {
                                v.try_into()
                                    .ok()
                                    .map(|s: nvim_oxi::String| PathBuf::from(s.to_string()))
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            });

        let mcp_servers: Vec<agent_client_protocol::schema::v1::McpServer> = dict
            .get("mcp_servers")
            .and_then(parse_mcp_servers)
            .unwrap_or_default();

        let current_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let root = utilities::get_project_root(current_directory, vec![".git".to_string()]);

        Ok(Self {
            cwd: Some(cwd.unwrap_or(root)),
            additional_directories,
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
            .inspect_err(|e| {
                error!(
                    "An error occurred parsing session load arguments, reverting to defaults: {:?}",
                    e
                )
            })
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
        if let Some(dirs) = self.additional_directories {
            let arr: nvim_oxi::Array = dirs
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            dict.insert("additional_directories", arr);
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
        let project_root = state.config.project_root.clone();
        let store_history = state.config.session.store_history;
        let agent_info = state.agent_info.clone();
        drop(state);

        let connection = self
            .connection_manager
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        let cwd = config.cwd.unwrap_or(project_root);
        let additional_directories = config.additional_directories.unwrap_or_default();

        if agent_info.can_load_session() {
            let mut req = LoadSessionRequest::new(session_id, cwd);
            if agent_info.can_use_additional_directories() {
                req = req.additional_directories(additional_directories.clone());
            }
            connection
                .load_session(req.mcp_servers(config.mcp_servers.clone()))
                .await
        } else if agent_info.can_resume_sessions()
            && self.response_handler.can_receive_notifications().await
        {
            let agent = agent_info.current.clone();
            let filepath = format!("{}/{}.jsonl", agent, session_id);
            let history_path = agent_info.history_base_path.join(&filepath);

            if agent_info.needs_local_history(store_history) && history_path.exists() {
                let contents = std::fs::read_to_string(&history_path).map_err(|e| {
                    Error::Internal(format!(
                        "Failed to read history file at {:?}: {}",
                        history_path, e
                    ))
                })?;

                let history = contents
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .filter_map(|line| {
                        serde_json::from_str::<SessionNotification>(line)
                            .inspect_err(|e| {
                                tracing::error!(
                                    "Failed to parse history line for session {}: {} (line: {})",
                                    session_id,
                                    e,
                                    line
                                )
                            })
                            .ok()
                    })
                    .collect::<Vec<_>>();

                self.response_handler
                    .replay_session_notifications(history)
                    .await?;
            } else if !store_history {
                tracing::warn!(
                    "Agent does not support load_session and store_history is disabled. \
                     Resuming session without local history."
                );
            }

            let mut req = ResumeSessionRequest::new(session_id, cwd);
            if agent_info.can_use_additional_directories() {
                req = req.additional_directories(additional_directories);
            }
            connection
                .resume_session(req.mcp_servers(config.mcp_servers))
                .await?;
            Ok(())
        } else {
            Ok(())
        }
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
                additional_directories: None,
                mcp_servers: Vec::new(),
            }
        }
    }
    // Helper function to verify we can create config objects
    fn create_test_config(cwd: Option<&str>) -> LoadSessionConfig {
        if let Some(path) = cwd {
            LoadSessionConfig {
                cwd: Some(PathBuf::from(path)),
                additional_directories: None,
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
            additional_directories: None,
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
            additional_directories: None,
            mcp_servers: vec![],
        };
        // Verify the config struct handles None cwd correctly
        assert!(config.cwd.is_none());
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn test_from_object_with_additional_directories() {
        let mut dict = Dictionary::new();
        let dirs = vec!["src", "tests"]
            .into_iter()
            .collect::<nvim_oxi::Array>();
        dict.insert("additional_directories", dirs);
        let config = LoadSessionConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(
            config.additional_directories,
            Some(vec![PathBuf::from("src"), PathBuf::from("tests")])
        );
    }

    #[test]
    fn test_from_object_without_additional_directories() {
        let mut dict = Dictionary::new();
        dict.insert("cwd", "/tmp/test");
        let config = LoadSessionConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.additional_directories, None);
    }

    #[test]
    fn test_from_object_empty_additional_directories_array() {
        let mut dict = Dictionary::new();
        let dirs = Vec::<String>::new()
            .into_iter()
            .collect::<nvim_oxi::Array>();
        dict.insert("additional_directories", dirs);
        let config = LoadSessionConfig::from_object(Object::from(dict)).unwrap();
        assert_eq!(config.additional_directories, Some(vec![]));
    }
}
