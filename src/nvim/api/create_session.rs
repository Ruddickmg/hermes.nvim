use agent_client_protocol::{McpServer, NewSessionRequest};
use nvim_oxi::{
    Dictionary, Function, Object,
    conversion::{Error, FromObject},
    lua::{Poppable, Pushable},
};
use tokio::sync::Mutex;
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};
use tracing::{debug, instrument};

use crate::{
    PluginState, acp::connection::ConnectionManager, api::mcp_servers::parse_mcp_servers, utilities,
};

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
        let mcp_servers: Option<Vec<McpServer>> =
            dict.get("mcpServers").and_then(parse_mcp_servers);

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
pub fn create_session(
    connection: Rc<RefCell<ConnectionManager>>,
    state: Arc<Mutex<PluginState>>,
) -> Object {
    let function: Function<CreateSessionArgs, Result<(), nvim_oxi::lua::Error>> =
        Function::from_fn(move |session: CreateSessionArgs| {
            debug!("createSession function called with: {:#?}", session);
            let state = state.blocking_lock();
            let root_markers = state.config.root_markers.clone();
            drop(state);
            let current_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let root = utilities::get_project_root(current_directory, root_markers);
            let request = match session {
                CreateSessionArgs::Default => NewSessionRequest::new(root),
                CreateSessionArgs::Configuration { cwd, mcp_servers } => {
                    NewSessionRequest::new(cwd.unwrap_or(root))
                        .mcp_servers(mcp_servers.unwrap_or_default())
                }
            };
            connection
                .borrow()
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
    use crate::api::mcp_servers::McpServerType;
    use agent_client_protocol::McpServer;
    use nvim_oxi::{Dictionary, Object, conversion::FromObject};
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    use std::path::PathBuf;

    use crate::api::CreateSessionArgs;

    // Strategy for generating MCP server type strings
    fn arb_mcp_server_type_string() -> impl Strategy<Value = String> {
        prop_oneof!(
            Just("stdio".to_string()),
            Just("http".to_string()),
            Just("sse".to_string()),
            Just("unknown".to_string()),
            "[a-z]+".prop_map(|s| s.to_string())
        )
    }

    proptest! {
        #[test]
        fn test_mcp_server_type_from_str_never_panics(server_type_str in arb_mcp_server_type_string()) {
            // Property: converting any string to McpServerType should never panic
            let _ = McpServerType::from(server_type_str);
        }

        #[test]
        fn test_mcp_server_type_known_values_parsed_correctly(
            server_type in prop_oneof!(
                Just("stdio"),
                Just("http"),
                Just("sse")
            )
        ) {
            let result = McpServerType::from(server_type.to_string());
            match server_type {
                "stdio" => prop_assert!(matches!(result, McpServerType::Stdio)),
                "http" => prop_assert!(matches!(result, McpServerType::Http)),
                "sse" => prop_assert!(matches!(result, McpServerType::Sse)),
                _ => unreachable!(),
            }
        }

        #[test]
        fn test_mcp_server_type_unknown_defaults_to_stdio(unknown in "[0-9a-zA-Z_]*") {
            // Property: Unknown types should default to Stdio
            let result = McpServerType::from(unknown.to_string());
            if !matches!(unknown.as_str(), "http" | "sse") {
                prop_assert!(matches!(result, McpServerType::Stdio), "Unknown type should default to Stdio");
            }
        }
    }

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
            CreateSessionArgs::Default => (),
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
            CreateSessionArgs::Configuration { .. } => (),
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
