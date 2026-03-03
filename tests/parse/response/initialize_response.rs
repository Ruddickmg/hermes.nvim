use agent_client_protocol::{AuthMethod, AuthMethodId, Implementation, InitializeResponse};
use hermes::nvim::parse::response::initialize_response;

#[test]
fn test_initialize_response_basic() {
    let response = InitializeResponse::default();

    let result = initialize_response(response);

    assert_eq!(result.get("protocolVersion").is_some(), true);
    assert_eq!(result.get("agentCapabilities").is_some(), true);
    assert_eq!(result.get("authMethods").is_some(), true);
}

#[test]
fn test_initialize_response_protocol_version() {
    let response = InitializeResponse::default();

    let result = initialize_response(response);
    let protocol_version = result.get("protocolVersion").unwrap();

    assert_eq!(protocol_version.get("value").is_some(), true);
}

#[test]
fn test_initialize_response_agent_capabilities() {
    let response = InitializeResponse::default();

    let result = initialize_response(response);
    let capabilities = result.get("agentCapabilities").unwrap();

    assert_eq!(capabilities.get("loadSession").is_some(), true);
    assert_eq!(capabilities.get("promptCapabilities").is_some(), true);
    assert_eq!(capabilities.get("mcpCapabilities").is_some(), true);
    assert_eq!(capabilities.get("sessionCapabilities").is_some(), true);
}

#[test]
fn test_initialize_response_prompt_capabilities() {
    let response = InitializeResponse::default();

    let result = initialize_response(response);
    let capabilities = result.get("agentCapabilities").unwrap();
    let prompt_caps = capabilities.get("promptCapabilities").unwrap();

    assert_eq!(prompt_caps.get("image").is_some(), true);
    assert_eq!(prompt_caps.get("audio").is_some(), true);
    assert_eq!(prompt_caps.get("embeddedContext").is_some(), true);
}

#[test]
fn test_initialize_response_mcp_capabilities() {
    let response = InitializeResponse::default();

    let result = initialize_response(response);
    let capabilities = result.get("agentCapabilities").unwrap();
    let mcp_caps = capabilities.get("mcpCapabilities").unwrap();

    assert_eq!(mcp_caps.get("http").is_some(), true);
    assert_eq!(mcp_caps.get("sse").is_some(), true);
}

#[test]
fn test_initialize_response_session_capabilities() {
    let response = InitializeResponse::default();

    let result = initialize_response(response);
    let capabilities = result.get("agentCapabilities").unwrap();
    let session_caps = capabilities.get("sessionCapabilities").unwrap();

    assert_eq!(session_caps.get("list").is_some(), true);
    assert_eq!(session_caps.get("fork").is_some(), true);
    assert_eq!(session_caps.get("resume").is_some(), true);
}

#[test]
fn test_initialize_response_auth_methods_empty() {
    let response = InitializeResponse::default();

    let result = initialize_response(response);
    let auth_methods = result.get("authMethods").unwrap();

    let arr = auth_methods.as_array().unwrap();
    assert_eq!(arr.is_empty(), true);
}

#[test]
fn test_initialize_response_auth_methods_with_method() {
    let mut response = InitializeResponse::default();
    let method = AuthMethod::new(AuthMethodId::new("oauth"), "OAuth");
    response.auth_methods = vec![method];

    let result = initialize_response(response);
    let auth_methods = result.get("authMethods").unwrap();

    let arr = auth_methods.as_array().unwrap();
    assert_eq!(arr.len(), 1);

    let method_dict = arr[0].as_dictionary().unwrap();
    assert_eq!(method_dict.get("id").unwrap().as_str().unwrap(), "oauth");
    assert_eq!(method_dict.get("name").unwrap().as_str().unwrap(), "OAuth");
}

#[test]
fn test_initialize_response_without_agent_info() {
    let response = InitializeResponse::default();

    let result = initialize_response(response);

    assert_eq!(result.get("agentInfo").is_some(), false);
}

#[test]
fn test_initialize_response_with_agent_info() {
    let mut response = InitializeResponse::default();
    response.agent_info = Some(Implementation::new("test-agent", "1.0.0"));

    let result = initialize_response(response);
    let agent_info = result.get("agentInfo").unwrap();

    assert_eq!(
        agent_info.get("name").unwrap().as_str().unwrap(),
        "test-agent"
    );
    assert_eq!(
        agent_info.get("version").unwrap().as_str().unwrap(),
        "1.0.0"
    );
}

#[test]
fn test_initialize_response_with_agent_info_title() {
    let mut response = InitializeResponse::default();
    response.agent_info = Some(Implementation::new("test-agent", "1.0.0").title("Test Agent"));

    let result = initialize_response(response);
    let agent_info = result.get("agentInfo").unwrap();

    assert_eq!(
        agent_info.get("title").unwrap().as_str().unwrap(),
        "Test Agent"
    );
}

#[test]
fn test_initialize_response_without_meta() {
    let response = InitializeResponse::default();

    let result = initialize_response(response);

    assert_eq!(result.get("meta").is_some(), false);
}

#[test]
fn test_initialize_response_with_meta() {
    let mut response = InitializeResponse::default();
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    response.meta = Some(meta);

    let result = initialize_response(response);

    assert_eq!(result.get("meta").is_some(), true);
}
