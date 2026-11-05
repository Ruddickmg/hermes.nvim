use hermes::{
    acp::connection::Assistant,
    api::{ConnectionArgs, LogoutArgs},
    nvim::autocommands::Commands,
};
use nvim_oxi::{Array, Dictionary, Function, Object, conversion::FromObject};
use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent, test_helpers::connect_to_mock_agent},
};

#[nvim_oxi::test]
fn test_logout_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(connect_obj.clone())?;

    let logout_obj = dict.get("logout").expect("logout function not found");
    let logout: Function<LogoutArgs, ()> = FromObject::from_object(logout_obj.clone())?;

    let disconnect_obj = dict
        .get("disconnect")
        .expect("disconnect function not found");
    let disconnect: Function<hermes::api::DisconnectArgs, ()> =
        FromObject::from_object(disconnect_obj.clone())?;

    // Start mock agent for this test
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);

    connect_to_mock_agent(&connect, &mock_handle)?;
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let wait_for_logout = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::LogoutResponse,
    >(Commands::LoggedOut);

    // Logout should succeed
    logout.call(LogoutArgs::All).unwrap();

    let result = wait_for_logout(Duration::from_secs(TIMEOUT_IN_SECONDS));

    assert!(result.is_ok(), "logout should succeed");

    // Cleanup
    disconnect.call(hermes::api::DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_logout_single() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(connect_obj.clone())?;

    let logout_obj = dict.get("logout").expect("logout function not found");
    let logout: Function<LogoutArgs, ()> = FromObject::from_object(logout_obj.clone())?;

    let disconnect_obj = dict
        .get("disconnect")
        .expect("disconnect function not found");
    let disconnect: Function<hermes::api::DisconnectArgs, ()> =
        FromObject::from_object(disconnect_obj.clone())?;

    // Start mock agent for this test
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);

    connect_to_mock_agent(&connect, &mock_handle)?;
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let wait_for_logout = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::LogoutResponse,
    >(Commands::LoggedOut);

    // Logout single agent by name
    logout.call(LogoutArgs::Single(Assistant::from("mock-agent")))?;

    let result = wait_for_logout(Duration::from_secs(TIMEOUT_IN_SECONDS));

    assert!(result.is_ok(), "logout single by name should succeed");

    // Cleanup
    disconnect.call(hermes::api::DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_logout_multiple() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(connect_obj.clone())?;

    let logout_obj = dict.get("logout").expect("logout function not found");
    let logout: Function<LogoutArgs, ()> = FromObject::from_object(logout_obj.clone())?;

    let disconnect_obj = dict
        .get("disconnect")
        .expect("disconnect function not found");
    let disconnect: Function<hermes::api::DisconnectArgs, ()> =
        FromObject::from_object(disconnect_obj.clone())?;

    // Start mock agent for this test
    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let wait_for_init = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::InitializeResponse,
    >(Commands::ConnectionInitialized);

    connect_to_mock_agent(&connect, &mock_handle)?;
    let _ = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let wait_for_logout = autocommand::listen_for_autocommand::<
        agent_client_protocol::v1::schema::LogoutResponse,
    >(Commands::LoggedOut);

    // Logout multiple agents including the connected one
    logout.call(LogoutArgs::Multiple(vec![Assistant::from("mock-agent")]))?;

    let result = wait_for_logout(Duration::from_secs(TIMEOUT_IN_SECONDS));

    assert!(result.is_ok(), "logout multiple by name should succeed");

    // Cleanup
    disconnect.call(hermes::api::DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}

#[nvim_oxi::test]
fn test_logout_with_invalid_arg_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;
    let logout: Function<Object, ()> = FromObject::from_object(
        dict.get("logout")
            .expect("logout function not found")
            .clone(),
    )?;

    // Pass a number instead of expected nil/string/array
    let result = logout.call(Object::from(42i64));

    assert!(result.is_ok(), "logout should succeed with invalid arg");

    Ok(())
}

#[nvim_oxi::test]
fn test_logout_with_array_of_numbers_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;
    let logout: Function<Object, ()> = FromObject::from_object(
        dict.get("logout")
            .expect("logout function not found")
            .clone(),
    )?;

    // Pass array of numbers instead of expected array of strings
    let result = logout.call(Object::from(Array::from_iter(vec![
        Object::from(1i64),
        Object::from(2i64),
    ])));

    assert!(
        result.is_ok(),
        "logout should succeed with array of numbers"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_logout_with_boolean_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;
    let logout: Function<Object, ()> = FromObject::from_object(
        dict.get("logout")
            .expect("logout function not found")
            .clone(),
    )?;

    // Pass a boolean instead of expected nil/string/array
    let result = logout.call(Object::from(true));

    assert!(result.is_ok(), "logout should succeed with boolean arg");

    Ok(())
}

#[nvim_oxi::test]
fn test_logout_with_dictionary_succeeds() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes::nvim::hermes()?;
    let logout: Function<Object, ()> = FromObject::from_object(
        dict.get("logout")
            .expect("logout function not found")
            .clone(),
    )?;

    // Pass a dictionary (table) instead of expected nil/string/array
    let mut invalid_dict = Dictionary::new();
    invalid_dict.insert("key", "value");
    let result = logout.call(Object::from(invalid_dict));

    assert!(result.is_ok(), "logout should succeed with dictionary arg");

    Ok(())
}
