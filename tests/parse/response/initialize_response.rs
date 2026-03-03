use agent_client_protocol::{
    AuthMethod, AuthMethodId, Implementation, InitializeResponse, ProtocolVersion,
};
use hermes::nvim::parse::response::initialize_response;
use nvim_oxi::Object;

fn make_test_response() -> InitializeResponse {
    InitializeResponse::new(ProtocolVersion::LATEST)
}

fn get_dict(obj: &Object) -> &nvim_oxi::Dictionary {
    unsafe { obj.as_dictionary_unchecked() }
}

fn get_array(obj: &Object) -> &nvim_oxi::Array {
    unsafe { obj.as_array_unchecked() }
}

#[test]
fn test_initialize_response_basic() {
    let response = make_test_response();

    let result = initialize_response(response);

    let protocol_version = result.get("protocolVersion").unwrap();
    assert_eq!(
        protocol_version.kind() == nvim_oxi::ObjectKind::String,
        true
    );

    let agent_capabilities = result.get("agentCapabilities").unwrap();
    let caps_dict = get_dict(agent_capabilities);
    assert_eq!(caps_dict.get("loadSession").is_some(), true);

    let auth_methods = result.get("authMethods").unwrap();
    let arr = get_array(auth_methods);
    assert_eq!(arr.is_empty(), true);
}

#[test]
fn test_initialize_response_protocol_version() {
    let response = make_test_response();

    let result = initialize_response(response);
    let protocol_version = result.get("protocolVersion").unwrap();

    assert_eq!(
        protocol_version.kind() == nvim_oxi::ObjectKind::String,
        true
    );
}

#[test]
fn test_initialize_response_agent_capabilities() {
    let response = make_test_response();

    let result = initialize_response(response);
    let capabilities = result.get("agentCapabilities").unwrap();

    let caps_dict = get_dict(capabilities);
    let load_session = caps_dict.get("loadSession").unwrap();
    assert_eq!(load_session.kind() == nvim_oxi::ObjectKind::Boolean, true);

    let prompt_caps = caps_dict.get("promptCapabilities").unwrap();
    let prompt_dict = get_dict(prompt_caps);
    assert_eq!(prompt_dict.get("image").is_some(), true);

    let mcp_caps = caps_dict.get("mcpCapabilities").unwrap();
    let mcp_dict = get_dict(mcp_caps);
    assert_eq!(mcp_dict.get("http").is_some(), true);

    let session_caps = caps_dict.get("sessionCapabilities").unwrap();
    let session_dict = get_dict(session_caps);
    assert_eq!(session_dict.get("list").is_some(), false);
}

#[test]
fn test_initialize_response_prompt_capabilities() {
    let response = make_test_response();

    let result = initialize_response(response);
    let capabilities = result.get("agentCapabilities").unwrap();
    let caps_dict = get_dict(capabilities);
    let prompt_caps = caps_dict.get("promptCapabilities").unwrap();

    let prompt_dict = get_dict(prompt_caps);
    let image = prompt_dict.get("image").unwrap();
    assert_eq!(image.kind() == nvim_oxi::ObjectKind::Boolean, true);

    let audio = prompt_dict.get("audio").unwrap();
    assert_eq!(audio.kind() == nvim_oxi::ObjectKind::Boolean, true);

    let embedded_context = prompt_dict.get("embeddedContext").unwrap();
    assert_eq!(
        embedded_context.kind() == nvim_oxi::ObjectKind::Boolean,
        true
    );
}

#[test]
fn test_initialize_response_mcp_capabilities() {
    let response = make_test_response();

    let result = initialize_response(response);
    let capabilities = result.get("agentCapabilities").unwrap();
    let caps_dict = get_dict(capabilities);
    let mcp_caps = caps_dict.get("mcpCapabilities").unwrap();

    let mcp_dict = get_dict(mcp_caps);
    let http = mcp_dict.get("http").unwrap();
    assert_eq!(http.kind() == nvim_oxi::ObjectKind::Boolean, true);

    let sse = mcp_dict.get("sse").unwrap();
    assert_eq!(sse.kind() == nvim_oxi::ObjectKind::Boolean, true);
}

#[test]
fn test_initialize_response_session_capabilities() {
    let response = make_test_response();

    let result = initialize_response(response);
    let capabilities = result.get("agentCapabilities").unwrap();
    let caps_dict = get_dict(capabilities);

    let session_caps = caps_dict.get("sessionCapabilities").unwrap();
    let session_dict = get_dict(session_caps);
    assert_eq!(session_dict.get("list").is_some(), false);
}

#[test]
fn test_initialize_response_auth_methods_empty() {
    let response = make_test_response();

    let result = initialize_response(response);
    let auth_methods = result.get("authMethods").unwrap();

    let arr = get_array(auth_methods);
    assert_eq!(arr.is_empty(), true);
}

#[test]
fn test_initialize_response_auth_methods_with_method() {
    let mut response = make_test_response();
    let method = AuthMethod::new(AuthMethodId::new("oauth"), "OAuth");
    response.auth_methods = vec![method];

    let result = initialize_response(response);
    let auth_methods = result.get("authMethods").unwrap();

    let arr = get_array(auth_methods);
    assert_eq!(arr.len(), 1);

    let method_dict = unsafe { arr[0].as_dictionary_unchecked() };
    unsafe {
        assert_eq!(
            method_dict
                .get("id")
                .unwrap()
                .as_nvim_str_unchecked()
                .to_string(),
            "oauth"
        );
        assert_eq!(
            method_dict
                .get("name")
                .unwrap()
                .as_nvim_str_unchecked()
                .to_string(),
            "OAuth"
        );
    }
}

#[test]
fn test_initialize_response_without_agent_info() {
    let response = make_test_response();

    let result = initialize_response(response);

    assert_eq!(result.get("agentInfo").is_some(), false);
}

#[test]
fn test_initialize_response_with_agent_info() {
    let mut response = make_test_response();
    response.agent_info = Some(Implementation::new("test-agent", "1.0.0"));

    let result = initialize_response(response);
    let agent_info = result.get("agentInfo").unwrap();

    let info_dict = get_dict(agent_info);
    unsafe {
        assert_eq!(
            info_dict
                .get("name")
                .unwrap()
                .as_nvim_str_unchecked()
                .to_string(),
            "test-agent"
        );
        assert_eq!(
            info_dict
                .get("version")
                .unwrap()
                .as_nvim_str_unchecked()
                .to_string(),
            "1.0.0"
        );
    }
}

#[test]
fn test_initialize_response_with_agent_info_title() {
    let mut response = make_test_response();
    response.agent_info = Some(Implementation::new("test-agent", "1.0.0").title("Test Agent"));

    let result = initialize_response(response);
    let agent_info = result.get("agentInfo").unwrap();

    let info_dict = get_dict(agent_info);
    unsafe {
        assert_eq!(
            info_dict
                .get("title")
                .unwrap()
                .as_nvim_str_unchecked()
                .to_string(),
            "Test Agent"
        );
    }
}

#[test]
fn test_initialize_response_without_meta() {
    let response = make_test_response();

    let result = initialize_response(response);

    assert_eq!(result.get("meta").is_some(), false);
}

#[test]
fn test_initialize_response_with_meta() {
    let mut response = make_test_response();
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    response.meta = Some(meta);

    let result = initialize_response(response);

    let meta_obj = result.get("meta").unwrap();
    let meta_dict = get_dict(meta_obj);
    let source = meta_dict.get("source").unwrap();
    unsafe {
        assert_eq!(source.as_nvim_str_unchecked().to_string(), "test");
    }
}
