//! Integration tests for the AutoCommand struct
//!
//! These tests run inside Neovim using the `#[nvim_oxi::test]` macro.

use agent_client_protocol::RequestPermissionOutcome;
use hermes::nvim::{
    autocommands::{AutoCommand, Commands},
    hermes,
    requests::{Requests, Responder},
};
use nvim_oxi::api::opts::CreateAutocmdOpts;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Helper to create a simple test autocommand listener
fn create_test_autocmd(command: &str) -> nvim_oxi::Result<u32> {
    // Note: hermes() already creates the "hermes" group, so we don't need to create it again
    let opts = CreateAutocmdOpts::builder()
        .patterns([command])
        .group("hermes")
        .command("echo 'test'")
        .build();

    let id = nvim_oxi::api::create_autocmd(["User"], &opts)?;

    Ok(id)
}

/// Convert hermes error to nvim_oxi error
fn to_nvim_error(e: hermes::acp::error::Error) -> nvim_oxi::Error {
    nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string()))
}

/// Test that AutoCommand::new creates a valid instance
#[nvim_oxi::test]
fn test_autocommand_new_creates_instance() -> nvim_oxi::Result<()> {
    // Initialize hermes to create the autocommand group
    let _ = hermes()?;

    let requests = Arc::new(Requests::new());
    let result = AutoCommand::new(requests);

    assert!(
        result.is_ok(),
        "AutoCommand::new should create a valid instance"
    );
    let _auto_command = result.map_err(to_nvim_error)?;

    Ok(())
}

/// Test that listener_attached returns false when no autocommand exists
#[nvim_oxi::test]
fn test_listener_attached_no_listener() -> nvim_oxi::Result<()> {
    // Initialize hermes to create the autocommand group
    let _ = hermes()?;

    let requests = Arc::new(Requests::new());
    let auto_command = AutoCommand::new(requests).map_err(to_nvim_error)?;

    // Use a pattern that doesn't have a listener
    let has_listener =
        futures::executor::block_on(auto_command.listener_attached("PermissionRequest"))
            .map_err(to_nvim_error)?;

    assert!(
        !has_listener,
        "Should return false when no listener is attached"
    );

    Ok(())
}

/// Test that listener_attached returns true when an autocommand exists
#[nvim_oxi::test]
fn test_listener_attached_with_listener() -> nvim_oxi::Result<()> {
    // Initialize hermes to create the autocommand group
    let _ = hermes()?;

    let requests = Arc::new(Requests::new());
    let auto_command = AutoCommand::new(requests).map_err(to_nvim_error)?;

    // Create an autocommand listener with a valid command pattern
    let autocmd_id = create_test_autocmd("PermissionRequest")?;

    // Verify immediately that we can see the autocmd
    let opts = nvim_oxi::api::opts::GetAutocmdsOpts::builder().build(); // No filters - get everything
    let autocmds = nvim_oxi::api::get_autocmds(&opts)?;
    let autocmds_vec: Vec<_> = autocmds.collect();

    if autocmds_vec.is_empty() {
        panic!(
            "No autocmds found at all! Created autocmd id: {:?}",
            autocmd_id
        );
    }

    // Print all autocmds to see what we have
    for (i, autocmd) in autocmds_vec.iter().enumerate() {
        eprintln!(
            "DEBUG[{}]: group_name={:?}, pattern='{}', event='{}'",
            i, autocmd.group_name, autocmd.pattern, autocmd.event
        );
    }

    // Check if any match what we expect
    let found = autocmds_vec.iter().any(|a| {
        let pattern_match = a.pattern == "PermissionRequest";
        let group_match = a
            .group_name
            .as_ref()
            .map(|g| g == "hermes")
            .unwrap_or(false);
        eprintln!(
            "DEBUG: Checking autocmd - pattern_match={}, group_match={}, pattern='{}', group={:?}",
            pattern_match, group_match, a.pattern, a.group_name
        );
        pattern_match && group_match
    });

    if !found {
        panic!(
            "Created autocmd 'PermissionRequest' in group 'hermes' not found among {} autocmds: {:?}",
            autocmds_vec.len(),
            autocmds_vec.iter().map(|a| (a.pattern.clone(), a.group_name.clone())).collect::<Vec<_>>()
        );
    }

    // Check that listener_attached returns true
    let has_listener =
        futures::executor::block_on(auto_command.listener_attached("PermissionRequest"))
            .map_err(to_nvim_error)?;

    assert!(has_listener, "Should return true when listener is attached");

    Ok(())
}

/// Test that execute_autocommand returns NoListenerAttached error when no listener exists
#[nvim_oxi::test]
fn test_execute_autocommand_no_listener_error() -> nvim_oxi::Result<()> {
    // Initialize hermes to create the autocommand group
    let _ = hermes()?;

    let requests = Arc::new(Requests::new());
    let auto_command = AutoCommand::new(requests).map_err(to_nvim_error)?;

    let test_data = serde_json::json!({"test": "data"});

    // Attempt to execute without a listener should fail
    // Use a valid command name that doesn't have a listener
    let result =
        futures::executor::block_on(auto_command.execute_autocommand("ToolCall", test_data));

    assert!(
        result.is_err(),
        "Should return error when no listener is attached"
    );

    // Verify it's the correct error type - NoListenerAttached
    match result {
        Err(hermes::acp::error::Error::NoListenerAttached(_)) => (), // Expected
        Err(e) => panic!("Expected NoListenerAttached error, got: {:?}", e),
        Ok(_) => panic!("Should have returned an error"),
    }

    Ok(())
}

/// Test execute_autocommand_request adds request_id and registers request
#[nvim_oxi::test]
fn test_execute_autocommand_request_adds_request_id() -> nvim_oxi::Result<()> {
    // Initialize hermes to create the autocommand group
    let _ = hermes()?;

    let requests = Arc::new(Requests::new());
    let auto_command = AutoCommand::new(requests.clone()).map_err(to_nvim_error)?;

    // Create an autocommand listener with a valid command
    create_test_autocmd("PermissionRequest")?;

    // Create a responder for testing
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let test_data = serde_json::json!({
        "test": "data",
        "session_id": "test-session"
    });

    // Execute request-style autocommand
    let result = futures::executor::block_on(auto_command.execute_autocommand_request(
        "test-session".to_string(),
        "PermissionRequest",
        test_data,
        responder,
    ));

    // Should succeed when listener exists
    assert!(result.is_ok(), "execute_autocommand_request should succeed");

    Ok(())
}

/// Test that execute_autocommand_request returns error when no listener
#[nvim_oxi::test]
fn test_execute_autocommand_request_no_listener_error() -> nvim_oxi::Result<()> {
    // Initialize hermes to create the autocommand group
    let _ = hermes()?;

    let requests = Arc::new(Requests::new());
    let auto_command = AutoCommand::new(requests).map_err(to_nvim_error)?;

    // Create a responder
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let test_data = serde_json::json!({"test": "data"});

    // Attempt to execute without a listener should fail
    // Use a valid command name that doesn't have a listener
    let result = futures::executor::block_on(auto_command.execute_autocommand_request(
        "test-session".to_string(),
        "ToolCall",
        test_data,
        responder,
    ));

    assert!(
        result.is_err(),
        "Should return error when no listener is attached"
    );

    Ok(())
}
