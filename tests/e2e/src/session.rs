use std::time::Duration;

use crate::{utilities::autocommand, TIMEOUT_IN_SECONDS};
use agent_client_protocol::{
    InitializeResponse, LoadSessionResponse, NewSessionResponse, PromptResponse, StopReason,
};
use hermes::{
    api::{
        ConnectionArgs, CreateSessionArgs, DisconnectArgs, LoadSessionConfig, PromptArgs,
        PromptContent,
    },
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{conversion::FromObject, Array, Dictionary, Function, Object};

#[nvim_oxi::test]
fn test_default_session_creation() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    connect.call((nvim_oxi::String::from("opencode"), None))?;

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
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    connect.call((nvim_oxi::String::from("opencode"), None))?;

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

// TODO: Figure out how to test this
#[nvim_oxi::test]
#[ignore = "Cancel is optional and I haven't been able to find an agent that supports it in a testable way. Will retry in a more targeted effort."]
fn test_cancel_during_prompt() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let prompt: Function<PromptArgs, ()> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;
    let cancel: Function<String, ()> =
        FromObject::from_object(dict.get("cancel").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    connect.call((nvim_oxi::String::from("opencode"), None))?;

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
            .map(FromObject::from_object)
            .collect::<Result<Vec<_>, _>>()?,
    );

    prompt.call((session_id.to_string(), content))?;

    std::thread::sleep(Duration::from_secs(1));

    cancel.call(session_id.to_string())?;

    let response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;

    if !matches!(response.stop_reason, StopReason::Cancelled) {
        return Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Expected stop_reason to be Cancelled, got {:?}",
            response.stop_reason
        ))));
    }

    Ok(())
}

#[nvim_oxi::test]
fn test_load_session() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let load_session: Function<(String, Option<LoadSessionConfig>), ()> =
        FromObject::from_object(dict.get("load_session").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    connect.call((nvim_oxi::String::from("opencode"), None))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Create a session first
    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    // Disconnect
    disconnect.call(DisconnectArgs::All)?;

    // Reconnect to load the session
    let wait_for_initialization2 =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_loaded_session =
        autocommand::listen_for_autocommand::<LoadSessionResponse>(Commands::SessionLoaded);

    connect.call((nvim_oxi::String::from("opencode"), None))?;

    wait_for_initialization2(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Load the session
    let config = LoadSessionConfig {
        cwd: Some(std::path::PathBuf::from(".")),
        mcp_servers: Vec::new(),
    };
    load_session.call((session_id.clone(), Some(config)))?;

    let loaded_session = wait_for_loaded_session(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;

    assert!(loaded_session.is_ok());

    Ok(())
}
