use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand,
        mock_agent::MockAgent,
        mock_config::{MockConfig, create_test_permission_request},
    },
};
use agent_client_protocol::{
    InitializeResponse, NewSessionResponse, PermissionOption, SessionId, ToolCallUpdate,
};
use hermes::{
    api::{
        ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent, RespondArgs,
    },
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, Object, conversion::FromObject};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

fn create_func<A>(plugin: Dictionary, name: &str) -> Function<A, ()> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .unwrap_or_else(|_| panic!("Failed to create function for {}", name))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PermissionRequestData {
    session_id: SessionId,
    request_id: String,
    tool_call: ToolCallUpdate,
    options: Vec<PermissionOption>,
}

/// Test that the mock agent triggers a PermissionRequest autocommand in Hermes.
///
/// This verifies:
/// 1. Mock agent connects via socket protocol
/// 2. Mock agent handles initialize + new_session
/// 3. When prompted, mock agent sends a RequestPermissionRequest to Hermes
/// 4. Hermes fires the PermissionRequest autocommand
#[nvim_oxi::test]
fn test_permission_request_fires_with_mock_agent() -> Result<(), nvim_oxi::Error> {
    // 1. Create mock agent configured to request permission
    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new()
            .set_permission_request(create_test_permission_request("mock-session"));
    }
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect = create_func::<ConnectionArgs>(dict.clone(), "connect");
    let disconnect = create_func::<DisconnectArgs>(dict.clone(), "disconnect");
    let create_session = create_func::<CreateSessionArgs>(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs>(dict.clone(), "prompt");

    // Set up autocommand listeners
    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_permission_request =
        autocommand::listen_for_autocommand::<PermissionRequestData>(Commands::PermissionRequest);

    // 2. Connect to mock agent via socket protocol
    let mut options = Dictionary::new();
    options.insert("protocol", "socket");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    let init_response = wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    info!("Mock agent initialized: {:?}", init_response);

    // 3. Create a session
    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;
    info!("Mock agent session created: {}", session_id);

    // 4. Send a prompt - mock agent is configured to request permission
    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Run a tool that needs permission");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session_id.to_string(), content))?;

    // 5. Wait for PermissionRequest autocommand
    let permission_request = wait_for_permission_request(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    info!(
        "Received PermissionRequest autocommand: {:?}",
        permission_request
    );

    // 6. Cleanup - disconnect without responding to permission request
    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        !permission_request.request_id.is_empty(),
        "PermissionRequest should have a request_id"
    );

    Ok(())
}

// Test that respond with invalid UUID doesn't crash
#[nvim_oxi::test]
fn respond_with_invalid_uuid_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let respond = create_func::<RespondArgs>(dict.clone(), "respond");

    // Call respond with an invalid UUID format - should log error and return Ok
    let result = respond.call((
        "invalid-uuid-not-valid".to_string(),
        nvim_oxi::Object::from("test response data"),
    ));

    assert!(
        result.is_ok(),
        "respond with invalid UUID should return Ok, not crash"
    );

    Ok(())
}

// Test that respond with unknown request ID doesn't crash
#[nvim_oxi::test]
fn respond_with_unknown_request_id_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let respond = create_func::<RespondArgs>(dict.clone(), "respond");

    // Call respond with a valid UUID format but unknown request ID
    let result = respond.call((
        "550e8400-e29b-41d4-a716-446655440000".to_string(),
        nvim_oxi::Object::from("test response data"),
    ));

    assert!(
        result.is_ok(),
        "respond with unknown request ID should return Ok, not crash"
    );

    Ok(())
}
