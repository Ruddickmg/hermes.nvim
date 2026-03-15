//! Integration tests for RequestHandler trait and Requests implementation
use crate::helpers::ui::wait_for;
use agent_client_protocol::{
    RequestPermissionOutcome, RequestPermissionRequest, SessionId, ToolCallId, ToolCallUpdate,
    ToolCallUpdateFields, WriteTextFileRequest, WriteTextFileResponse,
};
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::time::Duration;
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

#[nvim_oxi::test]
fn test_handle_response_removes_request_from_pending() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder =
        Responder::PermissionResponse(sender, create_test_permission_request("test-session"));

    let request_id = requests.add_request(session_id, responder);

    // Verify request exists before response
    assert!(
        requests.get_request(&request_id).is_some(),
        "Request should exist before response"
    );

    // Handle the response
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    requests
        .handle_response(&request_id, response_obj)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Wait for cleanup to complete
    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );

    assert!(
        cleaned_up,
        "Request should be removed from pending after response"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_write_file_request_cleanup() -> nvim_oxi::Result<()> {
    use assert_fs::NamedTempFile;

    let temp_file = NamedTempFile::new("cleanup_test.txt").unwrap();
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);

    let (sender, receiver) = oneshot::channel::<WriteTextFileResponse>();
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id = requests.add_request("test-session".to_string(), responder);

    // Verify request exists
    assert!(requests.get_request(&request_id).is_some());

    // Use handle_response to trigger cleanup (goes through respond() -> finish())
    // NOTE: default_response() doesn't call finish(), so we use handle_response for this test
    let response_obj = nvim_oxi::Object::from(0i64); // WriteFileResponse doesn't use the response data
    let result = requests.handle_response(&request_id, response_obj);
    assert!(
        result.is_ok(),
        "handle_response should succeed: {:?}",
        result
    );

    // Wait for cleanup
    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );

    assert!(
        cleaned_up,
        "Write file request should be cleaned up after response"
    );

    // Now we can drop the receiver
    drop(receiver);
    Ok(())
}

#[nvim_oxi::test]
fn test_multiple_requests_cleanup() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);

    // Add 3 requests and keep their receivers alive
    let mut request_ids = vec![];
    let mut receivers = vec![];
    for i in 0..3 {
        let session_id = format!("test-session-{}", i);
        let (sender, receiver) = oneshot::channel::<RequestPermissionOutcome>();
        let responder =
            Responder::PermissionResponse(sender, create_test_permission_request(&session_id));
        let request_id = requests.add_request(session_id, responder);
        request_ids.push(request_id);
        receivers.push(receiver); // Keep receiver alive
    }

    // Verify all 3 exist
    for id in &request_ids {
        assert!(
            requests.get_request(id).is_some(),
            "Request {} should exist",
            id
        );
    }

    // Respond to all 3
    for id in &request_ids {
        let response_obj = nvim_oxi::Object::from("selected-option");
        let result = requests.handle_response(id, response_obj);
        assert!(
            result.is_ok(),
            "handle_response should succeed for {}: {:?}",
            id,
            result
        );
    }

    // Wait for all to be cleaned up
    let all_cleaned = wait_for(
        || {
            request_ids
                .iter()
                .all(|id| requests.get_request(id).is_none())
        },
        Duration::from_millis(1000),
    );

    assert!(
        all_cleaned,
        "All 3 requests should be cleaned up after responses"
    );

    // Now we can drop all receivers
    drop(receivers);
    Ok(())
}

#[nvim_oxi::test]
fn test_responded_request_cannot_be_found() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new().map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder =
        Responder::PermissionResponse(sender, create_test_permission_request("test-session"));

    let request_id = requests.add_request(session_id, responder);

    // First response succeeds
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    let result1 = requests.handle_response(&request_id, response_obj.clone());
    assert!(result1.is_ok(), "First response should succeed");

    // Wait for cleanup
    wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );

    // Second response should fail (request no longer exists)
    let result2 = requests.handle_response(&request_id, response_obj);
    assert!(
        result2.is_err(),
        "Second response should fail - request cleaned up"
    );

    // Now drop the receiver
    drop(receiver);
    Ok(())
}
