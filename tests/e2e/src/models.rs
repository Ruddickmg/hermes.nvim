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
fn test_setup_returns_models_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("models").is_some(),
        "models function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_models_returns_nil_when_no_session() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let models: Function<String, Option<()>> =
        FromObject::from_object(dict.get("models").unwrap().clone())?;

    let result = models.call("nonexistent-session".to_string());

    assert_eq!(
        result,
        Ok(None),
        "models should return nil when session not found"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_models_returns_legacy_models() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let models: Function<String, Option<()>> =
        FromObject::from_object(dict.get("models").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_models = autocommand::listen_for_autocommand::<Selection>(Commands::Models);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let _result = models.call(session_id);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    let selection = wait_for_models(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    assert_eq!(
        selection.options,
        vec![HermesOption {
            value: "gpt4".into(),
            name: "GPT-4".into(),
            description: None,
            group: None,
        }]
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_models_returns_config_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let models: Function<String, Option<()>> =
        FromObject::from_object(dict.get("models").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_models = autocommand::listen_for_autocommand::<Selection>(Commands::Models);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let _result = models.call(session_id);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    let selection = wait_for_models(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    assert_eq!(
        selection.options,
        vec![HermesOption {
            value: "gpt4".into(),
            name: "GPT-4".into(),
            description: None,
            group: None,
        }]
    );

    Ok(())
}
