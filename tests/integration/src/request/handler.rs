//! Integration tests for RequestHandler trait and Requests implementation
use agent_client_protocol::{
    RequestPermissionOutcome, RequestPermissionRequest, SessionId, ToolCallId, ToolCallUpdate,
    ToolCallUpdateFields,
};
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use pretty_assertions::assert_eq;
use std::sync::Arc;
use tokio::sync::oneshot;
use uuid::Uuid;

fn create_test_permission_request(session_id: impl Into<String>) -> RequestPermissionRequest {
    RequestPermissionRequest::new(
        SessionId::from(session_id.into()),
        ToolCallUpdate::new(
            ToolCallId::from("test-call-id"),
            ToolCallUpdateFields::default(),
        ),
        vec![],
    )
}

#[nvim_oxi::test]
fn test_handle_response_success() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder =
        Responder::PermissionResponse(sender, create_test_permission_request("test-session"));

    let request_id = requests.add_request(session_id, responder);

    let response_obj = nvim_oxi::Object::from("selected-option-id");
    let result = requests.handle_response(&request_id, response_obj);

    assert!(result.is_ok());
    Ok(())
}

#[nvim_oxi::test]
fn test_handle_response_outcome_contains_option_id() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, mut receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder =
        Responder::PermissionResponse(sender, create_test_permission_request("test-session"));

    let request_id = requests.add_request(session_id, responder);

    let response_obj = nvim_oxi::Object::from("selected-option-id");
    requests
        .handle_response(&request_id, response_obj)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let outcome = receiver.try_recv().expect("Should receive outcome");
    match outcome {
        RequestPermissionOutcome::Selected(selected) => {
            assert_eq!(selected.option_id.0.as_ref(), "selected-option-id");
        }
        _ => panic!("Expected Selected outcome"),
    }
    Ok(())
}

#[nvim_oxi::test]
fn test_handle_response_not_found_returns_error() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let request_id = Uuid::new_v4();
    let response_obj = nvim_oxi::Object::from("some-option");

    let result = requests.handle_response(&request_id, response_obj);

    assert!(result.is_err());
    Ok(())
}

#[nvim_oxi::test]
fn test_cancel_session_requests_returns_ok() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();

    requests.add_request(
        session_id.clone(),
        Responder::PermissionResponse(sender, create_test_permission_request("test-session")),
    );

    let result = requests.cancel_session_requests(session_id);
    assert!(result.is_ok());
    Ok(())
}

#[nvim_oxi::test]
fn test_cancel_session_requests_no_matches_returns_ok() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("nonexistent-session");

    let result = requests.cancel_session_requests(session_id);
    assert!(result.is_ok());
    Ok(())
}

#[nvim_oxi::test]
fn test_cancel_session_requests_only_affects_target_session() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("target-session");
    let other_session_id = String::from("other-session");
    let (target_sender, mut target_receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let (other_sender, mut other_receiver) = oneshot::channel::<RequestPermissionOutcome>();

    requests.add_request(
        session_id.clone(),
        Responder::PermissionResponse(
            target_sender,
            create_test_permission_request("target-session"),
        ),
    );
    requests.add_request(
        other_session_id.clone(),
        Responder::PermissionResponse(
            other_sender,
            create_test_permission_request("other-session"),
        ),
    );

    requests
        .cancel_session_requests(session_id)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Target session should be cancelled
    let target_outcome = target_receiver
        .try_recv()
        .expect("Should receive cancellation");
    assert_eq!(target_outcome, RequestPermissionOutcome::Cancelled);

    // Other session should not be cancelled
    let other_outcome = other_receiver.try_recv();
    assert!(other_outcome.is_err());
    Ok(())
}

#[nvim_oxi::test]
fn test_get_request_returns_some_for_existing() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder =
        Responder::PermissionResponse(sender, create_test_permission_request("test-session"));

    let request_id = requests.add_request(session_id, responder);
    let retrieved = requests.get_request(&request_id);

    assert!(retrieved.is_some(), "Should find existing request");
    Ok(())
}

#[nvim_oxi::test]
fn test_get_request_returns_none_for_nonexistent() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let request_id = Uuid::new_v4();

    let retrieved = requests.get_request(&request_id);

    assert!(retrieved.is_none(), "Should not find non-existent request");
    Ok(())
}
