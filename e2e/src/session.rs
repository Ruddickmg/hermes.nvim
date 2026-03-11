use std::time::Duration;

use crate::{utilities::autocommand, TIMEOUT_IN_SECONDS};
use agent_client_protocol::{InitializeResponse, NewSessionResponse, PromptResponse, StopReason};
use hermes::{
    api::{CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{conversion::FromObject, Array, Dictionary, Function, Object};

#[nvim_oxi::test]
fn test_default_session_creation() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<(nvim_oxi::String, Option<Dictionary>), ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("createSession").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::CreatedSession);

    // Create options dictionary with protocol
    let mut options = Dictionary::new();
    options.insert("protocol", "stdio");

    connect.call((nvim_oxi::String::from("opencode"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;

    assert!(session.is_ok());

    Ok(())
}

#[nvim_oxi::test]
fn test_custom_session_creation() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<(nvim_oxi::String, Option<Dictionary>), ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("createSession").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::CreatedSession);

    // Create options dictionary with protocol
    let mut options = Dictionary::new();
    options.insert("protocol", "stdio");

    connect.call((nvim_oxi::String::from("opencode"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Create session with custom configuration
    create_session.call(CreateSessionArgs::Configuration {
        cwd: Some(".".into()),
        mcp_servers: None,
    })?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;

    assert!(session.is_ok());

    Ok(())
}

// Ignored: Test agents (opencode v1.2.24, copilot) don't fully support ACP cancellation feature
// Cancel notification is sent correctly but agents complete with EndTurn instead of Cancelled
// TODO: figure this out when I can find a client that this test will work for
#[nvim_oxi::test]
#[ignore]
fn test_cancel_during_prompt() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<(nvim_oxi::String, Option<Dictionary>), ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("createSession").unwrap().clone())?;
    let prompt: Function<PromptArgs, ()> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;
    let cancel: Function<String, ()> =
        FromObject::from_object(dict.get("cancel").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::CreatedSession);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    // Create options dictionary with protocol
    let mut options = Dictionary::new();
    options.insert("protocol", "stdio");

    connect.call((nvim_oxi::String::from("opencode"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert(
        "text",
        "Write a detailed 1000-word essay about artificial intelligence and its impact on society. Include multiple paragraphs covering: introduction to AI, current applications, ethical considerations, future implications, and conclusion. Make it comprehensive with specific examples.",
    );
    let content_array = Array::from_iter(vec![Object::from(content_dict)]);
    let content = PromptContent::Multiple(
        content_array
            .into_iter()
            .map(|obj| FromObject::from_object(obj))
            .collect::<Result<Vec<_>, _>>()?,
    );

    prompt.call((session_id.to_string(), content))?;

    // Wait for generation to start, then cancel
    std::thread::sleep(Duration::from_secs(5));
    cancel.call(session_id.to_string())?;

    let response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;

    assert!(
        matches!(response.stop_reason, StopReason::Cancelled),
        "Prompt should complete as cancelled"
    );

    Ok(())
}
