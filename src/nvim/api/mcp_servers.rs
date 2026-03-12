use agent_client_protocol::{
    EnvVariable, HttpHeader, McpServer, McpServerHttp, McpServerSse, McpServerStdio,
};
use nvim_oxi::{Dictionary, Object, ObjectKind};
use std::path::PathBuf;
use tracing::debug;

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

/// Parse MCP servers from a Lua array object
pub fn parse_mcp_servers(servers_obj: &Object) -> Option<Vec<McpServer>> {
    if let ObjectKind::Array = servers_obj.kind() {
        let array = unsafe { servers_obj.clone().into_array_unchecked() };

        let servers: Vec<McpServer> = array
            .into_iter()
            .filter_map(|server_obj| {
                let server_dict: Dictionary = server_obj.try_into().ok()?;
                let name: nvim_oxi::String = server_dict.get("name")?.clone().try_into().ok()?;
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
                    McpServerType::Http => parse_http_server(&server_dict, name),
                    McpServerType::Sse => parse_sse_server(&server_dict, name),
                    McpServerType::Stdio => parse_stdio_server(&server_dict, name),
                }
            })
            .collect();

        Some(servers)
    } else {
        debug!("mcpServers is not an array, kind: {:?}", servers_obj.kind());
        None
    }
}

fn parse_http_headers(server_dict: &Dictionary) -> Vec<HttpHeader> {
    if let Some(headers_obj) = server_dict.get("headers")
        && let ObjectKind::Array = headers_obj.kind()
    {
        let headers_array = unsafe { headers_obj.clone().into_array_unchecked() };
        return headers_array
            .into_iter()
            .filter_map(|header_obj| {
                let header_dict: Dictionary = header_obj.try_into().ok()?;
                // Expect single key-value pair per object in array: { "Key": "Value" }
                header_dict.into_iter().next().map(|(k, v)| {
                    let k_str = k;
                    let v_str: nvim_oxi::String = v.try_into().unwrap_or_default();
                    HttpHeader::new(k_str.to_string(), v_str.to_string())
                })
            })
            .collect();
    }
    Vec::new()
}

fn parse_http_server(server_dict: &Dictionary, name: nvim_oxi::String) -> Option<McpServer> {
    let url: nvim_oxi::String = server_dict
        .get("url")
        .or_else(|| server_dict.get("address"))?
        .clone()
        .try_into()
        .ok()?;

    let mut server = McpServerHttp::new(name.to_string(), url.to_string());
    let headers = parse_http_headers(server_dict);
    if !headers.is_empty() {
        server = server.headers(headers);
    }
    Some(McpServer::Http(server))
}

fn parse_sse_server(server_dict: &Dictionary, name: nvim_oxi::String) -> Option<McpServer> {
    let url: nvim_oxi::String = server_dict
        .get("url")
        .or_else(|| server_dict.get("address"))?
        .clone()
        .try_into()
        .ok()?;

    let mut server = McpServerSse::new(name.to_string(), url.to_string());
    let headers = parse_http_headers(server_dict);
    if !headers.is_empty() {
        server = server.headers(headers);
    }
    Some(McpServer::Sse(server))
}

fn parse_stdio_server(server_dict: &Dictionary, name: nvim_oxi::String) -> Option<McpServer> {
    let command: nvim_oxi::String = server_dict.get("command")?.clone().try_into().ok()?;
    let args: Vec<String> = server_dict
        .get("args")
        .map(|a| {
            if let ObjectKind::Array = a.kind() {
                unsafe { a.clone().into_array_unchecked() }
                    .into_iter()
                    .filter_map(|v| v.try_into().ok().map(|s: nvim_oxi::String| s.to_string()))
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
                let array = unsafe { e.clone().into_array_unchecked() };
                Some(
                    array
                        .into_iter()
                        .filter_map(|v| {
                            let dict: Dictionary = v.try_into().ok()?;
                            let name: nvim_oxi::String =
                                dict.get("name")?.clone().try_into().ok()?;
                            let value: nvim_oxi::String =
                                dict.get("value")?.clone().try_into().ok()?;
                            Some(EnvVariable::new(name.to_string(), value.to_string()))
                        })
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default();

    Some(McpServer::Stdio(
        McpServerStdio::new(name.to_string(), PathBuf::from(command.to_string()))
            .args(args)
            .env(env),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::McpServer;
    use nvim_oxi::Object;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

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

    // Stdio parsing helper
    fn create_stdio_dict() -> Object {
        let mut dict = Dictionary::new();
        let mut server = Dictionary::new();
        server.insert("name", "test-server");
        server.insert("command", "test-cmd");

        let servers = vec![server].into_iter().collect::<nvim_oxi::Array>();
        dict.insert("mcpServers", servers);

        Object::from(dict)
    }

    #[test]
    fn test_parse_stdio_defaults_is_configuration() {
        let obj = create_stdio_dict();
        let dict: Dictionary = obj.try_into().unwrap();
        let servers = parse_mcp_servers(dict.get("mcpServers").unwrap());
        assert!(servers.is_some());
        assert_eq!(servers.unwrap().len(), 1);
    }

    #[test]
    fn test_parse_stdio_name() {
        let obj = create_stdio_dict();
        let dict: Dictionary = obj.try_into().unwrap();
        let servers = parse_mcp_servers(dict.get("mcpServers").unwrap()).unwrap();
        match &servers[0] {
            McpServer::Stdio(s) => assert_eq!(s.name, "test-server"),
            _ => panic!("Expected Stdio server"),
        }
    }

    #[test]
    fn test_parse_stdio_command() {
        let obj = create_stdio_dict();
        let dict: Dictionary = obj.try_into().unwrap();
        let servers = parse_mcp_servers(dict.get("mcpServers").unwrap()).unwrap();
        match &servers[0] {
            McpServer::Stdio(s) => assert_eq!(s.command, PathBuf::from("test-cmd")),
            _ => panic!("Expected Stdio server"),
        }
    }

    // Helper for explicit types
    fn create_explicit_servers_dict() -> Object {
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

        Object::from(dict)
    }

    #[test]
    fn test_parse_explicit_sse_name() {
        let obj = create_explicit_servers_dict();
        let dict: Dictionary = obj.try_into().unwrap();
        let servers = parse_mcp_servers(dict.get("mcpServers").unwrap()).unwrap();
        match &servers[0] {
            McpServer::Sse(s) => assert_eq!(s.name, "sse-srv"),
            _ => panic!("Expected SSE server at index 0"),
        }
    }

    #[test]
    fn test_parse_explicit_sse_url() {
        let obj = create_explicit_servers_dict();
        let dict: Dictionary = obj.try_into().unwrap();
        let servers = parse_mcp_servers(dict.get("mcpServers").unwrap()).unwrap();
        match &servers[0] {
            McpServer::Sse(s) => assert_eq!(s.url, "http://localhost:8080"),
            _ => panic!("Expected SSE server at index 0"),
        }
    }

    #[test]
    fn test_parse_explicit_http_name() {
        let obj = create_explicit_servers_dict();
        let dict: Dictionary = obj.try_into().unwrap();
        let servers = parse_mcp_servers(dict.get("mcpServers").unwrap()).unwrap();
        match &servers[1] {
            McpServer::Http(h) => assert_eq!(h.name, "http-srv"),
            _ => panic!("Expected HTTP server at index 1"),
        }
    }

    #[test]
    fn test_parse_explicit_http_url() {
        let obj = create_explicit_servers_dict();
        let dict: Dictionary = obj.try_into().unwrap();
        let servers = parse_mcp_servers(dict.get("mcpServers").unwrap()).unwrap();
        match &servers[1] {
            McpServer::Http(h) => assert_eq!(h.url, "http://remote.com"),
            _ => panic!("Expected HTTP server at index 1"),
        }
    }

    #[test]
    fn test_parse_explicit_stdio_name() {
        let obj = create_explicit_servers_dict();
        let dict: Dictionary = obj.try_into().unwrap();
        let servers = parse_mcp_servers(dict.get("mcpServers").unwrap()).unwrap();
        match &servers[2] {
            McpServer::Stdio(s) => assert_eq!(s.name, "stdio-srv"),
            _ => panic!("Expected Stdio server at index 2"),
        }
    }

    #[test]
    fn test_parse_explicit_stdio_command() {
        let obj = create_explicit_servers_dict();
        let dict: Dictionary = obj.try_into().unwrap();
        let servers = parse_mcp_servers(dict.get("mcpServers").unwrap()).unwrap();
        match &servers[2] {
            McpServer::Stdio(s) => assert_eq!(s.command, PathBuf::from("bin")),
            _ => panic!("Expected Stdio server at index 2"),
        }
    }

    #[test]
    fn test_readme_example_compliance() {
        // Imitate the Lua table structure from README
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

        let servers_array: nvim_oxi::Array = vec![http_server, stdio_server].into_iter().collect();

        dict.insert("mcpServers", servers_array.clone());

        let servers = parse_mcp_servers(&Object::from(servers_array)).unwrap();
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
}
