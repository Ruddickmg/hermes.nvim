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
    SessionConfigSelectOption, SetSessionConfigOptionResponse,
};
use hermes::{
    acp::session_info::ModelConfigOption,
    api::{ConfigureModelArgs, ConnectionArgs, CreateSessionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};

#[nvim_oxi::test]
fn test_setup_returns_configure_model_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("configure_model").is_some(),
        "configure_model function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_configure_model_success() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let configure_model: Function<ConfigureModelArgs, ()> =
        FromObject::from_object(dict.get("configure_model").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_model_updated = autocommand::listen_for_autocommand::<Vec<ModelConfigOption>>(
        Commands::ModelConfigurationUpdated,
    );

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "mc-1",
            "Model Config One",
            "val1",
            vec![
                SessionConfigSelectOption::new("val1", "Value 1"),
                SessionConfigSelectOption::new("val2", "Value 2"),
            ],
        )
        .category(SessionConfigOptionCategory::ModelConfig);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
        let response_option = SessionConfigOption::select(
            "mc-1",
            "Model Config One",
            "val2",
            vec![
                SessionConfigSelectOption::new("val1", "Value 1"),
                SessionConfigSelectOption::new("val2", "Value 2"),
            ],
        )
        .category(SessionConfigOptionCategory::ModelConfig);
        config.set_session_config_option_response =
            Some(SetSessionConfigOptionResponse::new(vec![response_option]));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    configure_model.call((
        session_id,
        hermes::api::ConfigureModelConfig {
            id: "mc-1".to_string(),
            value: "val2".to_string(),
        },
    ))?;

    let model_configs = wait_for_model_updated(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(model_configs[0].id, "mc-1");

    Ok(())
}

#[nvim_oxi::test]
fn test_configure_model_merges_instead_of_replacing() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let configure_model: Function<ConfigureModelArgs, ()> =
        FromObject::from_object(dict.get("configure_model").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_model_updated = autocommand::listen_for_autocommand::<Vec<ModelConfigOption>>(
        Commands::ModelConfigurationUpdated,
    );

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let opt1 = SessionConfigOption::select(
            "mc-1",
            "Model Config One",
            "val1",
            vec![
                SessionConfigSelectOption::new("val1", "Value 1"),
                SessionConfigSelectOption::new("val2", "Value 2"),
            ],
        )
        .category(SessionConfigOptionCategory::ModelConfig);
        let opt2 = SessionConfigOption::select(
            "mc-2",
            "Model Config Two",
            "val1",
            vec![SessionConfigSelectOption::new("val1", "Value 1")],
        )
        .category(SessionConfigOptionCategory::ModelConfig);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![opt1, opt2]);
        // Mock agent returns only the updated config option in the response
        let response_option = SessionConfigOption::select(
            "mc-1",
            "Model Config One",
            "val2",
            vec![
                SessionConfigSelectOption::new("val1", "Value 1"),
                SessionConfigSelectOption::new("val2", "Value 2"),
            ],
        )
        .category(SessionConfigOptionCategory::ModelConfig);
        config.set_session_config_option_response =
            Some(SetSessionConfigOptionResponse::new(vec![response_option]));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    configure_model.call((
        session_id,
        hermes::api::ConfigureModelConfig {
            id: "mc-1".to_string(),
            value: "val2".to_string(),
        },
    ))?;

    let model_configs = wait_for_model_updated(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(model_configs.len(), 2);

    Ok(())
}
