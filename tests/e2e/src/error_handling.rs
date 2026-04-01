//! E2E tests for error handling in API functions
use hermes::{api::DisconnectArgs, nvim::hermes};
use nvim_oxi::{conversion::FromObject, Dictionary, Function};

fn create_func<A>(plugin: Dictionary, name: &str) -> Function<A, ()> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .unwrap_or_else(|_| panic!("Failed to create function for {}", name))
}

// Test cancel without connection - should log error and return Ok
#[nvim_oxi::test]
fn cancel_without_connection_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let cancel: Function<String, ()> = create_func(dict.clone(), "cancel");

    // Call cancel without connecting first - should log error but not crash
    let result = cancel.call("test-session-id".to_string());

    assert!(
        result.is_ok(),
        "cancel without connection should return Ok, not crash"
    );

    Ok(())
}

// Test authenticate without connection - should log error and return Ok
#[nvim_oxi::test]
fn authenticate_without_connection_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let authenticate: Function<String, ()> = create_func(dict.clone(), "authenticate");

    // Call authenticate without connecting first - should log error but not crash
    let result = authenticate.call("test-method-id".to_string());

    assert!(
        result.is_ok(),
        "authenticate without connection should return Ok, not crash"
    );

    Ok(())
}

// Test disconnect all without connection - should log error and return Ok
#[nvim_oxi::test]
fn disconnect_all_without_connection_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");

    // Call disconnect with All variant
    let result = disconnect.call(DisconnectArgs::All);

    assert!(
        result.is_ok(),
        "disconnect all without connection should return Ok, not crash"
    );

    Ok(())
}

// Test API error handling comprehensive test
#[nvim_oxi::test]
fn api_error_handling_test_suite() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    // Test various error conditions
    let cancel: Function<String, ()> = create_func(dict.clone(), "cancel");
    let authenticate: Function<String, ()> = create_func(dict.clone(), "authenticate");
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");

    // All should return Ok even in error conditions (no crashes)
    assert!(cancel.call("test-session".to_string()).is_ok());
    assert!(authenticate.call("test-auth".to_string()).is_ok());
    assert!(disconnect.call(DisconnectArgs::All).is_ok());

    Ok(())
}

// Test connect with invalid protocol option
#[nvim_oxi::test]
fn connect_with_invalid_protocol_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<(nvim_oxi::String, Option<Dictionary>), ()> =
        create_func(dict.clone(), "connect");

    // Create an options dict with invalid protocol type (number instead of string)
    let mut invalid_options = Dictionary::new();
    invalid_options.insert("protocol", 123i64); // Invalid: should be string

    // Should log error and return Ok
    let result = connect.call((nvim_oxi::String::from("test-agent"), Some(invalid_options)));

    assert!(
        result.is_ok(),
        "connect with invalid protocol should return Ok, not crash"
    );

    Ok(())
}

// Test connect with invalid command option
#[nvim_oxi::test]
fn connect_with_invalid_command_does_not_crash() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<(nvim_oxi::String, Option<Dictionary>), ()> =
        create_func(dict.clone(), "connect");

    // Create an options dict with invalid command type (number instead of string)
    let mut invalid_options = Dictionary::new();
    invalid_options.insert("command", 456i64); // Invalid: should be string

    // Should log error and return Ok
    let result = connect.call((nvim_oxi::String::from("test-agent"), Some(invalid_options)));

    assert!(
        result.is_ok(),
        "connect with invalid command should return Ok, not crash"
    );

    Ok(())
}
