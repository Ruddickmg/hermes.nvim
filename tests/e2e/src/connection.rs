use agent_client_protocol::v1::schema::{AuthenticateResponse, InitializeResponse};
use hermes::{
    api::{ConnectionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, Object, conversion::FromObject};
use pretty_assertions::assert_eq;
use std::time::Duration;
use tracing::warn;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand,
        mock_agent::MockAgent,
        test_helpers::{connect_to_mock_agent, connect_to_mock_agent_http},
    },
};

#[nvim_oxi::test]
fn test_setup_returns_connect_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("connect").is_some(),
        "connect function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
async fn test_connect_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(connect_obj.clone())?;

    // Start mock agent for this test
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    // Cleanup
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_initialization() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;

    let wait_for_response =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);

    // Start mock agent for this test
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    let response = wait_for_response(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    // Mock agent reports itself as "mock-agent"
    assert_eq!(
        response.agent_info.unwrap().name.to_lowercase().as_str(),
        "mock-agent"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_authenticate_flow() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let authenticate: Function<String, ()> =
        FromObject::from_object(dict.get("authenticate").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_authentication =
        autocommand::listen_for_autocommand::<AuthenticateResponse>(Commands::Authenticated);

    // Start mock agent for this test
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    let mut init_response = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    if let Some(auth_method) = init_response.auth_methods.pop() {
        authenticate.call(auth_method.id().to_string())?;
        let auth_response = wait_for_authentication(Duration::from_secs(TIMEOUT_IN_SECONDS));
        println!(
            "Authentication successful, received response: {:?}",
            auth_response
        );
        assert!(auth_response.is_ok(), "Authentication should succeed");
    } else {
        warn!("No authentication methods available from agent, skipping auth test");
    }

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_disconnect_with_invalid_arg_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let disconnect: Function<Object, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;

    // Pass a number instead of the expected string/array/nil
    let result = disconnect.call(Object::from(42i64));

    assert!(result.is_ok(), "disconnect should succeed with invalid arg");

    Ok(())
}

#[nvim_oxi::test]
fn test_connection_via_http() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);

    let agent = MockAgent::new();
    let mock_handle = MockAgent::start_websocket(agent).expect("Failed to start mock agent");

    connect_to_mock_agent_http(&connect, &mock_handle)?;

    let init_response = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        init_response
            .agent_info
            .unwrap()
            .name
            .to_lowercase()
            .as_str(),
        "mock-agent"
    );

    Ok(())
}

#[cfg(unix)]
#[nvim_oxi::test]
fn test_connection_via_unix_socket() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);

    let agent = MockAgent::new();
    let mock_handle = MockAgent::start_unix_socket(agent).expect("Failed to start mock agent");

    crate::utilities::test_helpers::connect_to_mock_agent_unix(&connect, &mock_handle)?;

    let init_response = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        init_response
            .agent_info
            .unwrap()
            .name
            .to_lowercase()
            .as_str(),
        "mock-agent"
    );

    Ok(())
}
