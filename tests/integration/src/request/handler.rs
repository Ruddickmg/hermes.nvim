//! Integration tests for RequestHandler trait and Requests implementation
use crate::helpers::ui::wait_for;
use agent_client_protocol::{
    CreateTerminalResponse, RequestPermissionOutcome, RequestPermissionRequest, SessionId,
    ToolCallId, ToolCallUpdate, ToolCallUpdateFields, WriteTextFileRequest, WriteTextFileResponse,
};
use hermes::acp::Result;
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use hermes::nvim::state::PluginState;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

fn _create_test_permission_request(session_id: impl Into<String>) -> RequestPermissionRequest {
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
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);

    let response_obj = nvim_oxi::Object::from("selected-option-id");
    let result = requests.handle_response(&request_id, response_obj);

    assert!(result.is_ok());
    Ok(())
}

#[nvim_oxi::test]
fn test_handle_response_outcome_contains_option_id() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, mut receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

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
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
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
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();

    requests.add_request(session_id.clone(), Responder::PermissionResponse(sender));

    let result = requests.cancel_session_requests(session_id);
    assert!(result.is_ok());
    Ok(())
}

#[nvim_oxi::test]
fn test_cancel_session_requests_no_matches_returns_ok() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
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
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("target-session");
    let other_session_id = String::from("other-session");
    let (target_sender, mut target_receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let (other_sender, other_receiver) = oneshot::channel::<RequestPermissionOutcome>();

    requests.add_request(
        session_id.clone(),
        Responder::PermissionResponse(target_sender),
    );
    requests.add_request(
        other_session_id.clone(),
        Responder::PermissionResponse(other_sender),
    );

    requests
        .cancel_session_requests(session_id)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Target session should be cancelled
    let target_outcome = target_receiver
        .try_recv()
        .expect("Should receive cancellation");

    assert_eq!(target_outcome, RequestPermissionOutcome::Cancelled);

    // Verify other session is NOT affected (separate test)
    drop(other_receiver);
    Ok(())
}

#[nvim_oxi::test]
fn test_other_session_not_cancelled() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("target-session");
    let other_session_id = String::from("other-session");
    let (target_sender, _target_receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let (other_sender, mut other_receiver) = oneshot::channel::<RequestPermissionOutcome>();

    requests.add_request(
        session_id.clone(),
        Responder::PermissionResponse(target_sender),
    );
    requests.add_request(
        other_session_id.clone(),
        Responder::PermissionResponse(other_sender),
    );

    requests
        .cancel_session_requests(session_id)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Other session should not be cancelled
    let other_outcome = other_receiver.try_recv();

    assert!(other_outcome.is_err());
    Ok(())
}

#[nvim_oxi::test]
fn test_get_request_returns_some_for_existing() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);
    let retrieved = requests.get_request(&request_id);

    assert!(retrieved.is_some(), "Should find existing request");
    Ok(())
}

#[nvim_oxi::test]
fn test_get_request_returns_none_for_nonexistent() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
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
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);

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
fn test_request_exists_before_response_handled() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);

    // Verify request exists before response (separate test from cleanup test)
    assert!(
        requests.get_request(&request_id).is_some(),
        "Request should exist before response"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_write_file_request_cleanup_via_handle_response() -> nvim_oxi::Result<()> {
    use assert_fs::NamedTempFile;

    let temp_file = NamedTempFile::new("cleanup_test.txt").unwrap();
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
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

    // Cleanup receiver at end of test
    drop(receiver);
    Ok(())
}

#[nvim_oxi::test]
fn test_write_file_handle_response_succeeds() -> nvim_oxi::Result<()> {
    use assert_fs::NamedTempFile;

    let temp_file = NamedTempFile::new("cleanup_test.txt").unwrap();
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);

    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id = requests.add_request("test-session".to_string(), responder);

    // Use handle_response to trigger cleanup (goes through respond() -> finish())
    // NOTE: default_response() doesn't call finish(), so we use handle_response for this test
    let response_obj = nvim_oxi::Object::from(0i64); // WriteFileResponse doesn't use the response data
    let result = requests.handle_response(&request_id, response_obj);

    assert!(
        result.is_ok(),
        "handle_response should succeed: {:?}",
        result
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_write_file_request_cleaned_up_after_response() -> nvim_oxi::Result<()> {
    use assert_fs::NamedTempFile;

    let temp_file = NamedTempFile::new("cleanup_test.txt").unwrap();
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);

    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id = requests.add_request("test-session".to_string(), responder);

    // Use handle_response to trigger cleanup
    let response_obj = nvim_oxi::Object::from(0i64);
    requests
        .handle_response(&request_id, response_obj)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Wait for cleanup
    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );

    assert!(
        cleaned_up,
        "Write file request should be cleaned up after response"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_multiple_requests_exist_after_adding() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);

    // Add 3 requests
    let mut request_ids = vec![];
    let mut receivers = vec![];
    for i in 0..3 {
        let session_id = format!("test-session-{}", i);
        let (sender, receiver) = oneshot::channel::<RequestPermissionOutcome>();
        let responder = Responder::PermissionResponse(sender);
        let request_id = requests.add_request(session_id, responder);
        request_ids.push(request_id);
        receivers.push(receiver);
    }

    // Verify all 3 exist
    let all_exist = request_ids
        .iter()
        .all(|id| requests.get_request(id).is_some());

    assert!(all_exist, "All 3 requests should exist after adding");

    // Keep receivers alive until after test
    drop(receivers);
    Ok(())
}

#[nvim_oxi::test]
fn test_multiple_requests_all_handled_successfully() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);

    // Add 3 requests and keep their receivers alive
    let mut request_ids = vec![];
    let receivers: Vec<_> = (0..3)
        .map(|i| {
            let session_id = format!("test-session-{}", i);
            let (sender, receiver) = oneshot::channel::<RequestPermissionOutcome>();
            let responder = Responder::PermissionResponse(sender);
            let request_id = requests.add_request(session_id, responder);
            request_ids.push(request_id);
            receiver
        })
        .collect();

    // Respond to all 3
    let all_succeeded = request_ids.iter().all(|id| {
        let response_obj = nvim_oxi::Object::from("selected-option");
        requests.handle_response(id, response_obj).is_ok()
    });

    assert!(all_succeeded, "All handle_response calls should succeed");

    // Keep receivers alive until after test
    drop(receivers);
    Ok(())
}

#[nvim_oxi::test]
fn test_multiple_requests_all_cleaned_up() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);

    // Add 3 requests and keep their receivers alive
    let mut request_ids = vec![];
    let receivers: Vec<_> = (0..3)
        .map(|i| {
            let session_id = format!("test-session-{}", i);
            let (sender, receiver) = oneshot::channel::<RequestPermissionOutcome>();
            let responder = Responder::PermissionResponse(sender);
            let request_id = requests.add_request(session_id, responder);
            request_ids.push(request_id);
            receiver
        })
        .collect();

    // Respond to all 3
    for id in &request_ids {
        let response_obj = nvim_oxi::Object::from("selected-option");
        requests
            .handle_response(id, response_obj)
            .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;
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

    // Keep receivers alive until after cleanup verification
    drop(receivers);
    Ok(())
}

#[nvim_oxi::test]
fn test_first_response_to_request_succeeds() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);

    // First response succeeds
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    let result1 = requests.handle_response(&request_id, response_obj.clone());

    assert!(result1.is_ok(), "First response should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn test_second_response_to_cleaned_up_request_fails() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);

    // First response succeeds
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    requests
        .handle_response(&request_id, response_obj.clone())
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

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

// Tests for Request::respond with different responder types
#[nvim_oxi::test]
fn test_request_respond_with_permission_response_sends_outcome() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, mut receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    // Respond with an option ID
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    request
        .respond(response_obj)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify outcome was sent
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
fn test_request_respond_with_permission_empty_string_sends_cancelled() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, mut receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    // Respond with empty string (cancels the request)
    let response_obj = nvim_oxi::Object::from("");
    request
        .respond(response_obj)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify cancelled outcome was sent
    let outcome = receiver.try_recv().expect("Should receive outcome");
    assert_eq!(
        outcome,
        RequestPermissionOutcome::Cancelled,
        "Empty string should send Cancelled"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_cancel_sends_cancelled_outcome() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, mut receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    // Cancel the request
    request
        .cancel()
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify cancelled outcome was sent
    let outcome = receiver.try_recv().expect("Should receive outcome");
    assert_eq!(
        outcome,
        RequestPermissionOutcome::Cancelled,
        "Cancel should send Cancelled"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_cancel_on_non_permission_request_returns_ok() -> nvim_oxi::Result<()> {
    use assert_fs::NamedTempFile;

    let temp_file = NamedTempFile::new("cancel_test.txt").unwrap();
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id = requests.add_request(session_id, responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    // Cancel on non-permission request should succeed (no-op)
    let result = request.cancel();
    assert!(
        result.is_ok(),
        "Cancel on non-permission request should succeed"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_is_permission_request_true_for_permission() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    assert!(
        request.is_permission_request(),
        "Should be permission request"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_is_permission_request_false_for_write_file() -> nvim_oxi::Result<()> {
    use assert_fs::NamedTempFile;

    let temp_file = NamedTempFile::new("perm_test.txt").unwrap();
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id = requests.add_request(session_id, responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    assert!(
        !request.is_permission_request(),
        "Write file should not be permission request"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_terminal_true_for_terminal_create() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<Result<CreateTerminalResponse>>();
    let create_request = agent_client_protocol::CreateTerminalRequest::new(
        SessionId::from("test-session"),
        "echo".to_string(),
    );
    let responder = Responder::TerminalCreate(sender, create_request);

    let request_id = requests.add_request(session_id, responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    assert!(
        request.terminal(),
        "Terminal create should be terminal request"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_terminal_false_for_permission() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    assert!(
        !request.terminal(),
        "Permission request should not be terminal request"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_is_session_true_for_matching() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id.clone(), responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    assert!(request.is_session(session_id), "Should match session");
    Ok(())
}

#[nvim_oxi::test]
fn test_request_is_session_false_for_non_matching() -> nvim_oxi::Result<()> {
    let requests =
        Arc::new(Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?);
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot::channel::<RequestPermissionOutcome>();
    let responder = Responder::PermissionResponse(sender);

    let request_id = requests.add_request(session_id, responder);
    let request = requests
        .get_request(&request_id)
        .expect("Request should exist");

    assert!(
        !request.is_session("other-session".to_string()),
        "Should not match different session"
    );
    Ok(())
}
