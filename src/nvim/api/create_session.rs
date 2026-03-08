use agent_client_protocol::{
    Client, EnvVariable, McpServer, McpServerHttp, McpServerSse, McpServerStdio, NewSessionRequest,
};
use nvim_oxi::{
    Dictionary, Function, Object, ObjectKind,
    conversion::{Error, FromObject},
    lua::{Poppable, Pushable},
};
use std::{path::PathBuf, rc::Rc};
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{
    acp::connection::ConnectionManager, nvim::autocommands::ResponseHandler, utilities::project,
};

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

        // Updated key to "mcpServers" to match README
        let mcp_servers: Option<Vec<McpServer>> = dict
            .get("mcpServers")
            .and_then(|servers_obj| {
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
                                        .map(|s: nvim_oxi::String| {
                                            McpServerType::from(s.to_string())
                                        })
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

                                    let mut server =
                                        McpServerHttp::new(name.to_string(), url.to_string());

                                    // Handle headers
                                    if let Some(headers_obj) = server_dict.get("headers") {
                                        if let ObjectKind::Array = headers_obj.kind() {
                                            let headers_array = unsafe {
                                                headers_obj.clone().into_array_unchecked()
                                            };
                                            let headers: Vec<_> = headers_array
                                                .into_iter()
                                                .filter_map(|header_obj| {
                                                    let header_dict: Dictionary =
                                                        header_obj.try_into().ok()?;
                                                    // Expect single key-value pair per object in array: { "Key": "Value" }
                                                    header_dict.into_iter().next().map(|(k, v)| {
                                                        let k_str: nvim_oxi::String =
                                                            k.try_into().unwrap_or_default();
                                                        let v_str: nvim_oxi::String =
                                                            v.try_into().unwrap_or_default();
                                                        agent_client_protocol::HttpHeader::new(
                                                            k_str.to_string(),
                                                            v_str.to_string(),
                                                        )
                                                    })
                                                })
                                                .collect();
                                            server = server.headers(headers);
                                        }
                                    }
                                    Some(McpServer::Http(server))
                                }
                                McpServerType::Sse => {
                                    let url: nvim_oxi::String = server_dict
                                        .get("url")
                                        .or_else(|| server_dict.get("address"))?
                                        .clone()
                                        .try_into()
                                        .ok()?;

                                    let mut server =
                                        McpServerSse::new(name.to_string(), url.to_string());

                                    // Handle headers
                                    if let Some(headers_obj) = server_dict.get("headers") {
                                        if let ObjectKind::Array = headers_obj.kind() {
                                            let headers_array = unsafe {
                                                headers_obj.clone().into_array_unchecked()
                                            };
                                            let headers: Vec<_> = headers_array
                                                .into_iter()
                                                .filter_map(|header_obj| {
                                                    let header_dict: Dictionary =
                                                        header_obj.try_into().ok()?;
                                                    header_dict.into_iter().next().map(|(k, v)| {
                                                        let k_str: nvim_oxi::String =
                                                            k.try_into().unwrap_or_default();
                                                        let v_str: nvim_oxi::String =
                                                            v.try_into().unwrap_or_default();
                                                        agent_client_protocol::HttpHeader::new(
                                                            k_str.to_string(),
                                                            v_str.to_string(),
                                                        )
                                                    })
                                                })
                                                .collect();
                                            server = server.headers(headers);
                                        }
                                    }
                                    Some(McpServer::Sse(server))
                                }
                                McpServerType::Stdio => {
                                    let command: nvim_oxi::String =
                                        server_dict.get("command")?.clone().try_into().ok()?;
                                    let args: Vec<String> = server_dict
                                        .get("args")
                                        .map(|a| {
                                            if let ObjectKind::Array = a.kind() {
                                                unsafe { a.clone().into_array_unchecked() }
                                                    .into_iter()
                                                    .filter_map(|v| {
                                                        v.try_into().ok().map(
                                                            |s: nvim_oxi::String| s.to_string(),
                                                        )
                                                    })
                                                    .collect()
                                            } else {
                                                Vec::new()
                                            }
                                        })
                                        .unwrap_or_default();

                                    let env: Vec<EnvVariable> = server_dict
                                        .get("env")
                                        .and_then(|e| {
                                            // Expect array of objects: { { name = "VAR", value = "VAL" } }
                                            if let ObjectKind::Array = e.kind() {
                                                let array =
                                                    unsafe { e.clone().into_array_unchecked() };
                                                Some(
                                                    array
                                                        .into_iter()
                                                        .filter_map(|v| {
                                                            let dict: Dictionary =
                                                                v.try_into().ok()?;
                                                            let name: nvim_oxi::String = dict
                                                                .get("name")?
                                                                .clone()
                                                                .try_into()
                                                                .ok()?;
                                                            let value: nvim_oxi::String = dict
                                                                .get("value")?
                                                                .clone()
                                                                .try_into()
                                                                .ok()?;
                                                            Some(EnvVariable::new(
                                                                name.to_string(),
                                                                value.to_string(),
                                                            ))
                                                        })
                                                        .collect(),
                                                )
                                            } else {
                                                None
                                            }
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
                    eprintln!(
                        "DEBUG: mcpServers is not an array, kind: {:?}",
                        servers_obj.kind()
                    );
                    None
                }
            })
            .or_else(|| {
                eprintln!("DEBUG: mcpServers key not found or returned None from and_then");
                None
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
                                server_dict.insert("type", "http");
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
                                server_dict.insert("type", "sse");
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
                                server_dict.insert("type", "stdio");
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
                    dict.insert("mcpServers", array);
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
            let root = project::get_project_root(current_directory, vec![".git".to_string()]);
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

#[cfg(test)]
mod session_args_tests {
    use crate::api::McpServerType;
    use agent_client_protocol::McpServer;
    use nvim_oxi::{Dictionary, Object, conversion::FromObject};
    use std::path::PathBuf;

    use crate::api::CreateSessionArgs;

    // McpServerType Tests

    #[test]
    fn test_mcp_server_type_display_stdio() {
        assert_eq!(format!("{}", McpServerType::Stdio), "stdio");
    }

    #[test]
    fn test_mcp_server_type_display_http() {
        assert_eq!(format!("{}", McpServerType::Http), "http");
    }

    #[test]
    fn test_mcp_server_type_display_sse() {
        assert_eq!(format!("{}", McpServerType::Sse), "sse");
    }

    #[test]
    fn test_mcp_server_type_from_string_stdio() {
        let server_type = McpServerType::from("stdio".to_string());
        assert!(matches!(server_type, McpServerType::Stdio));
    }

    #[test]
    fn test_mcp_server_type_from_string_http() {
        let server_type = McpServerType::from("http".to_string());
        assert!(matches!(server_type, McpServerType::Http));
    }

    #[test]
    fn test_mcp_server_type_from_string_sse() {
        let server_type = McpServerType::from("sse".to_string());
        assert!(matches!(server_type, McpServerType::Sse));
    }

    #[test]
    fn test_mcp_server_type_from_string_unknown_defaults_to_stdio() {
        let server_type = McpServerType::from("unknown".to_string());
        assert!(matches!(server_type, McpServerType::Stdio));
    }

    #[test]
    fn test_from_object_default() {
        let obj = Object::nil();
        let args = CreateSessionArgs::from_object(obj).unwrap();
        match args {
            CreateSessionArgs::Default => assert!(true),
            _ => panic!("Expected Default variant"),
        }
    }

    // Helper for Stdio Default
    fn create_stdio_default_args() -> CreateSessionArgs {
        let mut dict = Dictionary::new();
        let mut server = Dictionary::new();
        server.insert("name", "test-server");
        server.insert("command", "test-cmd");

        let servers = vec![server].into_iter().collect::<nvim_oxi::Array>();
        dict.insert("mcpServers", servers);

        let obj = Object::from(dict);
        CreateSessionArgs::from_object(obj).unwrap()
    }

    #[test]
    fn test_stdio_defaults_is_configuration() {
        let args = create_stdio_default_args();
        match args {
            CreateSessionArgs::Configuration { .. } => assert!(true),
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_stdio_defaults_server_count() {
        let args = create_stdio_default_args();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                assert_eq!(mcp_servers.unwrap().len(), 1);
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_stdio_defaults_name() {
        let args = create_stdio_default_args();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Stdio(s) => assert_eq!(s.name, "test-server"),
                    _ => panic!("Expected Stdio server"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_stdio_defaults_command() {
        let args = create_stdio_default_args();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Stdio(s) => assert_eq!(s.command, PathBuf::from("test-cmd")),
                    _ => panic!("Expected Stdio server"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    // Helper for Explicit Types
    fn create_explicit_args() -> CreateSessionArgs {
        let mut dict = Dictionary::new();
        let mut sse_server = Dictionary::new();
        sse_server.insert("type", "sse");
        sse_server.insert("name", "sse-srv");
        sse_server.insert("url", "http://localhost:8080");

        let mut http_server = Dictionary::new();
        http_server.insert("type", "http");
        http_server.insert("name", "http-srv");
        http_server.insert("url", "http://remote.com");

        let mut stdio_server = Dictionary::new();
        stdio_server.insert("type", "stdio");
        stdio_server.insert("name", "stdio-srv");
        stdio_server.insert("command", "bin");
        let args_arr = vec!["arg1", "arg2"]
            .into_iter()
            .collect::<nvim_oxi::Array>();
        stdio_server.insert("args", args_arr);

        let servers = vec![sse_server, http_server, stdio_server]
            .into_iter()
            .collect::<nvim_oxi::Array>();
        dict.insert("mcpServers", servers);

        let obj = Object::from(dict);
        CreateSessionArgs::from_object(obj).unwrap()
    }

    #[test]
    fn test_explicit_sse_name() {
        let args = create_explicit_args();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Sse(s) => assert_eq!(s.name, "sse-srv"),
                    _ => panic!("Expected SSE server at index 0"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_explicit_sse_url() {
        let args = create_explicit_args();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Sse(s) => assert_eq!(s.url, "http://localhost:8080"),
                    _ => panic!("Expected SSE server at index 0"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_explicit_http_name() {
        let args = create_explicit_args();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[1] {
                    McpServer::Http(h) => assert_eq!(h.name, "http-srv"),
                    _ => panic!("Expected HTTP server at index 1"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_explicit_http_url() {
        let args = create_explicit_args();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[1] {
                    McpServer::Http(h) => assert_eq!(h.url, "http://remote.com"),
                    _ => panic!("Expected HTTP server at index 1"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_explicit_stdio_name() {
        let args = create_explicit_args();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[2] {
                    McpServer::Stdio(s) => assert_eq!(s.name, "stdio-srv"),
                    _ => panic!("Expected Stdio server at index 2"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_explicit_stdio_command() {
        let args = create_explicit_args();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[2] {
                    McpServer::Stdio(s) => assert_eq!(s.command, PathBuf::from("bin")),
                    _ => panic!("Expected Stdio server at index 2"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_readme_example_compliance() {
        // Imitate the Lua table structure from README
        // hermes.createSession({
        //   mcpServers = { ... }
        // })

        let mut dict = Dictionary::new();

        let mut http_server = Dictionary::new();
        http_server.insert("type", "http");
        http_server.insert("name", "readme-http");
        http_server.insert("url", "http://example.com");

        // headers = { { ["Content-Type"] = "application/json" }, { headerName = "header value" } }
        let mut header1 = Dictionary::new();
        header1.insert("Content-Type", "application/json");
        let mut header2 = Dictionary::new();
        header2.insert("headerName", "header value");

        let headers = vec![header1, header2]
            .into_iter()
            .collect::<nvim_oxi::Array>();
        http_server.insert("headers", headers);

        let mut stdio_server = Dictionary::new();
        stdio_server.insert("type", "stdio");
        stdio_server.insert("name", "readme-stdio");
        stdio_server.insert("command", "cat");

        // env = { { name = "VAR", value = "VAL" } }
        let mut env_var = Dictionary::new();
        env_var.insert("name", "VAR");
        env_var.insert("value", "VAL");
        let env = vec![env_var].into_iter().collect::<nvim_oxi::Array>();
        stdio_server.insert("env", env);

        let servers = vec![http_server, stdio_server]
            .into_iter()
            .collect::<nvim_oxi::Array>();

        dict.insert("mcpServers", servers);

        let obj = Object::from(dict);
        let args = CreateSessionArgs::from_object(obj).unwrap();

        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                assert_eq!(servers.len(), 2);

                // Check HTTP server
                match &servers[0] {
                    McpServer::Http(h) => {
                        assert_eq!(h.name, "readme-http");
                        assert_eq!(h.headers.len(), 2);

                        // Check first header: Content-Type: application/json
                        let h1 = h.headers.iter().find(|h| h.name == "Content-Type").unwrap();
                        assert_eq!(h1.value, "application/json");

                        // Check second header: headerName: header value
                        let h2 = h.headers.iter().find(|h| h.name == "headerName").unwrap();
                        assert_eq!(h2.value, "header value");
                    }
                    _ => panic!("Expected HTTP server"),
                }

                // Check Stdio server
                match &servers[1] {
                    McpServer::Stdio(s) => {
                        assert_eq!(s.name, "readme-stdio");
                        assert_eq!(s.env.len(), 1);
                        assert_eq!(s.env[0].name, "VAR");
                        assert_eq!(s.env[0].value, "VAL");
                    }
                    _ => panic!("Expected Stdio server"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    // Round-trip helpers: build the dictionary that `push` now produces (with `type` field)
    // and verify `from_object` correctly restores each variant.
    fn round_trip_http_dict() -> CreateSessionArgs {
        let mut dict = Dictionary::new();
        let mut server_dict = Dictionary::new();
        server_dict.insert("type", "http");
        server_dict.insert("name", "http-srv");
        server_dict.insert("url", "http://example.com");
        let servers = vec![server_dict].into_iter().collect::<nvim_oxi::Array>();
        dict.insert("mcpServers", servers);
        CreateSessionArgs::from_object(Object::from(dict)).unwrap()
    }

    fn round_trip_sse_dict() -> CreateSessionArgs {
        let mut dict = Dictionary::new();
        let mut server_dict = Dictionary::new();
        server_dict.insert("type", "sse");
        server_dict.insert("name", "sse-srv");
        server_dict.insert("url", "http://sse.example.com");
        let servers = vec![server_dict].into_iter().collect::<nvim_oxi::Array>();
        dict.insert("mcpServers", servers);
        CreateSessionArgs::from_object(Object::from(dict)).unwrap()
    }

    fn round_trip_stdio_dict() -> CreateSessionArgs {
        let mut dict = Dictionary::new();
        let mut server_dict = Dictionary::new();
        server_dict.insert("type", "stdio");
        server_dict.insert("name", "stdio-srv");
        server_dict.insert("command", "my-cmd");
        let servers = vec![server_dict].into_iter().collect::<nvim_oxi::Array>();
        dict.insert("mcpServers", servers);
        CreateSessionArgs::from_object(Object::from(dict)).unwrap()
    }

    #[test]
    fn test_round_trip_http_name() {
        let args = round_trip_http_dict();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Http(h) => assert_eq!(h.name, "http-srv"),
                    _ => panic!("Expected Http server after round-trip"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_round_trip_http_url() {
        let args = round_trip_http_dict();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Http(h) => assert_eq!(h.url, "http://example.com"),
                    _ => panic!("Expected Http server after round-trip"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_round_trip_sse_name() {
        let args = round_trip_sse_dict();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Sse(s) => assert_eq!(s.name, "sse-srv"),
                    _ => panic!("Expected Sse server after round-trip"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_round_trip_sse_url() {
        let args = round_trip_sse_dict();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Sse(s) => assert_eq!(s.url, "http://sse.example.com"),
                    _ => panic!("Expected Sse server after round-trip"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_round_trip_stdio_name() {
        let args = round_trip_stdio_dict();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Stdio(s) => assert_eq!(s.name, "stdio-srv"),
                    _ => panic!("Expected Stdio server after round-trip"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }

    #[test]
    fn test_round_trip_stdio_command() {
        let args = round_trip_stdio_dict();
        match args {
            CreateSessionArgs::Configuration { mcp_servers, .. } => {
                let servers = mcp_servers.unwrap();
                match &servers[0] {
                    McpServer::Stdio(s) => assert_eq!(s.command, PathBuf::from("my-cmd")),
                    _ => panic!("Expected Stdio server after round-trip"),
                }
            }
            _ => panic!("Expected Configuration variant"),
        }
    }
}
