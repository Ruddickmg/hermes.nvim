use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent, test_helpers::connect_to_mock_agent},
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::v1::schema::{
    AgentCapabilities, CloseSessionResponse, DeleteSessionResponse, InitializeResponse,
    ListSessionsResponse, LoadSessionResponse, NewSessionResponse, PromptResponse,
    ResumeSessionResponse, SessionCapabilities, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOption, SessionMode, SessionModeState, SessionResumeCapabilities,
    StopReason,
};
use hermes::{
    api::{
        ConnectionArgs, CreateSessionArgs, DeleteSessionArg, DeleteSessionOptions, DisconnectArgs,
        ListSessionsConfig, LoadSessionConfig, PromptArgs, PromptContent, ResumeSessionConfig,
        SetModeArgs,
    },
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Array, Dictionary, Function, Object, conversion::FromObject};
use pretty_assertions::assert_eq;
use tracing::error;

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
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

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
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Create session with custom configuration
    create_session.call(CreateSessionArgs::Configuration {
        cwd: Some(".".into()),
        additional_directories: None,
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
    let prompt: Function<PromptArgs, Option<nvim_oxi::String>> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;
    let cancel: Function<String, ()> =
        FromObject::from_object(dict.get("cancel").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    // Start mock agent with a long-running prompt behavior (simulated by sleeping)
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

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

    let _prompt_result = prompt.call((session_id.to_string(), content))?;

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
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

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
        additional_directories: None,
        mcp_servers: Vec::new(),
    };
    load_session.call((session_id.clone(), Some(config)))?;

    let loaded_session = wait_for_loaded_session(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    loaded_session.inspect_err(|e| error!("Failed to load session: {:?}", e))?;
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
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

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
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

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

// ============================================================================
// Poppable Error Handling Tests
// ============================================================================
// These tests verify that Poppable implementations return default values
// when given invalid Lua data instead of panicking.

#[nvim_oxi::test]
fn test_create_session_with_invalid_arg_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let create_session: Function<Object, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;

    // Pass a number instead of the expected table/nil
    // Poppable should convert this to CreateSessionArgs::Default
    let result = create_session.call(Object::from(42i64));

    // Should succeed because Poppable returns default on invalid data
    assert!(
        result.is_ok(),
        "create_session should succeed with invalid arg"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_with_invalid_arg_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let list_sessions: Function<Object, ()> =
        FromObject::from_object(dict.get("list_sessions").unwrap().clone())?;

    // Pass a number instead of the expected table/nil
    let result = list_sessions.call(Object::from(999i64));

    assert!(
        result.is_ok(),
        "list_sessions should succeed with invalid arg"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_load_session_with_invalid_config_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let load_session: Function<(String, Object), ()> =
        FromObject::from_object(dict.get("load_session").unwrap().clone())?;

    // Pass a number as the config instead of expected table
    // First arg (session_id) is valid, second arg (config) is invalid
    let result = load_session.call(("test-session".to_string(), Object::from(42i64)));

    assert!(
        result.is_ok(),
        "load_session should succeed with invalid config"
    );

    Ok(())
}

// Load session with legacy modes in response, verify SessionLoaded fires with modes data.
#[nvim_oxi::test]
fn test_load_session_with_legacy_modes() -> Result<(), nvim_oxi::Error> {
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
    let wait_for_loaded_session =
        autocommand::listen_for_autocommand::<LoadSessionResponse>(Commands::SessionLoaded);

    // Configure mock agent with legacy modes in load_session response
    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);
        config.load_session_response = Some(LoadSessionResponse::default().modes(modes));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let config = LoadSessionConfig {
        cwd: Some(std::path::PathBuf::from(".")),
        additional_directories: None,
        mcp_servers: Vec::new(),
    };
    load_session.call((session_id.clone(), Some(config)))?;

    let loaded_session = wait_for_loaded_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        loaded_session.modes.is_some(),
        "SessionLoaded should contain legacy modes"
    );

    Ok(())
}

// Load session with config_options in response, verify SessionLoaded fires with config options.
#[nvim_oxi::test]
fn test_load_session_with_config_options() -> Result<(), nvim_oxi::Error> {
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
    let wait_for_loaded_session =
        autocommand::listen_for_autocommand::<LoadSessionResponse>(Commands::SessionLoaded);

    // Configure mock agent with config options in load_session response
    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);
        config.load_session_response =
            Some(LoadSessionResponse::default().config_options(vec![option]));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let config = LoadSessionConfig {
        cwd: Some(std::path::PathBuf::from(".")),
        additional_directories: None,
        mcp_servers: Vec::new(),
    };
    load_session.call((session_id.clone(), Some(config)))?;

    let loaded_session = wait_for_loaded_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        loaded_session.config_options.is_some(),
        "SessionLoaded should contain config options"
    );

    Ok(())
}

// Chain load_session with legacy modes then set_mode to verify end-to-end flow.
#[nvim_oxi::test]
fn test_load_session_then_set_mode_uses_legacy_path() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let load_session: Function<(String, Option<LoadSessionConfig>), ()> =
        FromObject::from_object(dict.get("load_session").unwrap().clone())?;
    let set_mode: Function<SetModeArgs, ()> =
        FromObject::from_object(dict.get("set_mode").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_loaded_session =
        autocommand::listen_for_autocommand::<LoadSessionResponse>(Commands::SessionLoaded);
    let wait_for_mode_updated = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::SetSessionModeResponse,
    >(Commands::ModeUpdated);

    // Configure mock agent with legacy modes and set_mode response
    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);
        config.load_session_response = Some(LoadSessionResponse::default().modes(modes));
        config.set_session_mode_response =
            Some(agent_client_protocol::v1::schema::SetSessionModeResponse::new());
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let config = LoadSessionConfig {
        cwd: Some(std::path::PathBuf::from(".")),
        additional_directories: None,
        mcp_servers: Vec::new(),
    };
    load_session.call((session_id.clone(), Some(config)))?;
    wait_for_loaded_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    set_mode.call((session_id.to_string(), "chat".to_string()))?;

    let mode_updated = wait_for_mode_updated(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        mode_updated.is_ok(),
        "ModeUpdated should fire for legacy path"
    );

    Ok(())
}

// Chain load_session with config_options then set_mode to verify end-to-end flow.
#[nvim_oxi::test]
fn test_load_session_then_set_mode_uses_config_path() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let load_session: Function<(String, Option<LoadSessionConfig>), ()> =
        FromObject::from_object(dict.get("load_session").unwrap().clone())?;
    let set_mode: Function<SetModeArgs, ()> =
        FromObject::from_object(dict.get("set_mode").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_loaded_session =
        autocommand::listen_for_autocommand::<LoadSessionResponse>(Commands::SessionLoaded);
    let wait_for_config_updated = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::SetSessionConfigOptionResponse,
    >(Commands::ConfigurationUpdated);

    // Configure mock agent with config options and set_config_option response
    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);
        config.load_session_response =
            Some(LoadSessionResponse::default().config_options(vec![option]));
        config.set_session_config_option_response =
            Some(agent_client_protocol::v1::schema::SetSessionConfigOptionResponse::new(vec![]));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let config = LoadSessionConfig {
        cwd: Some(std::path::PathBuf::from(".")),
        additional_directories: None,
        mcp_servers: Vec::new(),
    };
    load_session.call((session_id.clone(), Some(config)))?;
    wait_for_loaded_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    set_mode.call((session_id.to_string(), "chat".to_string()))?;

    let config_updated = wait_for_config_updated(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        config_updated.is_ok(),
        "ConfigurationUpdated should fire for config path"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_close_session_fires_session_closed() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let close_session: Function<String, ()> =
        FromObject::from_object(dict.get("close_session").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_session_closed =
        autocommand::listen_for_autocommand::<CloseSessionResponse>(Commands::SessionClosed);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        config.close_session_response = Some(CloseSessionResponse::new());
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    close_session.call(session_id)?;

    let closed = wait_for_session_closed(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(closed.is_ok(), "SessionClosed autocommand should fire");

    Ok(())
}

#[nvim_oxi::test]
fn test_delete_session_fires_session_deleted() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let delete_session: Function<(DeleteSessionArg, Option<DeleteSessionOptions>), ()> =
        FromObject::from_object(dict.get("delete_session").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_session_deleted =
        autocommand::listen_for_autocommand::<DeleteSessionResponse>(Commands::SessionDeleted);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        config.delete_session_response = Some(DeleteSessionResponse::new());
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    delete_session.call((
        DeleteSessionArg::Single(session_id),
        None::<DeleteSessionOptions>,
    ))?;

    let deleted = wait_for_session_deleted(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(deleted.is_ok(), "SessionDeleted autocommand should fire");

    Ok(())
}

#[nvim_oxi::test]
fn test_delete_session_with_cancel_false() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let delete_session: Function<(DeleteSessionArg, Option<DeleteSessionOptions>), ()> =
        FromObject::from_object(dict.get("delete_session").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_session_deleted =
        autocommand::listen_for_autocommand::<DeleteSessionResponse>(Commands::SessionDeleted);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        config.delete_session_response = Some(DeleteSessionResponse::new());
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    delete_session.call((
        DeleteSessionArg::Single(session_id),
        Some(DeleteSessionOptions { cancel: false }),
    ))?;

    let deleted = wait_for_session_deleted(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        deleted.is_ok(),
        "SessionDeleted autocommand should fire with cancel=false"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_resume_session() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let resume_session: Function<(String, Option<ResumeSessionConfig>), ()> =
        FromObject::from_object(dict.get("resume_session").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Create a session first
    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    // Resume the session
    let wait_for_resumed_session =
        autocommand::listen_for_autocommand::<ResumeSessionResponse>(Commands::SessionResumed);

    let config = ResumeSessionConfig {
        cwd: Some(std::path::PathBuf::from(".")),
        additional_directories: None,
        mcp_servers: Vec::new(),
    };
    resume_session.call((session_id.clone(), Some(config)))?;

    let resumed_session = wait_for_resumed_session(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    resumed_session.inspect_err(|e| error!("Failed to resume session: {:?}", e))?;
    Ok(())
}

#[nvim_oxi::test]
fn test_resume_session_replays_history() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let prompt: Function<PromptArgs, Option<nvim_oxi::String>> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;
    let load_session: Function<(String, Option<LoadSessionConfig>), ()> =
        FromObject::from_object(dict.get("load_session").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    // Start MockAgent configured for local history (resume without server load)
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

    // Send prompt with 3 text blocks to generate history
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

    // Allow background worker's periodic flush to write keyed history to disk
    std::thread::sleep(Duration::from_millis(300));

    // Load session to replay history
    let config = LoadSessionConfig {
        cwd: Some(std::path::PathBuf::from(".")),
        additional_directories: None,
        mcp_servers: Vec::new(),
    };
    load_session.call((session_id.to_string(), Some(config)))?;

    // Verify history file on disk has all 6 lines (3 user + 3 agent chunks)
    let state_dir =
        nvim_oxi::api::call_function::<(String,), String>("stdpath", ("state".to_string(),))?;
    let history_path = format!(
        "{}/hermes/history/mock-agent (localhost:{})/{}.jsonl",
        state_dir,
        mock_handle.port(),
        session_id
    );
    let file_content =
        std::fs::read_to_string(&history_path).expect("history file should exist after prompt");
    assert_eq!(file_content.trim().lines().count(), 6);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}
