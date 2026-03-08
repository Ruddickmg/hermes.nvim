use agent_client_protocol::{
    Client, EnvVariable, McpServer, McpServerHttp, McpServerSse, McpServerStdio, NewSessionRequest,
};
use nvim_oxi::{
    Dictionary, Function, Object, ObjectKind,
    conversion::{Error, FromObject},
    lua::{Poppable, Pushable},
};
use std::{collections::HashMap, path::PathBuf, rc::Rc};
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{acp::connection::ConnectionManager, nvim::autocommands::ResponseHandler};

pub fn get_project_root(current_directory: PathBuf, root_markers: Vec<String>) -> PathBuf {
    let markers: HashMap<String, bool> =
        root_markers.iter().map(|m| (m.to_string(), true)).collect();
    let buf = current_directory.ancestors().find(|dir| {
        dir.read_dir()
            .map(|mut files| {
                files.any(|file| {
                    file.map(|details| {
                        details
                            .file_name()
                            .into_string()
                            .map(|file_name| markers.contains_key(&file_name))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    });
    buf.map(PathBuf::from).unwrap_or(current_directory)
}

#[derive(Debug, Clone)]
pub enum McpServerType {
    Stdio,
    Http,
    Sse,
}

impl std::fmt::Display for McpServerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpServerType::Stdio => write!(f, "stdio"),
            McpServerType::Http => write!(f, "http"),
            McpServerType::Sse => write!(f, "sse"),
        }
    }
}

impl From<String> for McpServerType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "http" => McpServerType::Http,
            "sse" => McpServerType::Sse,
            _ => McpServerType::Stdio,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CreateSessionArgs {
    Default,
    Configuration {
        cwd: Option<PathBuf>,
        mcp_servers: Option<Vec<McpServer>>,
    },
}

impl FromObject for CreateSessionArgs {
    fn from_object(obj: Object) -> Result<Self, Error> {
        if obj.is_nil() {
            return Ok(Self::Default);
        }

        let dict: Dictionary = obj.try_into()?;

        let cwd: Option<PathBuf> = dict.get("cwd").and_then(|obj| {
            obj.clone()
                .try_into()
                .ok()
                .map(|s: nvim_oxi::String| PathBuf::from(s.to_string()))
        });

        let mcp_servers: Option<Vec<McpServer>> = dict.get("mcp_servers").and_then(|servers_obj| {
            if let ObjectKind::Array = servers_obj.kind() {
                let array = unsafe { servers_obj.clone().into_array_unchecked() };

                let servers: Vec<McpServer> = array
                    .into_iter()
                    .filter_map(|server_obj| {
                        let server_dict: Dictionary = server_obj.try_into().ok()?;
                        let name: nvim_oxi::String =
                            server_dict.get("name")?.clone().try_into().ok()?;
                        let type_: McpServerType = server_dict
                            .get("type")
                            .map(|t| {
                                t.clone()
                                    .try_into()
                                    .map(|s: nvim_oxi::String| McpServerType::from(s.to_string()))
                                    .unwrap_or(McpServerType::Stdio)
                            })
                            .unwrap_or(McpServerType::Stdio);

                        match type_ {
                            McpServerType::Http => {
                                let url: nvim_oxi::String = server_dict
                                    .get("url")
                                    .or_else(|| server_dict.get("address"))?
                                    .clone()
                                    .try_into()
                                    .ok()?;
                                Some(McpServer::Http(McpServerHttp::new(
                                    name.to_string(),
                                    url.to_string(),
                                )))
                            }
                            McpServerType::Sse => {
                                let url: nvim_oxi::String = server_dict
                                    .get("url")
                                    .or_else(|| server_dict.get("address"))?
                                    .clone()
                                    .try_into()
                                    .ok()?;
                                Some(McpServer::Sse(McpServerSse::new(
                                    name.to_string(),
                                    url.to_string(),
                                )))
                            }
                            McpServerType::Stdio => {
                                let command: nvim_oxi::String =
                                    server_dict.get("command")?.clone().try_into().ok()?;
                                let args: Vec<String> = server_dict
                                    .get("args")
                                    .map(|a| {
                                        let arr: nvim_oxi::Array =
                                            unsafe { a.clone().into_array_unchecked() };
                                        arr.into_iter()
                                            .filter_map(|v| {
                                                v.try_into()
                                                    .ok()
                                                    .map(|s: nvim_oxi::String| s.to_string())
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();

                                let env: Vec<EnvVariable> = server_dict
                                    .get("env")
                                    .and_then(|e| {
                                        let dict: Dictionary = e.clone().try_into().ok()?;
                                        let vars = dict
                                            .into_iter()
                                            .filter_map(|(k, v)| {
                                                let k: nvim_oxi::String = k.try_into().ok()?;
                                                let v: nvim_oxi::String = v.try_into().ok()?;
                                                Some(EnvVariable::new(k.to_string(), v.to_string()))
                                            })
                                            .collect();
                                        Some(vars)
                                    })
                                    .unwrap_or_default();

                                Some(McpServer::Stdio(
                                    McpServerStdio::new(
                                        name.to_string(),
                                        PathBuf::from(command.to_string()),
                                    )
                                    .args(args)
                                    .env(env),
                                ))
                            }
                        }
                    })
                    .collect();

                Some(servers)
            } else {
                None
            }
        });

        Ok(Self::Configuration { cwd, mcp_servers })
    }
}

impl Poppable for CreateSessionArgs {
    unsafe fn pop(lua_state: *mut nvim_oxi::lua::ffi::State) -> Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Self::from_object(obj).map_err(|e| nvim_oxi::lua::Error::RuntimeError(e.to_string()))
    }
}

impl Pushable for CreateSessionArgs {
    unsafe fn push(
        self,
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> Result<i32, nvim_oxi::lua::Error> {
        let obj = match self {
            Self::Default => Object::nil(),
            Self::Configuration { cwd, mcp_servers } => {
                let mut dict = Dictionary::new();
                if let Some(cwd) = cwd {
                    dict.insert("cwd", cwd.to_string_lossy().to_string());
                }
                if let Some(servers) = mcp_servers {
                    let array = servers
                        .into_iter()
                        .map(|server| match server {
                            McpServer::Http(http) => {
                                let mut server_dict = Dictionary::new();
                                server_dict.insert("name", http.name);
                                server_dict.insert("url", http.url);
                                let arr: nvim_oxi::Array = http
                                    .headers
                                    .into_iter()
                                    .map(|header| {
                                        let mut header_dict = Dictionary::new();
                                        header_dict.insert("name", header.name);
                                        header_dict.insert("value", header.value);
                                        header_dict
                                    })
                                    .collect();
                                server_dict.insert("headers", arr);
                                Ok(server_dict)
                            }
                            McpServer::Sse(sse) => {
                                let mut server_dict = Dictionary::new();
                                server_dict.insert("name", sse.name);
                                server_dict.insert("url", sse.url);
                                let arr: nvim_oxi::Array = sse
                                    .headers
                                    .into_iter()
                                    .map(|header| {
                                        let mut header_dict = Dictionary::new();
                                        header_dict.insert("name", header.name);
                                        header_dict.insert("value", header.value);
                                        header_dict
                                    })
                                    .collect();
                                server_dict.insert("headers", arr);
                                Ok(server_dict)
                            }
                            McpServer::Stdio(stdio) => {
                                let mut server_dict = Dictionary::new();
                                server_dict.insert("name", stdio.name);
                                server_dict.insert("command", stdio.command.to_str());
                                server_dict.insert(
                                    "args",
                                    stdio.args.into_iter().collect::<nvim_oxi::Array>(),
                                );
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
                                        .collect::<nvim_oxi::Array>(),
                                );
                                Ok(server_dict)
                            }
                            _ => Err(nvim_oxi::lua::Error::RuntimeError(format!(
                                "Unsupported MCP server type: {:#?}",
                                server
                            ))),
                        })
                        .collect::<Result<nvim_oxi::Array, nvim_oxi::lua::Error>>()?;
                    dict.insert("mcp_servers", array);
                }
                Object::from(dict)
            }
        };
        Ok(unsafe { obj.push(lua_state)? })
    }
}

#[instrument(level = "trace", skip_all)]
pub fn create_session<H: Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
    let function: Function<CreateSessionArgs, Result<(), nvim_oxi::lua::Error>> =
        Function::from_fn(move |session: CreateSessionArgs| {
            debug!("createSession function called with: {:#?}", session);
            let current_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let root = get_project_root(current_directory, vec![".git".to_string()]);
            let request = match session {
                CreateSessionArgs::Default => NewSessionRequest::new(root),
                CreateSessionArgs::Configuration { cwd, mcp_servers } => {
                    NewSessionRequest::new(cwd.unwrap_or(root))
                        .mcp_servers(mcp_servers.unwrap_or_default())
                }
            };
            connection
                .blocking_lock()
                .get_current_connection()
                .ok_or_else(|| {
                    nvim_oxi::lua::Error::RuntimeError(
                        "No connection found, call the connect function".to_string(),
                    )
                })?
                .create_session(request)?;
            Ok(())
        });
    function.into()
}
