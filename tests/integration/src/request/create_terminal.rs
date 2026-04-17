//! Integration tests for Responder::CreateTerminal
//!
//! Each test verifies exactly ONE behavior with exactly ONE assertion.
use agent_client_protocol::{CreateTerminalRequest, CreateTerminalResponse, SessionId};
use hermes::acp::Result;
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use hermes::nvim::state::PluginState;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

/// Helper to block on an async future in synchronous tests
fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

fn create_terminal_request(command: &str, args: Vec<String>) -> CreateTerminalRequest {
    let mut request = CreateTerminalRequest::new(SessionId::from("test-session"), command);
    request.args = args;
    request
}

fn setup_terminal_request(
    command: &str,
    args: Vec<String>,
) -> (
    Arc<Requests>,
    Uuid,
    oneshot::Receiver<Result<CreateTerminalResponse>>,
) {
    let requests = Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).unwrap());
    let (sender, receiver) = oneshot::channel::<Result<CreateTerminalResponse>>();
    let responder = Responder::TerminalCreate(sender, create_terminal_request(command, args));
    let request_id = block_on(requests.add_request("test-session".to_string(), responder));
    (requests, request_id, receiver)
}

// === Response handling tests ===

#[nvim_oxi::test]
fn create_terminal_cleanup_after_response() -> nvim_oxi::Result<()> {
    let (requests, request_id, _receiver) =
        setup_terminal_request("echo", vec!["hello".to_string()]);
    let terminal_id = "user-generated-terminal-id";

    // User responds with their generated terminal ID
    let response_obj = nvim_oxi::Object::from(terminal_id);
    block_on(requests.handle_response(&request_id, response_obj))
        .expect("Failed to handle response");

    // Wait for cleanup
    let cleaned_up = crate::helpers::ui::wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        std::time::Duration::from_millis(500),
    );

    assert!(
        cleaned_up,
        "CreateTerminal request should be cleaned up after response"
    );

    Ok(())
}

#[nvim_oxi::test]
fn create_terminal_user_handler_response_received() -> nvim_oxi::Result<()> {
    let (requests, request_id, mut receiver) =
        setup_terminal_request("echo", vec!["hello".to_string()]);
    let terminal_id = "my-custom-terminal-id-123";

    // User responds with their generated terminal ID
    let response_obj = nvim_oxi::Object::from(terminal_id);
    block_on(requests.handle_response(&request_id, response_obj))
        .expect("Failed to handle response");

    // Agent should receive the response with user-generated ID
    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");

    assert_eq!(
        response.terminal_id.to_string(),
        terminal_id,
        "Agent should receive CreateTerminalResponse with user-generated terminal ID"
    );

    Ok(())
}

#[nvim_oxi::test]
fn create_terminal_invalid_response_sends_error() -> nvim_oxi::Result<()> {
    let (requests, request_id, mut receiver) =
        setup_terminal_request("echo", vec!["hello".to_string()]);

    // Send invalid response (integer instead of string)
    let response_obj = nvim_oxi::Object::from(42i64);
    block_on(requests.handle_response(&request_id, response_obj))
        .expect("Failed to handle response");

    // Agent should receive an error
    let response = receiver.try_recv().expect("Should receive response");
    assert!(
        response.is_err(),
        "Agent should receive error for invalid terminal_id"
    );

    Ok(())
}
