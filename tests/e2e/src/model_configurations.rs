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
    acp::session_info::ModelConfigOption,
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};

#[nvim_oxi::test]
fn test_setup_returns_model_configurations_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("model_configurations").is_some(),
        "model_configurations function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_model_configurations_returns_nil_when_no_session() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let model_configurations: Function<String, Option<()>> =
        FromObject::from_object(dict.get("model_configurations").unwrap().clone())?;

    let result = model_configurations.call("nonexistent-session".to_string());

    assert_eq!(
        result,
        Ok(None),
        "model_configurations should return nil when session not found"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_model_configurations_returns_config_options() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let model_configurations: Function<String, Option<()>> =
        FromObject::from_object(dict.get("model_configurations").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_model_configs = autocommand::listen_for_autocommand::<Vec<ModelConfigOption>>(
        Commands::ModelConfigurations,
    );

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        let option = SessionConfigOption::select(
            "mc-1",
            "Model Config One",
            "val1",
            vec![SessionConfigSelectOption::new("val1", "Value 1")],
        )
        .category(SessionConfigOptionCategory::ModelConfig);
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![option]);
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let _result = model_configurations.call(session_id);

    let model_configs = wait_for_model_configs(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(model_configs[0].id, "mc-1");

    Ok(())
}
