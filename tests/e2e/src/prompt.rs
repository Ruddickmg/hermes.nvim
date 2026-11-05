use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent, test_helpers::connect_to_mock_agent},
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::v1::schema::{
    AgentCapabilities, InitializeResponse, NewSessionResponse, PromptResponse, SessionCapabilities,
    SessionResumeCapabilities, StopReason,
};
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
    let prompt: Function<PromptArgs, Option<nvim_oxi::String>> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    // Start mock agent
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Create single text content
    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Hello, what time is it?");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    let _prompt_result = prompt.call((session_id.to_string(), content))?;

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
    let prompt: Function<PromptArgs, Option<nvim_oxi::String>> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    // Start mock agent
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

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

    let _prompt_result = prompt.call((session_id.to_string(), content))?;

    // Wait for agent response (mock agent is fast)
    let response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(response.stop_reason, StopReason::EndTurn);

    Ok(())
}

#[nvim_oxi::test]
fn test_prompt_with_invalid_content_returns_nil() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let prompt: Function<(String, Object), Option<nvim_oxi::String>> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    // Pass a number as content instead of expected array/table
    let result = prompt.call(("test-session".to_string(), Object::from(0i64)));

    // api_method swallows errors into None (nil), so the call succeeds but returns nil
    assert_eq!(
        result,
        Ok(None),
        "prompt with invalid content should return nil"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_prompt_returns_prompt_id() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let prompt: Function<PromptArgs, Option<nvim_oxi::String>> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    // Start mock agent
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Hello, what time is it?");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    let prompt_result = prompt.call((session_id.to_string(), content))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    let prompt_id = prompt_result.expect("prompt should return a prompt_id string");
    assert!(
        !prompt_id.is_empty(),
        "prompt_id should be a non-empty UUID"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_prompt_writes_history() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let prompt: Function<PromptArgs, Option<nvim_oxi::String>> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    // Start MockAgent configured for local history
    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let caps = AgentCapabilities::new()
            .load_session(false)
            .session_capabilities(
                SessionCapabilities::new().resume(Some(SessionResumeCapabilities::new())),
            );
        config.initialize_response =
            InitializeResponse::new(ProtocolVersion::LATEST).agent_capabilities(caps);
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Send prompt with multiple text content blocks
    let mut text1 = Dictionary::new();
    text1.insert("type", "text");
    text1.insert("text", "Hello");
    let mut text2 = Dictionary::new();
    text2.insert("type", "text");
    text2.insert("text", "World");
    let mut text3 = Dictionary::new();
    text3.insert("type", "text");
    text3.insert("text", "Test");

    let content_array = Array::from_iter(vec![
        Object::from(text1),
        Object::from(text2),
        Object::from(text3),
    ]);
    let content = PromptContent::Multiple(
        content_array
            .into_iter()
            .map(FromObject::from_object)
            .collect::<Result<Vec<_>, _>>()?,
    );

    let prompt_result = prompt.call((session_id.to_string(), content))?;
    let _prompt_id = prompt_result.expect("prompt should return a prompt_id string");
    let port = mock_handle.port();

    // Allow the background worker's periodic flush (100ms timeout) to write keyed history
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Verify history file on disk — single assertion checking 6 lines (3 user + 3 agent chunks)
    let state_dir =
        nvim_oxi::api::call_function::<(String,), String>("stdpath", ("state".to_string(),))?;
    let history_path = format!(
        "{}/hermes/history/mock-agent (localhost:{})/{}.jsonl",
        state_dir, port, session_id
    );
    let file_content =
        std::fs::read_to_string(&history_path).expect("history file should exist after prompt");
    assert_eq!(file_content.trim().lines().count(), 6);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();
    Ok(())
}
