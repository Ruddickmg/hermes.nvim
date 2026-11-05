use std::time::Duration;

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
    acp::session_info::{HermesOption, Selection},
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};

#[nvim_oxi::test]
fn test_setup_returns_modes_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("modes").is_some(),
        "modes function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_modes_returns_nil_when_no_session() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let modes: Function<String, Option<()>> =
        FromObject::from_object(dict.get("modes").unwrap().clone())?;

    let result = modes.call("nonexistent-session".to_string());

    assert_eq!(
        result,
        Ok(None),
        "modes should return nil when session not found"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_modes_returns_legacy_modes() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let modes: Function<String, Option<()>> =
        FromObject::from_object(dict.get("modes").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_modes = autocommand::listen_for_autocommand::<Selection>(Commands::Modes);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);
        config.new_session_response = NewSessionResponse::new(generate_session_id()).modes(modes);
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let _result = modes.call(session_id);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    let selection = wait_for_modes(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    assert_eq!(
        selection.options,
        vec![HermesOption {
            value: "chat".into(),
            name: "Chat".into(),
            description: None,
            group: None,
        }]
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_modes_returns_config_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let modes: Function<String, Option<()>> =
        FromObject::from_object(dict.get("modes").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_modes = autocommand::listen_for_autocommand::<Selection>(Commands::Modes);

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
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let _result = modes.call(session_id);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    let selection = wait_for_modes(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    assert_eq!(
        selection.options,
        vec![HermesOption {
            value: "chat".into(),
            name: "Chat".into(),
            description: None,
            group: None,
        }]
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_modes_returns_grouped_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let modes: Function<String, Option<()>> =
        FromObject::from_object(dict.get("modes").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_modes = autocommand::listen_for_autocommand::<Selection>(Commands::Modes);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let group = agent_client_protocol::v1::schema::SessionConfigSelectGroup::new(
            "my-group",
            "My Group",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        );
        let option = SessionConfigOption::new(
            "mode",
            "Mode",
            agent_client_protocol::v1::schema::SessionConfigKind::Select(
                agent_client_protocol::v1::schema::SessionConfigSelect::new("chat", vec![group]),
            ),
        )
        .category(SessionConfigOptionCategory::Mode);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let _result = modes.call(session_id);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    let selection = wait_for_modes(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    assert_eq!(
        selection.options,
        vec![HermesOption {
            value: "chat".into(),
            name: "Chat".into(),
            description: None,
            group: Some("My Group".into()),
        }]
    );

    Ok(())
}
