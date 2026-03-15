//! Integration tests for utility UI functions
//!
//! NOTE: UI interaction tests are disabled because vim.ui.select() blocks execution
//! in the test environment. These tests verify the permission request identification
//! and UI window detection (without requiring user interaction).
//!

use crate::helpers::ui::{wait_for_floating_window, wait_for_some};
use agent_client_protocol::{
    PermissionOption, PermissionOptionId, PermissionOptionKind, RequestPermissionRequest,
    SessionId, ToolCallId, ToolCallUpdate, ToolCallUpdateFields,
};
use hermes::nvim::requests::{Request, Responder};
use hermes::utilities::ui::show_permission_ui;
use std::cell::RefCell;
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::debug;

fn create_permission_option(id: &str, name: &str) -> PermissionOption {
    PermissionOption::new(
        PermissionOptionId::new(id),
        name.to_string(),
        PermissionOptionKind::AllowOnce,
    )
}

#[allow(dead_code)]
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

#[ignore]
// these tests are not possible as of now, the show_permission_ui is blocking for some reason, once it's called no progress can be made in the test and it will timeout
#[tracing_test::traced_test]
#[nvim_oxi::test]
fn show_permission_ui_options_can_be_selected() -> nvim_oxi::Result<()> {
    let (sender, reciever) = tokio::sync::oneshot::channel::<String>();
    let options = vec![
        create_permission_option("opt-1", "Option 1"),
        create_permission_option("opt-2", "Option 2"),
    ];
    let thing1 = RefCell::new(Some(sender));
    let thing2 = RefCell::new(reciever);

    debug!("Showing permission UI with options: {:?}", options);
    let _ = show_permission_ui(&options, "Hello!", move |selection| {
        debug!("Calling back! selected: {}", selection);
        if let Some(other) = thing1.take() {
            other.send(selection.clone()).ok();
            debug!("Sending selection through channel: {}", selection);
        }
    });

    if let Some(window) = wait_for_floating_window(Duration::from_secs(5)) {
        debug!("Found floating window: {}", window);
    };

    debug!("Executing keys!");
    let keys = nvim_oxi::String::from("<CR>");
    let mode = nvim_oxi::String::from("x");
    nvim_oxi::api::feedkeys(&keys, &mode, false);
    debug!("Done executing keys!");

    // Select SECOND option (navigate down):
    // let keys = nvim_oxi::String::from("j<CR>");
    // let mode = nvim_oxi::String::from("x");
    // nvim_oxi::api::feedkeys(&keys, &mode, false);
    //
    // // Select THIRD option (navigate down twice):
    // let keys = nvim_oxi::String::from("jj<CR>");
    // let mode = nvim_oxi::String::from("x");
    // nvim_oxi::api::feedkeys(&keys, &mode, false);
    //
    // // CANCEL with Escape:
    // let keys = nvim_oxi::String::from("<Esc>");
    // let mode = nvim_oxi::String::from("x");
    // nvim_oxi::api::feedkeys(&keys, &mode, false);

    let result = wait_for_some(Duration::from_secs(5), || {
        debug!("Waiting for selection...");
        thing2.borrow_mut().try_recv().ok()
    })?;

    println!("Received selection: {}", result);

    // Verify this is identified as a permission request
    // assert!(
    //     request.is_permission_request(),
    //     "Should be a permission request"
    // );

    Ok(())
}

/// Test that Request::is_permission_request() correctly identifies non-permission requests
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
