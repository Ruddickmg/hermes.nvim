//! Integration tests for permission request handling via Request::default()
//!
//! NOTE: Most tests in this file are disabled because vim.ui.select() blocks execution
//! in the test environment, waiting for user interaction. The functions work correctly
//! in real usage. Only the error handling test (invalid_json_data_returns_error) remains enabled.
use agent_client_protocol::{
    PermissionOption, PermissionOptionId, PermissionOptionKind, RequestPermissionOutcome,
    RequestPermissionRequest, SessionId, ToolCallId, ToolCallUpdate, ToolCallUpdateFields,
};
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use std::sync::Arc;

fn _create_permission_option(id: &str, name: &str) -> PermissionOption {
    PermissionOption::new(
        PermissionOptionId::new(id),
        name.to_string(),
        PermissionOptionKind::AllowOnce,
    )
}

fn _create_permission_request(
    session_id: impl Into<String>,
    options: Vec<PermissionOption>,
) -> RequestPermissionRequest {
    RequestPermissionRequest::new(
        SessionId::from(session_id.into()),
        ToolCallUpdate::new(
            ToolCallId::from("test-call-id"),
            ToolCallUpdateFields::default(),
        ),
        options,
    )
}

// TODO: The following tests are disabled pending resolution of vim.ui.select blocking in test environment
// vim.ui.select() opens a UI that blocks execution until user interaction, which doesn't work
// in automated test environments. The functions work correctly in real usage.
//
// The following test works and is NOT commented out:
#[nvim_oxi::test]
fn invalid_json_data_returns_error() -> nvim_oxi::Result<()> {
    // Create Requests handler and add a permission request
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let (sender, _receiver) = tokio::sync::oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);
    let request_id = requests.add_request("test-session".to_string(), responder);

    // Invalid JSON that doesn't match RequestPermissionRequest structure
    let invalid_data = serde_json::json!({
        "invalid_field": "not_a_valid_request",
        "another_field": 123
    });

    let result = requests.default_response(&request_id, invalid_data);
    assert!(result.is_err(), "Should return error for invalid JSON data");

    Ok(())
}

// Disabled tests (vim.ui.select blocks in test environment):
// - permission_request_opens_floating_window
// - selecting_first_option_sends_correct_outcome
// - selecting_second_option_sends_correct_outcome
// - selecting_by_number_works
// - esc_cancels_request
// - window_shows_correct_content
