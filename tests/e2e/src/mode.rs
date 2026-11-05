use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand, mock_agent::MockAgent, mock_config::generate_session_id,
        test_helpers::connect_to_mock_agent,
    },
};
use agent_client_protocol::v1::schema::{
    InitializeResponse, NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOption, SessionMode, SessionModeState,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, SetModeArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use std::time::Duration;

#[nvim_oxi::test]
fn test_setup_returns_set_mode_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("set_mode").is_some(),
        "set_mode function should be registered"
    );

    Ok(())
}

// Mock agent doesn't support session modes (returns None for modes),
// so set_mode should do nothing and return Ok when session_info has no modes entry.
#[nvim_oxi::test]
fn test_set_mode_no_modes_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_mode: Function<SetModeArgs, ()> =
        FromObject::from_object(dict.get("set_mode").unwrap().clone())?;

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

    // session_info has no modes entry, so set_mode does nothing and returns Ok
    let result = set_mode.call((session_id.to_string(), "test-mode".to_string()));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        result,
        Ok(()),
        "set_mode should return Ok when no modes are configured"
    );

    Ok(())
}

// Configure mock agent to return legacy session.modes.
// set_mode should call connection.set_mode(...) which the mock agent handles.
#[nvim_oxi::test]
fn test_set_mode_with_legacy_modes() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_mode: Function<SetModeArgs, ()> =
        FromObject::from_object(dict.get("set_mode").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    // Configure mock agent with legacy modes
    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);
        config.new_session_response = NewSessionResponse::new(generate_session_id()).modes(modes);
        config.set_session_mode_response =
            Some(agent_client_protocol::v1::schema::SetSessionModeResponse::new());
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    let result = set_mode.call((session_id.to_string(), "chat".to_string()));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(result, Ok(()), "set_mode with legacy modes should succeed");

    Ok(())
}

// Configure mock agent to return new config_options with Mode category.
// set_mode should call connection.set_config_option(...) which the mock agent handles.
#[nvim_oxi::test]
fn test_set_mode_with_config_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_mode: Function<SetModeArgs, ()> =
        FromObject::from_object(dict.get("set_mode").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    // Configure mock agent with new config options path
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
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
        config.set_session_config_option_response =
            Some(agent_client_protocol::v1::schema::SetSessionConfigOptionResponse::new(vec![]));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    let result = set_mode.call((session_id.to_string(), "chat".to_string()));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        result,
        Ok(()),
        "set_mode with config options should succeed"
    );

    Ok(())
}
