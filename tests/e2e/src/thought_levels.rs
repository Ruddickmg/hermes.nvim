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
    SessionConfigSelectOption,
};
use hermes::{
    acp::session_info::{HermesOption, Selection},
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};

#[nvim_oxi::test]
fn test_setup_returns_thought_levels_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("thought_levels").is_some(),
        "thought_levels function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_thought_levels_returns_nil_when_no_session() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let thought_levels: Function<String, Option<()>> =
        FromObject::from_object(dict.get("thought_levels").unwrap().clone())?;

    let result = thought_levels.call("nonexistent-session".to_string());

    assert_eq!(
        result,
        Ok(None),
        "thought_levels should return nil when session not found"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_thought_levels_returns_config_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let thought_levels: Function<String, Option<()>> =
        FromObject::from_object(dict.get("thought_levels").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_thought_levels =
        autocommand::listen_for_autocommand::<Selection>(Commands::ThoughtLevels);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "thought_level",
            "Thought Level",
            "low",
            vec![
                SessionConfigSelectOption::new("low", "Low"),
                SessionConfigSelectOption::new("medium", "Medium"),
            ],
        )
        .category(SessionConfigOptionCategory::ThoughtLevel);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let _result = thought_levels.call(session_id);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    let selection = wait_for_thought_levels(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    assert_eq!(
        selection.options,
        vec![
            HermesOption {
                value: "low".into(),
                name: "Low".into(),
                description: None,
                group: None,
            },
            HermesOption {
                value: "medium".into(),
                name: "Medium".into(),
                description: None,
                group: None,
            },
        ]
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_thought_level_updated_fires_after_set() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_thought_level: Function<(String, String), ()> =
        FromObject::from_object(dict.get("set_thought_level").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_thought_level_updated =
        autocommand::listen_for_autocommand::<HermesOption>(Commands::ThoughtLevelUpdated);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "thought_level",
            "Thought Level",
            "low",
            vec![
                SessionConfigSelectOption::new("low", "Low"),
                SessionConfigSelectOption::new("medium", "Medium"),
            ],
        )
        .category(SessionConfigOptionCategory::ThoughtLevel);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
        let response_option = SessionConfigOption::select(
            "thought_level",
            "Thought Level",
            "medium",
            vec![
                SessionConfigSelectOption::new("low", "Low"),
                SessionConfigSelectOption::new("medium", "Medium"),
            ],
        )
        .category(SessionConfigOptionCategory::ThoughtLevel);
        config.set_session_config_option_response = Some(
            agent_client_protocol::v1::schema::SetSessionConfigOptionResponse::new(vec![
                response_option,
            ]),
        );
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    set_thought_level.call((session_id, "medium".to_string()))?;

    let thought_level_updated =
        wait_for_thought_level_updated(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        thought_level_updated,
        HermesOption {
            value: "medium".into(),
            name: "Medium".into(),
            description: None,
            group: None,
        }
    );

    Ok(())
}
