//! Integration tests for Responder::ReadFileResponse
//!
//! Each test verifies exactly ONE behavior with exactly ONE assertion.
//! Setup code and .expect() calls don't count as assertions.
use crate::helpers::ui::wait_for;
use agent_client_protocol::{ReadTextFileRequest, ReadTextFileResponse, SessionId};
use assert_fs::NamedTempFile;
use assert_fs::prelude::*;
use async_channel::bounded as oneshot_channel;
use async_lock::Mutex;
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use hermes::nvim::state::PluginState;
use hermes::utilities::NvimRuntime;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
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

fn create_read_request(
    path: &std::path::Path,
    start: Option<u32>,
    limit: Option<u32>,
) -> ReadTextFileRequest {
    let mut request =
        ReadTextFileRequest::new(SessionId::from("test-session"), PathBuf::from(path));
    request.line = start;
    request.limit = limit;
    request
}

fn create_file_with_lines(line_count: usize) -> (NamedTempFile, PathBuf) {
    let temp_file = NamedTempFile::new("test.txt").unwrap();
    let path = temp_file.path().to_path_buf();
    {
        let mut file = std::fs::File::create(&path).unwrap();
        for i in 0..line_count {
            writeln!(file, "line{}", i).unwrap();
        }
    }
    (temp_file, path)
}

fn setup_read_request(
    path: &std::path::Path,
    start: Option<u32>,
    limit: Option<u32>,
) -> (
    Arc<Requests>,
    Uuid,
    async_channel::Receiver<agent_client_protocol::Result<ReadTextFileResponse>>,
) {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).unwrap(),
    );
    let (sender, receiver) =
        oneshot_channel::<agent_client_protocol::Result<ReadTextFileResponse>>(1);
    let responder = Responder::ReadFileResponse(sender, create_read_request(path, start, limit));
    let request_id = block_on(requests.add_request("test-session".to_string(), responder));
    (requests, request_id, receiver)
}

// === Basic read operations ===

#[nvim_oxi::test]
fn read_file_default_response_succeeds() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, _receiver) = setup_read_request(&path, None, None);

    let result = block_on(requests.default_response(&request_id, serde_json::Value::Null));
    assert!(result.is_ok());

    Ok(())
}

#[nvim_oxi::test]
fn read_file_returns_all_content() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, mut receiver) = setup_read_request(&path, None, None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null))
        .expect("default_response should succeed");

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert_eq!(response.content, "line0\nline1\nline2\nline3\nline4\n");

    Ok(())
}

#[nvim_oxi::test]
fn read_file_creates_request_in_pending() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, _receiver) = setup_read_request(&path, None, None);

    assert!(block_on(requests.get_request(&request_id)).is_some());

    Ok(())
}

#[nvim_oxi::test]
fn read_file_gets_removed_from_pending() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, _receiver) = setup_read_request(&path, None, None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

// === Buffer reading ===

#[nvim_oxi::test]
fn read_file_prefers_buffer_over_disk() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("buffer_test.txt").unwrap();
    temp_file
        .write_str("original line 1\noriginal line 2\noriginal line 3\n")
        .unwrap();

    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;
    nvim_oxi::api::command("normal! gg0cwMODIFIED LINE")?;

    let (requests, request_id, mut receiver) = setup_read_request(temp_file.path(), None, None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert!(response.content.contains("MODIFIED LINE"));

    Ok(())
}

#[nvim_oxi::test]
fn read_file_buffer_cleanup_works() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("buffer_cleanup_test.txt").unwrap();
    temp_file.write_str("line1\nline2\n").unwrap();

    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    let (requests, request_id, _receiver) = setup_read_request(temp_file.path(), None, None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

// === Line ranges ===

#[nvim_oxi::test]
fn read_file_line_range_returns_correct_lines() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    // ACP uses 1-based indexing, so line=2, limit=4 means lines 2-4 (1-based)
    // After conversion to 0-based: start=1, end=3, so we get lines 1, 2 (indices 1..3)
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(2), Some(4));

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    // Lines 1, 2 (0-based) = line1, line2 (range is 1..3, exclusive end)
    assert_eq!(response.content, "line1\nline2\n");

    Ok(())
}

#[nvim_oxi::test]
fn read_file_line_range_cleanup_works() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, _receiver) = setup_read_request(&path, Some(1), Some(4));

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

#[nvim_oxi::test]
fn read_file_range_with_buffer_modifications() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("range_buffer_test.txt").unwrap();
    temp_file
        .write_str("line0\nline1\nline2\nline3\nline4\n")
        .unwrap();

    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;
    nvim_oxi::api::command("normal! ggj0cwMODIFIED1")?;
    nvim_oxi::api::command("normal! j0cwMODIFIED2")?;

    let (requests, request_id, mut receiver) =
        setup_read_request(temp_file.path(), Some(1), Some(4));

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert!(response.content.contains("MODIFIED1") && response.content.contains("MODIFIED2"));

    Ok(())
}

// === Buffer and File Consistency Tests ===

#[nvim_oxi::test]
fn read_file_buffer_and_file_return_same_content() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("consistency_test.txt").unwrap();
    temp_file
        .write_str("line0\nline1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\n")
        .unwrap();
    let path = temp_file.path().to_path_buf();

    // Read from disk (file not open in buffer)
    let (requests1, request_id1, mut receiver1) = setup_read_request(&path, Some(3), Some(7));
    block_on(requests1.default_response(&request_id1, serde_json::Value::Null)).ok();
    let file_response = receiver1
        .try_recv()
        .expect("Should receive file response")
        .expect("Should be Ok");

    // Read from buffer (file open in buffer)
    nvim_oxi::api::command(&format!("edit {}", path.display()))?;
    let (requests2, request_id2, mut receiver2) = setup_read_request(&path, Some(3), Some(7));
    block_on(requests2.default_response(&request_id2, serde_json::Value::Null)).ok();
    let buffer_response = receiver2
        .try_recv()
        .expect("Should receive buffer response")
        .expect("Should be Ok");

    // Both should return identical content (lines 2-6, 0-based)
    assert_eq!(
        file_response.content, buffer_response.content,
        "File and buffer paths should return identical content for the same 1-based range"
    );
    Ok(())
}

#[nvim_oxi::test]
fn read_file_buffer_and_file_edge_case_line_one() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("edge_case_test.txt").unwrap();
    temp_file.write_str("line0\nline1\nline2\n").unwrap();
    let path = temp_file.path().to_path_buf();

    // Read from disk with line=1 (should read from beginning)
    let (requests1, request_id1, mut receiver1) = setup_read_request(&path, Some(1), None);
    block_on(requests1.default_response(&request_id1, serde_json::Value::Null)).ok();
    let file_response = receiver1
        .try_recv()
        .expect("Should receive file response")
        .expect("Should be Ok");

    // Read from buffer with line=1
    nvim_oxi::api::command(&format!("edit {}", path.display()))?;
    let (requests2, request_id2, mut receiver2) = setup_read_request(&path, Some(1), None);
    block_on(requests2.default_response(&request_id2, serde_json::Value::Null)).ok();
    let buffer_response = receiver2
        .try_recv()
        .expect("Should receive buffer response")
        .expect("Should be Ok");

    // Both should start from line 0 (converted from 1-based line=1)
    assert_eq!(
        file_response.content, buffer_response.content,
        "Both paths should handle line=1 (converted to 0) consistently"
    );
    assert!(file_response.content.contains("line0"));
    Ok(())
}

#[nvim_oxi::test]
fn read_file_buffer_shows_modifications() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("modifications_test.txt").unwrap();
    temp_file.write_str("line0\nline1\nline2\n").unwrap();
    let path = temp_file.path().to_path_buf();

    // Open in buffer and modify first line
    nvim_oxi::api::command(&format!("edit {}", path.display()))?;
    nvim_oxi::api::command("normal! gg0cwMODIFIED")?;

    // Read from buffer (should see modifications)
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(1), Some(2));
    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();
    let buffer_response = receiver
        .try_recv()
        .expect("Should receive buffer response")
        .expect("Should be Ok");

    // Buffer should show modified content
    assert!(
        buffer_response.content.contains("MODIFIED"),
        "Buffer path should reflect unsaved modifications"
    );

    Ok(())
}

#[nvim_oxi::test]
fn read_file_file_ignores_modifications() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("modifications_test.txt").unwrap();
    temp_file.write_str("line0\nline1\nline2\n").unwrap();
    let path = temp_file.path().to_path_buf();

    // Open in buffer and modify first line (but don't save)
    nvim_oxi::api::command(&format!("edit {}", path.display()))?;
    nvim_oxi::api::command("normal! gg0cwMODIFIED")?;

    // Close buffer without saving
    nvim_oxi::api::command("bd!")?;

    // Read from disk (should see original content)
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(1), Some(2));
    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();
    let file_response = receiver
        .try_recv()
        .expect("Should receive file response")
        .expect("Should be Ok");

    // File should show original content, not modifications
    assert!(
        !file_response.content.contains("MODIFIED"),
        "File path should read from disk without modifications"
    );

    Ok(())
}

#[nvim_oxi::test]
fn read_file_buffer_and_file_apply_same_conversion() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("consistent_test.txt").unwrap();
    temp_file.write_str("line0\nline1\nline2\n").unwrap();
    let path = temp_file.path().to_path_buf();

    // Open in buffer but don't modify
    nvim_oxi::api::command(&format!("edit {}", path.display()))?;

    // Read from buffer with 1-based indexing
    let (requests1, request_id1, mut receiver1) = setup_read_request(&path, Some(2), Some(3));
    block_on(requests1.default_response(&request_id1, serde_json::Value::Null)).ok();
    let buffer_response = receiver1
        .try_recv()
        .expect("Should receive buffer response")
        .expect("Should be Ok");

    // Close buffer
    nvim_oxi::api::command("bd!")?;

    // Read from disk with same 1-based parameters
    let (requests2, request_id2, mut receiver2) = setup_read_request(&path, Some(2), Some(3));
    block_on(requests2.default_response(&request_id2, serde_json::Value::Null)).ok();
    let file_response = receiver2
        .try_recv()
        .expect("Should receive file response")
        .expect("Should be Ok");

    // Both should return identical content since both apply same 1-based conversion
    // line=2, limit=3 (1-based) → start=1, end=2 (0-based) → "line1"
    assert_eq!(
        buffer_response.content, file_response.content,
        "Buffer and file paths should return identical content with same 1-based parameters"
    );

    Ok(())
}

// === Error handling ===

#[nvim_oxi::test]
fn read_file_line_zero_returns_invalid_params_error() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(0), None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver.try_recv().expect("Should receive response");
    assert!(
        response.is_err(),
        "Line 0 should return invalid_params error"
    );

    Ok(())
}

#[nvim_oxi::test]
fn read_file_limit_zero_returns_invalid_params_error() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, mut receiver) = setup_read_request(&path, None, Some(0));

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver.try_recv().expect("Should receive response");
    assert!(
        response.is_err(),
        "Limit 0 should return invalid_params error"
    );

    Ok(())
}

#[nvim_oxi::test]
fn read_file_line_zero_cleanup_works() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, _receiver) = setup_read_request(&path, Some(0), None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );
    assert!(
        cleaned_up,
        "Request should be cleaned up after validation error"
    );

    Ok(())
}

#[nvim_oxi::test]
fn read_file_invalid_line_error_sent_to_agent() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(0), Some(3));

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver.try_recv().expect("Should receive response");
    let is_invalid_params = response.is_err();
    assert!(
        is_invalid_params,
        "Invalid line should send error response to agent through channel"
    );

    Ok(())
}

#[nvim_oxi::test]
fn read_file_missing_file_returns_error() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).unwrap(),
    );
    let (sender, mut receiver) = oneshot_channel::<Result<ReadTextFileResponse, _>>(1);
    let responder = Responder::ReadFileResponse(
        sender,
        create_read_request(PathBuf::from("/nonexistent/file.txt").as_path(), None, None),
    );
    let request_id = block_on(requests.add_request("test-session".to_string(), responder));

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver.try_recv().expect("Should receive response");
    assert!(response.is_err());

    Ok(())
}

#[nvim_oxi::test]
fn read_file_missing_file_cleanup_works() -> nvim_oxi::Result<()> {
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).unwrap(),
    );
    let (sender, _receiver) = async_channel::bounded::<Result<ReadTextFileResponse, _>>(1);
    let responder = Responder::ReadFileResponse(
        sender,
        create_read_request(PathBuf::from("/nonexistent/file.txt").as_path(), None, None),
    );
    let request_id = block_on(requests.add_request("test-session".to_string(), responder));

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

#[nvim_oxi::test]
fn read_file_empty_file_returns_empty_content() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("empty_test.txt").unwrap();
    temp_file.write_str("").unwrap();

    let (requests, request_id, mut receiver) = setup_read_request(temp_file.path(), None, None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert!(response.content.is_empty());

    Ok(())
}

#[nvim_oxi::test]
fn read_file_empty_file_cleanup_works() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("empty_cleanup_test.txt").unwrap();
    temp_file.write_str("").unwrap();

    let (requests, request_id, _receiver) = setup_read_request(temp_file.path(), None, None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

#[nvim_oxi::test]
fn read_file_start_beyond_length_returns_empty() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(3);
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(10), None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert!(response.content.is_empty());

    Ok(())
}

#[nvim_oxi::test]
fn read_file_start_beyond_length_cleanup_works() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(3);
    let (requests, request_id, _receiver) = setup_read_request(&path, Some(10), None);

    block_on(requests.default_response(&request_id, serde_json::Value::Null))
        .expect("Failed to send default response for out-of-range read request");

    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

// === Respond path ===

#[nvim_oxi::test]
fn read_file_respond_path_sends_custom_content() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).unwrap(),
    );
    let (sender, mut receiver) = oneshot_channel::<Result<ReadTextFileResponse, _>>(1);
    let responder = Responder::ReadFileResponse(sender, create_read_request(&path, None, None));
    let request_id = block_on(requests.add_request("test-session".to_string(), responder));

    let custom_content = "Custom content from user";
    let response_obj = nvim_oxi::Object::from(custom_content);
    block_on(requests.handle_response(&request_id, response_obj))
        .expect("Failed to handle custom content response");

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert_eq!(response.content, custom_content);

    Ok(())
}

#[nvim_oxi::test]
fn read_file_respond_path_cleanup_works() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(5);
    let requests = Arc::new(
        Requests::new(mock_runtime(), Arc::new(Mutex::new(PluginState::default()))).unwrap(),
    );
    let (sender, _receiver) = async_channel::bounded::<Result<ReadTextFileResponse, _>>(1);
    let responder = Responder::ReadFileResponse(sender, create_read_request(&path, None, None));
    let request_id = block_on(requests.add_request("test-session".to_string(), responder));

    let response_obj = nvim_oxi::Object::from("test content");
    block_on(requests.handle_response(&request_id, response_obj))
        .expect("Failed to handle response for cleanup verification");

    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

// === Large file ===

#[nvim_oxi::test]
fn read_file_large_returns_correct_range() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(300);
    // ACP uses 1-based indexing: line=101, limit=201 means lines 101-201 (1-based)
    // After conversion to 0-based: start=100, end=200, so we get lines 100-199
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(101), Some(201));

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    // Lines 100-199 (0-based) = line100 to line199
    assert!(response.content.contains("line100") && response.content.contains("line199"));

    Ok(())
}

#[nvim_oxi::test]
fn read_file_large_cleanup_works() -> nvim_oxi::Result<()> {
    let (_temp_file, path) = create_file_with_lines(300);
    let (requests, request_id, _receiver) = setup_read_request(&path, Some(101), Some(201));

    block_on(requests.default_response(&request_id, serde_json::Value::Null)).ok();

    let cleaned_up = wait_for(
        || block_on(requests.get_request(&request_id)).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}
