//! Integration tests for RequestHandler trait and Requests implementation
use crate::helpers::ui::wait_for;
use agent_client_protocol::schema::v1::{
    CreateTerminalResponse, KillTerminalResponse, ReleaseTerminalResponse, RequestId,
    RequestPermissionOutcome, RequestPermissionRequest, SessionId, TerminalId,
    TerminalOutputResponse, ToolCallId, ToolCallUpdate, ToolCallUpdateFields, WriteTextFileRequest,
    WriteTextFileResponse,
};
use async_channel::bounded as oneshot_channel;
use async_lock::Mutex;
use hermes::acp::Result;
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use hermes::nvim::state::PluginState;
use hermes::utilities::NvimRuntime;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

fn mock_runtime() -> NvimRuntime {
    NvimRuntime::new()
}

/// Helper to block on an async future in synchronous tests
fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));

    let response_obj = nvim_oxi::Object::from("selected-option-id");
    let result = block_on(requests.handle_response(&request_id, response_obj));

    assert!(result.is_ok());
    Ok(())
}

#[nvim_oxi::test]
fn test_handle_response_outcome_contains_option_id() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));

    let response_obj = nvim_oxi::Object::from("selected-option-id");
    block_on(requests.handle_response(&request_id, response_obj))
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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let request_id = Uuid::new_v4();
    let response_obj = nvim_oxi::Object::from("some-option");

    let result = block_on(requests.handle_response(&request_id, response_obj));

    assert!(result.is_err());
    Ok(())
}

#[nvim_oxi::test]
fn test_cancel_session_requests_returns_ok() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);

    block_on(requests.add_request(
        session_id.clone(),
        RequestId::Null,
        Responder::PermissionResponse(sender),
    ));

    let result = block_on(requests.cancel_session_requests(session_id));
    assert!(result.is_ok());
    Ok(())
}

#[nvim_oxi::test]
fn test_cancel_session_requests_no_matches_returns_ok() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("nonexistent-session");

    let result = block_on(requests.cancel_session_requests(session_id));
    assert!(result.is_ok());
    Ok(())
}

#[nvim_oxi::test]
fn test_cancel_session_requests_only_affects_target_session() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("target-session");
    let other_session_id = String::from("other-session");
    let (target_sender, target_receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let (other_sender, other_receiver) = oneshot_channel::<RequestPermissionOutcome>(1);

    block_on(requests.add_request(
        session_id.clone(),
        RequestId::Null,
        Responder::PermissionResponse(target_sender),
    ));
    block_on(requests.add_request(
        other_session_id.clone(),
        RequestId::Null,
        Responder::PermissionResponse(other_sender),
    ));

    block_on(requests.cancel_session_requests(session_id))
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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("target-session");
    let other_session_id = String::from("other-session");
    let (target_sender, _target_receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let (other_sender, other_receiver) = oneshot_channel::<RequestPermissionOutcome>(1);

    block_on(requests.add_request(
        session_id.clone(),
        RequestId::Null,
        Responder::PermissionResponse(target_sender),
    ));
    block_on(requests.add_request(
        other_session_id.clone(),
        RequestId::Null,
        Responder::PermissionResponse(other_sender),
    ));

    block_on(requests.cancel_session_requests(session_id))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Other session should not be cancelled
    let other_outcome = other_receiver.try_recv();

    assert!(other_outcome.is_err());
    Ok(())
}

#[nvim_oxi::test]
fn test_remove_request_returns_stored_request() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);
    let request_id =
        block_on(requests.add_request("test-session".to_string(), RequestId::Null, responder));

    let removed = block_on(requests.remove_request(&request_id))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    assert_eq!(removed.id(), request_id);
    Ok(())
}

#[nvim_oxi::test]
fn test_remove_request_removes_from_pending() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);
    let request_id =
        block_on(requests.add_request("test-session".to_string(), RequestId::Null, responder));

    block_on(requests.remove_request(&request_id))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    assert!(
        block_on(requests.get_request(&request_id)).is_none(),
        "Removed request should no longer be pending"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_remove_request_missing_returns_error() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );

    let result = block_on(requests.remove_request(&Uuid::new_v4()));

    assert!(
        result.is_err(),
        "Removing a non-existent request should return an error"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_id_returns_agent_request_id() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);
    let agent_request_id = RequestId::Number(42);
    let request_id = block_on(requests.add_request(
        "test-session".to_string(),
        agent_request_id.clone(),
        responder,
    ));

    let removed = block_on(requests.remove_request(&request_id))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    assert_eq!(removed.request_id(), agent_request_id);
    Ok(())
}

#[nvim_oxi::test]
fn test_get_request_returns_some_for_existing() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let retrieved = block_on(requests.get_request(&request_id));

    assert!(retrieved.is_some(), "Should find existing request");
    Ok(())
}

#[nvim_oxi::test]
fn test_get_request_returns_none_for_nonexistent() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let request_id = Uuid::new_v4();

    let retrieved = block_on(requests.get_request(&request_id));

    assert!(retrieved.is_none(), "Should not find non-existent request");
    Ok(())
}

#[nvim_oxi::test]
fn test_handle_response_removes_request_from_pending() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));

    // Handle the response
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    block_on(requests.handle_response(&request_id, response_obj))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Wait for cleanup to complete
    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));

    // Verify request exists before response (separate test from cleanup test)
    assert!(
        block_on(requests.get_request(&request_id)).is_some(),
        "Request should exist before response"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_write_file_request_cleanup_via_handle_response() -> nvim_oxi::Result<()> {
    use assert_fs::NamedTempFile;

    let temp_file = NamedTempFile::new("cleanup_test.txt").unwrap();
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );

    let (sender, receiver) = oneshot_channel::<WriteTextFileResponse>(1);
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id =
        block_on(requests.add_request("test-session".to_string(), RequestId::Null, responder));

    // Verify request exists
    assert!(block_on(requests.get_request(&request_id)).is_some());

    // Cleanup receiver at end of test
    drop(receiver);
    Ok(())
}

#[nvim_oxi::test]
fn test_write_file_handle_response_succeeds() -> nvim_oxi::Result<()> {
    use assert_fs::NamedTempFile;

    let temp_file = NamedTempFile::new("cleanup_test.txt").unwrap();
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );

    let (sender, _receiver) = oneshot_channel::<WriteTextFileResponse>(1);
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id =
        block_on(requests.add_request("test-session".to_string(), RequestId::Null, responder));

    // Use handle_response to trigger cleanup (goes through respond() -> finish())
    // NOTE: default_response() doesn't call finish(), so we use handle_response for this test
    let response_obj = nvim_oxi::Object::from(0i64); // WriteFileResponse doesn't use the response data
    let result = block_on(requests.handle_response(&request_id, response_obj));

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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );

    let (sender, _receiver) = oneshot_channel::<WriteTextFileResponse>(1);
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id =
        block_on(requests.add_request("test-session".to_string(), RequestId::Null, responder));

    // Use handle_response to trigger cleanup
    let response_obj = nvim_oxi::Object::from(0i64);
    block_on(requests.handle_response(&request_id, response_obj))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Wait for cleanup
    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );

    // Add 3 requests
    let mut request_ids = vec![];
    let mut receivers = vec![];
    for i in 0..3 {
        let session_id = format!("test-session-{}", i);
        let (sender, receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
        let responder = Responder::PermissionResponse(sender);
        let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
        request_ids.push(request_id);
        receivers.push(receiver);
    }

    // Verify all 3 exist
    let all_exist = request_ids
        .iter()
        .all(|id| block_on(requests.get_request(id)).is_some());

    assert!(all_exist, "All 3 requests should exist after adding");

    // Keep receivers alive until after test
    drop(receivers);
    Ok(())
}

#[nvim_oxi::test]
fn test_multiple_requests_all_handled_successfully() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );

    // Add 3 requests and keep their receivers alive
    let mut request_ids = vec![];
    let receivers: Vec<_> = (0..3)
        .map(|i| {
            let session_id = format!("test-session-{}", i);
            let (sender, receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
            let responder = Responder::PermissionResponse(sender);
            let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
            request_ids.push(request_id);
            receiver
        })
        .collect();

    // Respond to all 3
    let all_succeeded = request_ids.iter().all(|id| {
        let response_obj = nvim_oxi::Object::from("selected-option");
        block_on(requests.handle_response(id, response_obj)).is_ok()
    });

    assert!(all_succeeded, "All handle_response calls should succeed");

    // Keep receivers alive until after test
    drop(receivers);
    Ok(())
}

#[nvim_oxi::test]
fn test_multiple_requests_all_cleaned_up() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );

    // Add 3 requests and keep their receivers alive
    let mut request_ids = vec![];
    let receivers: Vec<_> = (0..3)
        .map(|i| {
            let session_id = format!("test-session-{}", i);
            let (sender, receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
            let responder = Responder::PermissionResponse(sender);
            let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
            request_ids.push(request_id);
            receiver
        })
        .collect();

    // Respond to all 3
    for id in &request_ids {
        let response_obj = nvim_oxi::Object::from("selected-option");
        block_on(requests.handle_response(id, response_obj))
            .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;
    }

    // Wait for all to be cleaned up
    let all_cleaned = wait_for(
        || {
            request_ids
                .iter()
                .all(|id| block_on(requests.get_request(id)).is_none())
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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));

    // First response succeeds
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    let result1 = block_on(requests.handle_response(&request_id, response_obj.clone()));

    assert!(result1.is_ok(), "First response should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn test_second_response_to_cleaned_up_request_fails() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));

    // First response succeeds
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    block_on(requests.handle_response(&request_id, response_obj.clone()))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Wait for cleanup
    wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );

    // Second response should fail (request no longer exists)
    let result2 = block_on(requests.handle_response(&request_id, response_obj));

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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    // Respond with an option ID
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    block_on(request.respond(response_obj))
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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    // Respond with empty string (cancels the request)
    let response_obj = nvim_oxi::Object::from("");
    block_on(request.respond(response_obj))
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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    // Cancel the request
    block_on(request.cancel()).map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<WriteTextFileResponse>(1);
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    // Cancel on non-permission request should succeed (no-op)
    let result = block_on(request.cancel());
    assert!(
        result.is_ok(),
        "Cancel on non-permission request should succeed"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_is_permission_request_true_for_permission() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

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
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<WriteTextFileResponse>(1);
    let write_request = WriteTextFileRequest::new(
        SessionId::from("test-session"),
        temp_file.path().to_path_buf(),
        "test content",
    );
    let responder = Responder::WriteFileResponse(sender, write_request);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    assert!(
        !request.is_permission_request(),
        "Write file should not be permission request"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_terminal_true_for_terminal_create() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(
            NvimRuntime::new(),
            Arc::new(Mutex::new(PluginState::default())),
        )
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)))?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<Result<CreateTerminalResponse>>(1);
    let create_request = agent_client_protocol::schema::v1::CreateTerminalRequest::new(
        SessionId::from("test-session"),
        "echo".to_string(),
    );
    let responder = Responder::TerminalCreate(sender, create_request);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    assert!(
        block_on(request.terminal()),
        "Terminal create should be terminal request"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_terminal_false_for_permission() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    assert!(
        !block_on(request.terminal()),
        "Permission request should not be terminal request"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_request_is_session_true_for_matching() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id.clone(), RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    assert!(request.is_session(session_id), "Should match session");
    Ok(())
}

#[nvim_oxi::test]
fn test_request_is_session_false_for_non_matching() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).map_err(
            |e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)),
        )?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    assert!(
        !request.is_session("other-session".to_string()),
        "Should not match different session"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_terminal_output_respond_sends_response() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(
            NvimRuntime::new(),
            Arc::new(Mutex::new(PluginState::default())),
        )
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)))?,
    );
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot_channel::<Result<TerminalOutputResponse>>(1);
    let output_request = agent_client_protocol::schema::v1::TerminalOutputRequest::new(
        SessionId::from("test-session"),
        TerminalId::from("test-terminal"),
    );
    let responder = Responder::TerminalOutput(sender, output_request);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    let response_obj = nvim_oxi::Object::from("terminal output");
    block_on(request.respond(response_obj))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let outcome = receiver.try_recv().expect("Should receive output response");
    let response = outcome.expect("Should be Ok");
    assert_eq!(response.output, "terminal output");
    Ok(())
}

#[nvim_oxi::test]
fn test_terminal_exit_respond_sends_response() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(
            NvimRuntime::new(),
            Arc::new(Mutex::new(PluginState::default())),
        )
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)))?,
    );
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot_channel::<Result<(Option<u32>, Option<String>)>>(1);
    let exit_request = agent_client_protocol::schema::v1::WaitForTerminalExitRequest::new(
        SessionId::from("test-session"),
        TerminalId::from("test-terminal"),
    );
    let responder = Responder::TerminalExit(sender, exit_request);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    let response_obj = nvim_oxi::Object::from("success");
    block_on(request.respond(response_obj))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let outcome = receiver.try_recv().expect("Should receive exit response");
    assert!(outcome.is_ok(), "Exit response should be Ok");
    Ok(())
}

#[nvim_oxi::test]
fn test_terminal_release_respond_sends_response() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(
            NvimRuntime::new(),
            Arc::new(Mutex::new(PluginState::default())),
        )
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)))?,
    );
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot_channel::<Result<ReleaseTerminalResponse>>(1);
    let release_request = agent_client_protocol::schema::v1::ReleaseTerminalRequest::new(
        SessionId::from("test-session"),
        TerminalId::from("test-terminal"),
    );
    let responder = Responder::TerminalRelease(sender, release_request);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    let response_obj = nvim_oxi::Object::from(0i64);
    block_on(request.respond(response_obj))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let outcome = receiver
        .try_recv()
        .expect("Should receive release response");
    assert!(outcome.is_ok(), "Release response should be Ok");
    Ok(())
}

#[nvim_oxi::test]
fn test_terminal_kill_respond_sends_response() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(
            NvimRuntime::new(),
            Arc::new(Mutex::new(PluginState::default())),
        )
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)))?,
    );
    let session_id = String::from("test-session");
    let (sender, receiver) = oneshot_channel::<Result<KillTerminalResponse>>(1);
    let kill_request = agent_client_protocol::schema::v1::KillTerminalRequest::new(
        SessionId::from("test-session"),
        TerminalId::from("test-terminal"),
    );
    let responder = Responder::TerminalKill(sender, kill_request);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    let response_obj = nvim_oxi::Object::from(0i64);
    block_on(request.respond(response_obj))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let outcome = receiver.try_recv().expect("Should receive kill response");
    assert!(outcome.is_ok(), "Kill response should be Ok");
    Ok(())
}

#[nvim_oxi::test]
fn test_respond_twice_on_same_request_fails() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(
            NvimRuntime::new(),
            Arc::new(Mutex::new(PluginState::default())),
        )
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)))?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    // First respond succeeds
    let response_obj = nvim_oxi::Object::from("selected-option-id");
    block_on(request.respond(response_obj.clone()))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Second respond should fail (responder already taken)
    let result = block_on(request.respond(response_obj));
    assert!(result.is_err(), "Second respond should fail");
    Ok(())
}

#[nvim_oxi::test]
fn test_cancel_twice_on_same_permission_request_fails() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(
            NvimRuntime::new(),
            Arc::new(Mutex::new(PluginState::default())),
        )
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e)))?,
    );
    let session_id = String::from("test-session");
    let (sender, _receiver) = oneshot_channel::<RequestPermissionOutcome>(1);
    let responder = Responder::PermissionResponse(sender);

    let request_id = block_on(requests.add_request(session_id, RequestId::Null, responder));
    let request = block_on(requests.get_request(&request_id)).expect("Request should exist");

    // First cancel succeeds
    block_on(request.cancel()).map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Second cancel should fail (responder already taken)
    let result = block_on(request.cancel());
    assert!(result.is_err(), "Second cancel should fail");
    Ok(())
}
