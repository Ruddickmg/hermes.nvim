use hermes::{acp::connection::Assistant, api::ConnectionArgs, nvim::autocommands::Commands};
use nvim_oxi::{Array, Dictionary, Function, Object, conversion::FromObject};
use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent, test_helpers::connect_to_mock_agent},
};

#[nvim_oxi::test]
fn test_disconnect_all() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(
        dict.get("connect")
            .expect("connect function not found")
            .clone(),
    )?;

    let disconnect: Function<hermes::api::DisconnectArgs, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Start mock agent for this test
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    // Wait for initialization before disconnecting
    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Disconnect All should succeed
    disconnect.call(hermes::api::DisconnectArgs::All)?;

    // Cleanup
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_disconnect_single() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(
        dict.get("connect")
            .expect("connect function not found")
            .clone(),
    )?;

    let disconnect: Function<hermes::api::DisconnectArgs, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Start mock agent for this test
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    // Wait for initialization before disconnecting
    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Disconnect single agent by name
    disconnect.call(hermes::api::DisconnectArgs::Single(Assistant::from(
        "mock-agent",
    )))?;

    // Cleanup
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_disconnect_multiple() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(
        dict.get("connect")
            .expect("connect function not found")
            .clone(),
    )?;

    let disconnect: Function<hermes::api::DisconnectArgs, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Start mock agent for this test
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    connect_to_mock_agent(&connect, &mock_handle)?;

    // Wait for initialization before disconnecting
    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Disconnect multiple agents including the connected one
    disconnect.call(hermes::api::DisconnectArgs::Multiple(vec![
        Assistant::from("mock-agent"),
    ]))?;

    // Cleanup
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_disconnect_with_number_arg_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;
    let disconnect: Function<Object, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Pass a number instead of expected nil/string/array
    let result = disconnect.call(Object::from(42i64));

    assert!(result.is_ok(), "disconnect should succeed with invalid arg");

    Ok(())
}

#[nvim_oxi::test]
fn test_disconnect_with_array_of_numbers_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;
    let disconnect: Function<Object, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Pass array of numbers instead of expected array of strings
    let result = disconnect.call(Object::from(Array::from_iter(vec![
        Object::from(1i64),
        Object::from(2i64),
    ])));

    assert!(
        result.is_ok(),
        "disconnect should succeed with array of numbers"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_disconnect_with_boolean_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;
    let disconnect: Function<Object, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Pass a boolean instead of expected nil/string/array
    let result = disconnect.call(Object::from(true));

    assert!(result.is_ok(), "disconnect should succeed with boolean arg");

    Ok(())
}

#[nvim_oxi::test]
fn test_disconnect_with_dictionary_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;
    let disconnect: Function<Object, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function not found")
            .clone(),
    )?;

    // Pass a dictionary (table) instead of expected nil/string/array
    let mut invalid_dict = Dictionary::new();
    invalid_dict.insert("key", "value");
    let result = disconnect.call(Object::from(invalid_dict));

    assert!(
        result.is_ok(),
        "disconnect should succeed with dictionary arg"
    );

    Ok(())
}
