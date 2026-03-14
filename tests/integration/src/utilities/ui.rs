//! Integration tests for utility UI functions
//!
//! NOTE: These tests verify the show_permission_ui callback mechanism without
//! opening the actual UI (which would block in tests).
//!
//! The utilities module is private, so we test through the public Request API.
use agent_client_protocol::{
    PermissionOption, PermissionOptionId, PermissionOptionKind, RequestPermissionOutcome,
    RequestPermissionRequest, SessionId, ToolCallId, ToolCallUpdate, ToolCallUpdateFields,
};
use hermes::nvim::requests::{Request, Responder};

fn create_permission_option(id: &str, name: &str) -> PermissionOption {
    PermissionOption::new(
        PermissionOptionId::new(id),
        name.to_string(),
        PermissionOptionKind::AllowOnce,
    )
}

fn create_permission_request(
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

// Test that Request::default() correctly identifies and routes permission requests
// This indirectly tests show_permission_ui through the public API
#[nvim_oxi::test]
fn test_permission_request_flow_starts() -> nvim_oxi::Result<()> {
    // Note: We can't fully test the UI flow because vim.ui.select blocks,
    // but we can verify the request is correctly identified as a permission request
    let (sender, _receiver) = tokio::sync::oneshot::channel::<RequestPermissionOutcome>();
    let options = vec![
        create_permission_option("opt-1", "Option 1"),
        create_permission_option("opt-2", "Option 2"),
    ];
    let responder =
        Responder::PermissionResponse(sender, create_permission_request("test-session", options));
    let request = Request::new("test-session".to_string(), responder);

    // Verify this is identified as a permission request
    assert!(
        request.is_permission_request(),
        "Should be a permission request"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_non_permission_request_not_permission() -> nvim_oxi::Result<()> {
    use agent_client_protocol::{SessionId, WriteTextFileRequest};
    use std::path::PathBuf;

    let (sender, _receiver) =
        tokio::sync::oneshot::channel::<agent_client_protocol::WriteTextFileResponse>();
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        PathBuf::from("/tmp/test.txt"),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);
    let request = Request::new("test-session".to_string(), responder);

    // Verify this is NOT a permission request
    assert!(
        !request.is_permission_request(),
        "Should not be a permission request"
    );

    Ok(())
}
