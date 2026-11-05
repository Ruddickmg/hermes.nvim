use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand, mock_agent::MockAgent, mock_config::generate_session_id,
        test_helpers::connect_to_mock_agent,
    },
};
use agent_client_protocol::v1::schema::{
    InitializeResponse, NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOption,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, SetThoughtLevelArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use std::time::Duration;

#[nvim_oxi::test]
fn test_setup_returns_set_thought_level_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("set_thought_level").is_some(),
        "set_thought_level function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_set_thought_level_no_thought_levels_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_thought_level: Function<SetThoughtLevelArgs, ()> =
        FromObject::from_object(dict.get("set_thought_level").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    let result = set_thought_level.call((session_id.to_string(), "low".to_string()));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        result.is_ok(),
        "set_thought_level should return Ok when no thought levels are configured"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_set_thought_level_with_config_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_thought_level: Function<SetThoughtLevelArgs, ()> =
        FromObject::from_object(dict.get("set_thought_level").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "thought_level",
            "Thought Level",
            "low",
            vec![SessionConfigSelectOption::new("low", "Low")],
        )
        .category(SessionConfigOptionCategory::ThoughtLevel);
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

    let result = set_thought_level.call((session_id.to_string(), "low".to_string()));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        result.is_ok(),
        "set_thought_level with config options should succeed"
    );

    Ok(())
}
