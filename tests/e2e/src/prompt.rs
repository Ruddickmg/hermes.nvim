use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent},
};
use agent_client_protocol::{InitializeResponse, NewSessionResponse, PromptResponse, StopReason};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Array, Dictionary, Function, Object, conversion::FromObject};
use pretty_assertions::assert_eq;

#[nvim_oxi::test]
fn test_setup_returns_prompt_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("prompt").is_some(),
        "prompt function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_prompt_single_content() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let prompt: Function<PromptArgs, ()> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    // Start mock agent
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Create single text content
    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Hello, what time is it?");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session_id.to_string(), content))?;

    // Wait for agent response (mock agent is fast)
    let response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(response.stop_reason, StopReason::EndTurn);

    Ok(())
}

#[nvim_oxi::test]
fn test_prompt_multiple_content() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let prompt: Function<PromptArgs, ()> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    // Start mock agent
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Create array of content types (only text and link are supported by mock agent)
    let mut text_dict = Dictionary::new();
    text_dict.insert("type", "text");
    text_dict.insert("text", "What time is it?");

    let mut link_dict = Dictionary::new();
    link_dict.insert("type", "link");
    link_dict.insert("name", "Example file");
    link_dict.insert("uri", "/path/to/example.txt");

    let content_array = Array::from_iter(vec![Object::from(text_dict), Object::from(link_dict)]);

    let content = PromptContent::Multiple(
        content_array
            .into_iter()
            .map(FromObject::from_object)
            .collect::<Result<Vec<_>, _>>()?,
    );

    prompt.call((session_id.to_string(), content))?;

    // Wait for agent response (mock agent is fast)
    let response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(response.stop_reason, StopReason::EndTurn);

    Ok(())
}

#[nvim_oxi::test]
fn test_prompt_with_invalid_content_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let prompt: Function<(String, Object), ()> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    // Pass a number as content instead of expected array/table
    let result = prompt.call(("test-session".to_string(), Object::from(0i64)));

    assert!(result.is_ok(), "prompt should succeed with invalid content");

    Ok(())
}
