use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand, mock_agent::MockAgent, mock_config::generate_session_id,
        test_helpers::connect_to_mock_agent,
    },
};
use agent_client_protocol::schema::v1::{
    InitializeResponse, NewSessionResponse, SessionConfigKind, SessionConfigOption,
    SetSessionConfigOptionResponse,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, SetConfigOptionArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};

#[nvim_oxi::test]
fn test_setup_returns_config_options_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("config_options").is_some(),
        "config_options function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_setup_returns_set_config_option_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("set_config_option").is_some(),
        "set_config_option function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_config_options_returns_nil_when_no_session() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let config_options: Function<String, Option<()>> =
        FromObject::from_object(dict.get("config_options").unwrap().clone())?;

    let result = config_options.call("nonexistent-session".to_string());

    assert_eq!(
        result,
        Ok(None),
        "config_options should return nil when session not found"
    );

    Ok(())
}

/// Boolean options arrive on session creation and are surfaced with a flattened
/// `type` discriminator, so Lua can distinguish them from selects.
#[nvim_oxi::test]
fn test_config_options_returns_boolean_option_from_session_creation() -> Result<(), nvim_oxi::Error>
{
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let config_options: Function<String, Option<()>> =
        FromObject::from_object(dict.get("config_options").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_options =
        autocommand::listen_for_autocommand::<Vec<SessionConfigOption>>(Commands::ConfigOptions);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![
                SessionConfigOption::boolean("brave_mode", "Brave Mode", false),
            ]);
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let _result = config_options.call(session.session_id.to_string());
    let options = wait_for_options(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(options[0].id.to_string(), "brave_mode");

    Ok(())
}

#[nvim_oxi::test]
fn test_config_options_boolean_value_is_flattened_onto_option() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let config_options: Function<String, Option<()>> =
        FromObject::from_object(dict.get("config_options").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_options =
        autocommand::listen_for_autocommand::<Vec<SessionConfigOption>>(Commands::ConfigOptions);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![
                SessionConfigOption::boolean("brave_mode", "Brave Mode", true),
            ]);
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let _result = config_options.call(session.session_id.to_string());
    let options = wait_for_options(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(matches!(
        &options[0].kind,
        SessionConfigKind::Boolean(boolean) if boolean.current_value
    ));

    Ok(())
}

/// The boolean value must reach the agent as a boolean, not a string value id.
#[nvim_oxi::test]
fn test_set_config_option_sends_boolean_value_for_boolean_option() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_config_option: Function<SetConfigOptionArgs, Option<()>> =
        FromObject::from_object(dict.get("set_config_option").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_updated = autocommand::listen_for_autocommand::<SetSessionConfigOptionResponse>(
        Commands::ConfigurationUpdated,
    );

    let agent = MockAgent::new();
    let config_handle = agent.config().clone();
    {
        let mut config = config_handle.lock().unwrap();
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![
                SessionConfigOption::boolean("brave_mode", "Brave Mode", false),
            ]);
        config.set_session_config_option_response =
            Some(SetSessionConfigOptionResponse::new(vec![
                SessionConfigOption::boolean("brave_mode", "Brave Mode", true),
            ]));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let _result = set_config_option.call((
        session.session_id.to_string(),
        hermes::api::SetConfigOptionConfig {
            id: "brave_mode".to_string(),
            value: hermes::api::ConfigOptionValue::Boolean(true),
        },
    ));
    wait_for_updated(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let sent = config_handle
        .lock()
        .unwrap()
        .set_session_config_option_request
        .clone()
        .expect("set_config_option request should have been captured");

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        sent.value,
        agent_client_protocol::schema::v1::SessionConfigOptionValue::boolean(true)
    );

    Ok(())
}

/// Setting a boolean must update the stored snapshot so a later
/// `config_options()` reflects the agent's reported value.
#[nvim_oxi::test]
fn test_set_config_option_updates_stored_boolean_value() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_config_option: Function<SetConfigOptionArgs, Option<()>> =
        FromObject::from_object(dict.get("set_config_option").unwrap().clone())?;
    let config_options: Function<String, Option<()>> =
        FromObject::from_object(dict.get("config_options").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_updated = autocommand::listen_for_autocommand::<SetSessionConfigOptionResponse>(
        Commands::ConfigurationUpdated,
    );
    let wait_for_options =
        autocommand::listen_for_autocommand::<Vec<SessionConfigOption>>(Commands::ConfigOptions);

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        config.new_session_response =
            NewSessionResponse::new(generate_session_id()).config_options(vec![
                SessionConfigOption::boolean("brave_mode", "Brave Mode", false),
            ]);
        config.set_session_config_option_response =
            Some(SetSessionConfigOptionResponse::new(vec![
                SessionConfigOption::boolean("brave_mode", "Brave Mode", true),
            ]));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id.to_string();

    let _result = set_config_option.call((
        session_id.clone(),
        hermes::api::SetConfigOptionConfig {
            id: "brave_mode".to_string(),
            value: hermes::api::ConfigOptionValue::Boolean(true),
        },
    ));
    wait_for_updated(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let _result = config_options.call(session_id);
    let options = wait_for_options(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(matches!(
        &options[0].kind,
        SessionConfigKind::Boolean(boolean) if boolean.current_value
    ));

    Ok(())
}

/// Boolean config option support is always advertised, with no permission gate.
#[nvim_oxi::test]
fn test_boolean_capability_advertised() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);

    let agent = MockAgent::new();
    let config_handle = agent.config().clone();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let caps = config_handle
        .lock()
        .unwrap()
        .initialize_request
        .as_ref()
        .expect("initialize request should be captured")
        .client_capabilities
        .session
        .clone()
        .expect("session capabilities should be advertised")
        .config_options;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert!(
        caps.and_then(|options| options.boolean).is_some(),
        "boolean config option support should be advertised"
    );

    Ok(())
}
