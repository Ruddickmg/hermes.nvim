//! Integration tests for Responder::WriteFileResponse via Request
use crate::helpers::ui::wait_for;
use agent_client_protocol::{SessionId, WriteTextFileRequest, WriteTextFileResponse};
use assert_fs::prelude::*;
use assert_fs::{NamedTempFile, TempDir};
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use hermes::nvim::state::PluginState;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};

/// Helper function to create a WriteTextFileRequest
fn create_write_request(path: &Path, content: &str) -> WriteTextFileRequest {
    WriteTextFileRequest::new(SessionId::from("test-session"), path, content)
}

#[nvim_oxi::test]
fn open_buffer_marked_modified() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test.txt").unwrap();
    temp_file.write_str("initial content").unwrap();

    // Open file in Neovim buffer
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Create Requests handler and add a write file request
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );
    let (sender, mut receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "updated content"),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    // Execute the request
    requests
        .default_response(&request_id, serde_json::Value::Null)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify buffer is marked as modified
    let buffer = nvim_oxi::api::list_bufs()
        .into_iter()
        .find(|b| b.get_name().map(|p| p == temp_file.path()).unwrap_or(false))
        .expect("Buffer should exist");

    let is_modified: bool = nvim_oxi::api::get_option_value::<bool>(
        "modified",
        &nvim_oxi::api::opts::OptionOpts::builder()
            .buffer(buffer.clone())
            .build(),
    )
    .expect("Should get modified option");

    assert!(
        is_modified,
        "Buffer should be marked as modified after agent update"
    );

    receiver
        .try_recv()
        .expect("Should receive success response");

    Ok(())
}

#[nvim_oxi::test]
fn open_buffer_disk_content_unchanged() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test.txt").unwrap();
    temp_file.write_str("initial content").unwrap();

    // Open file in Neovim buffer
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Create Requests handler and add a write file request
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );
    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "updated content"),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    // Execute the request
    requests
        .default_response(&request_id, serde_json::Value::Null)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify file on disk is NOT updated (buffer only, not saved)
    let disk_content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert_eq!(
        disk_content, "initial content",
        "File on disk should NOT be updated when buffer is already open"
    );

    Ok(())
}

#[nvim_oxi::test]
fn new_file_created() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let new_file = temp_dir.child("new_file.txt");

    // Create Requests handler and add a write file request
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );
    let (sender, mut receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(new_file.path(), "created content"),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    // Execute the request
    requests
        .default_response(&request_id, serde_json::Value::Null)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    new_file.assert("created content\n");
    receiver
        .try_recv()
        .expect("Should receive success response");

    Ok(())
}

#[nvim_oxi::test]
fn file_exists_but_closed() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("existing.txt").unwrap();
    temp_file.write_str("old content").unwrap();

    // Create Requests handler and add a write file request
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );
    let (sender, mut receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "new content"),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    // Execute the request
    requests
        .default_response(&request_id, serde_json::Value::Null)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Note: Neovim always adds a trailing newline when writing files
    temp_file.assert("new content\n");
    receiver
        .try_recv()
        .expect("Should receive success response");

    Ok(())
}

#[nvim_oxi::test]
fn responder_send_failure_returns_error() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("send_fail.txt").unwrap();

    // Create Requests handler and add a write file request
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );
    let (sender, receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder =
        Responder::WriteFileResponse(sender, create_write_request(temp_file.path(), "content"));
    let request_id = requests.add_request("test-session".to_string(), responder);

    drop(receiver);

    let result = requests.default_response(&request_id, serde_json::Value::Null);
    assert!(result.is_err(), "Should return error when send fails");
    Ok(())
}

#[nvim_oxi::test]
fn responder_send_failure_contains_acp_message() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("send_fail.txt").unwrap();

    // Create Requests handler and add a write file request
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );
    let (sender, receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder =
        Responder::WriteFileResponse(sender, create_write_request(temp_file.path(), "content"));
    let request_id = requests.add_request("test-session".to_string(), responder);

    drop(receiver);

    let result = requests.default_response(&request_id, serde_json::Value::Null);
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Failed to respond to ACP"));
    Ok(())
}

#[nvim_oxi::test]
fn buffer_already_open_marked_modified() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("open_file.txt").unwrap();
    temp_file.write_str("original disk content").unwrap();

    // Open the file in Neovim
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Create Requests handler and add a write file request
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );
    let (sender, mut receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "agent updated content"),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    // Execute the request
    requests
        .default_response(&request_id, serde_json::Value::Null)
        .expect("Request should succeed");

    // Verify: Buffer should be updated and marked modified
    let updated_buffer = nvim_oxi::api::list_bufs()
        .into_iter()
        .find(|b| b.get_name().map(|p| p == temp_file.path()).unwrap_or(false))
        .expect("Buffer should still exist");

    let is_modified: bool = nvim_oxi::api::get_option_value::<bool>(
        "modified",
        &nvim_oxi::api::opts::OptionOpts::builder()
            .buffer(updated_buffer.clone())
            .build(),
    )
    .expect("Should get modified option");
    assert!(is_modified, "Buffer should be marked as modified");

    receiver
        .try_recv()
        .expect("Should receive success response");

    Ok(())
}

#[nvim_oxi::test]
fn buffer_already_open_disk_unchanged() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("open_file.txt").unwrap();
    temp_file.write_str("original disk content").unwrap();

    // Open the file in Neovim
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Create Requests handler and add a write file request
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );
    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "agent updated content"),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    // Execute the request
    requests
        .default_response(&request_id, serde_json::Value::Null)
        .expect("Request should succeed");

    // Verify: File on disk should NOT be changed
    let disk_content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert_eq!(
        disk_content, "original disk content",
        "Disk file should remain unchanged when buffer is open"
    );

    Ok(())
}

#[nvim_oxi::test]
fn buffer_already_open_response_sent() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("open_file.txt").unwrap();
    temp_file.write_str("original disk content").unwrap();

    // Open the file in Neovim
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Create Requests handler and add a write file request
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );
    let (sender, mut receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "agent updated content"),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    // Execute the request
    requests
        .default_response(&request_id, serde_json::Value::Null)
        .expect("Request should succeed");

    // Verify: Response was sent
    assert!(
        receiver.try_recv().is_ok(),
        "Should receive success response"
    );

    Ok(())
}

#[nvim_oxi::test]
fn write_file_cleanup_request_exists_before_response() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("default_cleanup.txt").unwrap();

    // Use Requests handler to verify cleanup
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );

    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "test content"),
    );

    let request_id = requests.add_request("test-session".to_string(), responder);

    // Verify request exists before response
    assert!(requests.get_request(&request_id).is_some());
    Ok(())
}

#[nvim_oxi::test]
fn write_file_cleanup_default_response_succeeds() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("default_cleanup.txt").unwrap();

    // Use Requests handler to verify cleanup
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );

    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "test content"),
    );

    let request_id = requests.add_request("test-session".to_string(), responder);

    // Use default_response (goes through Request::default() -> now calls finish())
    let result = requests.default_response(&request_id, serde_json::Value::Null);
    assert!(result.is_ok());
    Ok(())
}

#[nvim_oxi::test]
fn write_file_cleanup_request_removed_after_response() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("default_cleanup.txt").unwrap();

    // Use Requests handler to verify cleanup
    let requests = Arc::new(
        Requests::new(Arc::new(Mutex::new(PluginState::default()))).map_err(|e| {
            nvim_oxi::api::Error::Other(format!("Failed to create Requests: {}", e))
        })?,
    );

    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "test content"),
    );

    let request_id = requests.add_request("test-session".to_string(), responder);

    // Use default_response (goes through Request::default() -> now calls finish())
    requests
        .default_response(&request_id, serde_json::Value::Null)
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Wait for cleanup via AsyncHandle + mpsc channel
    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );

    assert!(
        cleaned_up,
        "WriteFile request should be cleaned up via default_response after finish() fix"
    );
    Ok(())
}
