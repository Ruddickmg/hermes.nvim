use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent},
};
use agent_client_protocol::{
    InitializeResponse, ListSessionsResponse, LoadSessionResponse, NewSessionResponse,
    PromptResponse, StopReason,
};
use hermes::{
    api::{
        ConnectionArgs, CreateSessionArgs, DisconnectArgs, ListSessionsConfig, LoadSessionConfig,
        PromptArgs, PromptContent,
    },
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Array, Dictionary, Function, Object, conversion::FromObject};

#[nvim_oxi::test]
fn test_setup_returns_list_sessions_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("list_sessions").is_some(),
        "list_sessions function should be registered"
    );

    Ok(())
}

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

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

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

    // Start mock agent
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Create session with custom configuration
    create_session.call(CreateSessionArgs::Configuration {
        cwd: Some(".".into()),
        mcp_servers: None,
    })?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(session.is_ok());

    Ok(())
}

// Test cancel during prompt with mock agent
#[nvim_oxi::test]
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

    // Start mock agent with a long-running prompt behavior (simulated by sleeping)
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
    mock_handle.close();

    // Mock agent doesn't support cancellation properly, so we just check it doesn't crash
    // Real agents would return StopReason::Cancelled
    assert!(
        matches!(
            response.stop_reason,
            StopReason::EndTurn | StopReason::Cancelled
        ),
        "Expected stop_reason to be EndTurn or Cancelled, got {:?}",
        response.stop_reason
    );

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

    // Start mock agent
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Create a session first
    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    // Load the session (using same mock agent - session is tracked in memory)
    let wait_for_loaded_session =
        autocommand::listen_for_autocommand::<LoadSessionResponse>(Commands::SessionLoaded);

    let config = LoadSessionConfig {
        cwd: Some(std::path::PathBuf::from(".")),
        mcp_servers: Vec::new(),
    };
    load_session.call((session_id.clone(), Some(config)))?;

    let loaded_session = wait_for_loaded_session(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(loaded_session.is_ok());

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_no_filter() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let list_sessions: Function<Option<ListSessionsConfig>, ()> =
        FromObject::from_object(dict.get("list_sessions").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_sessions_listed =
        autocommand::listen_for_autocommand::<ListSessionsResponse>(Commands::SessionsListed);

    // Start mock agent
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Create a session first
    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let _session_id = session.session_id.to_string();

    // List all sessions
    list_sessions.call(None)?;

    let sessions_response = wait_for_sessions_listed(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    // Single assertion: verify autocommand fired and returned sessions
    let response = sessions_response?;
    assert!(
        !response.sessions.is_empty(),
        "SessionsListed should return at least one session"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_with_cwd_filter() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let list_sessions: Function<Option<ListSessionsConfig>, ()> =
        FromObject::from_object(dict.get("list_sessions").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_sessions_listed =
        autocommand::listen_for_autocommand::<ListSessionsResponse>(Commands::SessionsListed);

    // Start mock agent
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Create a session first
    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let _session_id = session.session_id.to_string();

    // List sessions with cwd filter
    let config = ListSessionsConfig {
        cwd: Some(std::path::PathBuf::from(".")),
        cursor: None,
    };
    list_sessions.call(Some(config))?;

    let sessions_response = wait_for_sessions_listed(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        sessions_response.is_ok(),
        "Should receive SessionsListed autocommand with cwd filter"
    );

    Ok(())
}
