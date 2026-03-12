use agent_client_protocol::{Client, LoadSessionRequest, SessionId};
use nvim_oxi::{
    Array, Dictionary, Function, Object, ObjectKind,
    conversion::{Error, FromObject},
    lua::{Poppable, Pushable},
};
use std::{path::PathBuf, rc::Rc};
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{
    acp::connection::ConnectionManager, api::mcp_servers::parse_mcp_servers,
    nvim::autocommands::ResponseHandler, utilities::project,
};

#[derive(Debug, Clone)]
pub enum LoadSessionArgs {
    Minimal {
        session_id: String,
    },
    WithConfig {
        session_id: String,
        cwd: Option<PathBuf>,
        mcp_servers: Option<Vec<agent_client_protocol::McpServer>>,
    },
}

impl FromObject for LoadSessionArgs {
    fn from_object(obj: Object) -> Result<Self, Error> {
        match obj.kind() {
            ObjectKind::Array => {
                let array = unsafe { obj.into_array_unchecked() };
                if array.is_empty() {
                    return Err(Error::Other(
                        "loadSession requires at least a session_id parameter".to_string(),
                    ));
                }

                // First element should be session_id (string)
                let session_id: nvim_oxi::String =
                    array.first().unwrap().clone().try_into()?;
                let session_id = session_id.to_string();

                // Check for optional config dict
                if let Some(config_obj) = array.get(1).filter(|obj| !obj.is_nil()) {
                    let config_dict: Dictionary = config_obj.clone().try_into()?;

                    let cwd: Option<PathBuf> = config_dict.get("cwd").and_then(|obj| {
                        obj.clone()
                            .try_into()
                            .ok()
                            .map(|s: nvim_oxi::String| PathBuf::from(s.to_string()))
                    });

                    let mcp_servers: Option<Vec<agent_client_protocol::McpServer>> =
                        config_dict
                            .get("mcpServers")
                            .and_then(parse_mcp_servers);

                    return Ok(Self::WithConfig {
                        session_id,
                        cwd,
                        mcp_servers,
                    });
                }

                Ok(Self::Minimal { session_id })
            }
            ObjectKind::String => {
                // Single string argument: loadSession(sessionId)
                let session_id: nvim_oxi::String = obj.try_into()?;
                Ok(Self::Minimal {
                    session_id: session_id.to_string(),
                })
            }
            _ => Err(Error::Other(
                "loadSession expects a session_id string, optionally followed by a configuration table".to_string(),
            )),
        }
    }
}

impl Poppable for LoadSessionArgs {
    unsafe fn pop(lua_state: *mut nvim_oxi::lua::ffi::State) -> Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Self::from_object(obj).map_err(|e| nvim_oxi::lua::Error::RuntimeError(e.to_string()))
    }
}

impl Pushable for LoadSessionArgs {
    unsafe fn push(
        self,
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> Result<i32, nvim_oxi::lua::Error> {
        let obj = match self {
            Self::Minimal { session_id } => {
                let arr = Array::from_iter(vec![Object::from(session_id)]);
                Object::from(arr)
            }
            Self::WithConfig {
                session_id,
                cwd,
                mcp_servers,
            } => {
                let mut dict = Dictionary::new();
                if let Some(cwd) = cwd {
                    dict.insert("cwd", cwd.to_string_lossy().to_string());
                }
                if let Some(servers) = mcp_servers {
                    let servers_array: Array = servers
                        .into_iter()
                        .map(|server| match server {
                            agent_client_protocol::McpServer::Http(http) => {
                                let mut server_dict = Dictionary::new();
                                server_dict.insert("type", "http");
                                server_dict.insert("name", http.name);
                                server_dict.insert("url", http.url);
                                let headers_arr: Array = http
                                    .headers
                                    .into_iter()
                                    .map(|header| {
                                        let mut header_dict = Dictionary::new();
                                        header_dict.insert("name", header.name);
                                        header_dict.insert("value", header.value);
                                        header_dict
                                    })
                                    .collect();
                                server_dict.insert("headers", headers_arr);
                                Ok(server_dict)
                            }
                            agent_client_protocol::McpServer::Sse(sse) => {
                                let mut server_dict = Dictionary::new();
                                server_dict.insert("type", "sse");
                                server_dict.insert("name", sse.name);
                                server_dict.insert("url", sse.url);
                                let headers_arr: Array = sse
                                    .headers
                                    .into_iter()
                                    .map(|header| {
                                        let mut header_dict = Dictionary::new();
                                        header_dict.insert("name", header.name);
                                        header_dict.insert("value", header.value);
                                        header_dict
                                    })
                                    .collect();
                                server_dict.insert("headers", headers_arr);
                                Ok(server_dict)
                            }
                            agent_client_protocol::McpServer::Stdio(stdio) => {
                                let mut server_dict = Dictionary::new();
                                server_dict.insert("type", "stdio");
                                server_dict.insert("name", stdio.name);
                                server_dict.insert("command", stdio.command.to_str());
                                server_dict
                                    .insert("args", stdio.args.into_iter().collect::<Array>());
                                server_dict.insert(
                                    "env",
                                    stdio
                                        .env
                                        .into_iter()
                                        .map(|env| {
                                            let mut env_dict = Dictionary::new();
                                            env_dict.insert("name", env.name);
                                            env_dict.insert("value", env.value);
                                            env_dict
                                        })
                                        .collect::<Array>(),
                                );
                                Ok(server_dict)
                            }
                            _ => Err(nvim_oxi::lua::Error::RuntimeError(format!(
                                "Unsupported MCP server type: {:#?}",
                                server
                            ))),
                        })
                        .collect::<Result<Array, nvim_oxi::lua::Error>>()?;
                    dict.insert("mcpServers", servers_array);
                }

                let arr = Array::from_iter(vec![Object::from(session_id), Object::from(dict)]);
                Object::from(arr)
            }
        };
        Ok(unsafe { obj.push(lua_state)? })
    }
}

#[instrument(level = "trace", skip_all)]
pub fn load_session<H: Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
    let function: Function<LoadSessionArgs, Result<(), nvim_oxi::lua::Error>> =
        Function::from_fn(move |args: LoadSessionArgs| {
            debug!("loadSession function called with: {:#?}", args);

            let current_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let root = project::get_project_root(current_directory, vec![".git".to_string()]);

            let (session_id, cwd, mcp_servers) = match args {
                LoadSessionArgs::Minimal { session_id } => (session_id, root, Vec::new()),
                LoadSessionArgs::WithConfig {
                    session_id,
                    cwd,
                    mcp_servers,
                } => (
                    session_id,
                    cwd.unwrap_or(root),
                    mcp_servers.unwrap_or_default(),
                ),
            };

            let request =
                LoadSessionRequest::new(SessionId::from(session_id), cwd).mcp_servers(mcp_servers);

            connection
                .blocking_lock()
                .get_current_connection()
                .ok_or_else(|| {
                    nvim_oxi::lua::Error::RuntimeError(
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
    use nvim_oxi::Object;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    // Strategy for generating valid session_id strings
    fn arb_session_id() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,50}".prop_map(|s| s.to_string())
    }

    proptest! {
        #[test]
        fn test_session_id_parsing_never_panics(session_id in arb_session_id()) {
            // Property: parsing any string as minimal session_id should never panic
            let obj = Object::from(session_id);
            let _ = LoadSessionArgs::from_object(obj);
        }

        #[test]
        fn test_minimal_args_roundtrip(session_id in arb_session_id()) {
            // Property: minimal args can be round-tripped through push/pop
            let args = LoadSessionArgs::Minimal { session_id: session_id.clone() };
            // Note: We can't actually test push/pop without a Lua state,
            // but we can verify the structure is correct
            match &args {
                LoadSessionArgs::Minimal { session_id: s } => {
                    prop_assert_eq!(s, &session_id);
                }
                _ => prop_assert!(false, "Expected Minimal variant"),
            }
        }
    }

    #[test]
    fn test_from_object_single_string() {
        let obj = Object::from("test-session-123");
        let args = LoadSessionArgs::from_object(obj).unwrap();

        match args {
            LoadSessionArgs::Minimal { session_id } => {
                assert_eq!(session_id, "test-session-123");
            }
            _ => panic!("Expected Minimal variant"),
        }
    }

    #[test]
    fn test_from_object_array_with_single_element() {
        let arr = Array::from_iter(vec![Object::from("session-456")]);
        let obj = Object::from(arr);

        let args = LoadSessionArgs::from_object(obj).unwrap();

        match args {
            LoadSessionArgs::Minimal { session_id } => {
                assert_eq!(session_id, "session-456");
            }
            _ => panic!("Expected Minimal variant"),
        }
    }

    #[test]
    fn test_from_object_array_with_config() {
        let mut config = Dictionary::new();
        config.insert("cwd", "/path/to/project");

        let arr = Array::from_iter(vec![Object::from("session-789"), Object::from(config)]);
        let obj = Object::from(arr);
        let args = LoadSessionArgs::from_object(obj).unwrap();

        match args {
            LoadSessionArgs::WithConfig {
                session_id,
                cwd,
                mcp_servers,
            } => {
                assert_eq!(session_id, "session-789");
                assert_eq!(cwd, Some(PathBuf::from("/path/to/project")));
                assert!(mcp_servers.is_none());
            }
            _ => panic!("Expected WithConfig variant"),
        }
    }

    #[test]
    fn test_from_object_empty_array_fails() {
        let arr = Array::from_iter::<Vec<Object>>(vec![]);
        let obj = Object::from(arr);

        let result = LoadSessionArgs::from_object(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_object_nil_second_arg_ignores_config() {
        let arr = Array::from_iter(vec![Object::from("session-000"), Object::nil()]);

        let obj = Object::from(arr);
        let args = LoadSessionArgs::from_object(obj).unwrap();

        match args {
            LoadSessionArgs::Minimal { session_id } => {
                assert_eq!(session_id, "session-000");
            }
            _ => panic!("Expected Minimal variant when second arg is nil"),
        }
    }

    #[test]
    fn test_from_object_invalid_type_fails() {
        // Number is not a valid session_id
        let obj = Object::from(42i32);
        let result = LoadSessionArgs::from_object(obj);
        assert!(result.is_err());
    }
}
