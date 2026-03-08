use agent_client_protocol::McpServer;
use hermes::nvim::api::CreateSessionArgs;
use nvim_oxi::{Dictionary, Object, conversion::FromObject};
use std::path::PathBuf;

#[nvim_oxi::test]
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
    dict.insert("mcp_servers", servers);

    let obj = Object::from(dict);
    CreateSessionArgs::from_object(obj).unwrap()
}

#[nvim_oxi::test]
fn test_stdio_defaults_is_configuration() {
    let args = create_stdio_default_args();
    match args {
        CreateSessionArgs::Configuration { .. } => assert!(true),
        _ => panic!("Expected Configuration variant"),
    }
}

#[nvim_oxi::test]
fn test_stdio_defaults_server_count() {
    let args = create_stdio_default_args();
    match args {
        CreateSessionArgs::Configuration { mcp_servers, .. } => {
            assert_eq!(mcp_servers.unwrap().len(), 1);
        }
        _ => panic!("Expected Configuration variant"),
    }
}

#[nvim_oxi::test]
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

#[nvim_oxi::test]
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
    dict.insert("mcp_servers", servers);

    let obj = Object::from(dict);
    CreateSessionArgs::from_object(obj).unwrap()
}

#[nvim_oxi::test]
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

#[nvim_oxi::test]
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

#[nvim_oxi::test]
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

#[nvim_oxi::test]
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

#[nvim_oxi::test]
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

#[nvim_oxi::test]
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

#[nvim_oxi::test]
fn test_explicit_stdio_args() {
    let args = create_explicit_args();
    match args {
        CreateSessionArgs::Configuration { mcp_servers, .. } => {
            let servers = mcp_servers.unwrap();
            match &servers[2] {
                McpServer::Stdio(s) => assert_eq!(s.args, vec!["arg1", "arg2"]),
                _ => panic!("Expected Stdio server at index 2"),
            }
        }
        _ => panic!("Expected Configuration variant"),
    }
}
