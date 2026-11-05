//! Example E2E tests using the Mock Agent
//!
//! These tests demonstrate how to use the MockAgent for testing
//! without requiring external agents (opencode, copilot) to be installed.

use agent_client_protocol::v1::schema::{InitializeResponse, NewSessionResponse};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent, test_helpers::connect_to_mock_agent},
};

fn create_func<A, R>(plugin: Dictionary, name: &str) -> Function<A, R> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .unwrap_or_else(|_| panic!("Failed to create function for {}", name))
}

/// Test connecting to a mock agent via socket protocol
#[nvim_oxi::test]
fn test_mock_agent_connection() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    // 1. Create mock agent and start it
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    // 2. Connect to mock agent using socket protocol
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");
    connect_to_mock_agent(&connect, &mock_handle)?;

    // 3. Create a session
    let create_session: Function<CreateSessionArgs, ()> =
        create_func(dict.clone(), "create_session");
    create_session.call(CreateSessionArgs::Default)?;

    // 4. Disconnect
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");
    disconnect.call(DisconnectArgs::All)?;

    // 5. Mock agent automatically shuts down when handle is dropped
    mock_handle.close();

    Ok(())
}

/// Test basic prompt with mock agent
#[nvim_oxi::test]
fn test_mock_agent_prompt() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    // Start mock agent
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    // Connect
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");

    // Set up autocommand listeners
    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    connect_to_mock_agent(&connect, &mock_handle)?;

    // Wait for connection to be fully initialized before creating session
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Create session and capture the session ID
    let create_session: Function<CreateSessionArgs, ()> =
        create_func(dict.clone(), "create_session");
    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Send prompt using the actual session ID
    let prompt: Function<PromptArgs, Option<nvim_oxi::String>> =
        create_func(dict.clone(), "prompt");
    let content = PromptContent::Single(hermes::api::ContentBlockType::Text {
        text: "Hello, mock agent!".to_string(),
    });
    let prompt_result = prompt.call((session_id.to_string(), content))?;
    assert!(prompt_result.is_some(), "prompt should return a prompt_id");

    // Disconnect and cleanup
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");
    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}
