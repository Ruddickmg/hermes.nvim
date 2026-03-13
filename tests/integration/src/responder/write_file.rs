//! Integration tests for Responder::WriteFileResponse::default()
use agent_client_protocol::{SessionId, WriteTextFileRequest, WriteTextFileResponse};
use assert_fs::prelude::*;
use assert_fs::{NamedTempFile, TempDir};
use hermes::nvim::requests::Responder;
use std::path::Path;
use tokio::sync::oneshot;

/// Helper function to create a WriteTextFileRequest
fn create_write_request(path: &Path, content: &str) -> WriteTextFileRequest {
    WriteTextFileRequest::new(SessionId::from("test-session"), path, content)
}

#[nvim_oxi::test]
fn open_buffer_updated() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test.txt").unwrap();
    temp_file.write_str("initial content").unwrap();

    // Open file in Neovim buffer
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    let (sender, mut receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "updated content"),
    );

    responder
        .default()
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    temp_file.assert("updated content");
    receiver
        .try_recv()
        .expect("Should receive success response");

    Ok(())
}

#[nvim_oxi::test]
fn file_exists_but_closed() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("existing.txt").unwrap();
    temp_file.write_str("old content").unwrap();

    let (sender, mut receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(temp_file.path(), "new content"),
    );

    responder
        .default()
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    temp_file.assert("new content");
    receiver
        .try_recv()
        .expect("Should receive success response");

    Ok(())
}

#[nvim_oxi::test]
fn new_file_created() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let new_file = temp_dir.child("new_file.txt");

    let (sender, mut receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        create_write_request(new_file.path(), "created content"),
    );

    responder
        .default()
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    new_file.assert("created content");
    receiver
        .try_recv()
        .expect("Should receive success response");

    Ok(())
}

#[nvim_oxi::test]
fn invalid_path_returns_error() -> nvim_oxi::Result<()> {
    use std::path::PathBuf;

    let invalid_path = PathBuf::from("/nonexistent/path/file.txt");
    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder =
        Responder::WriteFileResponse(sender, create_write_request(&invalid_path, "content"));

    let result = responder.default();
    assert!(result.is_err(), "Should return error for invalid path");

    Ok(())
}

#[nvim_oxi::test]
fn responder_send_failure_handled() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("send_fail.txt").unwrap();
    let (sender, receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder =
        Responder::WriteFileResponse(sender, create_write_request(temp_file.path(), "content"));

    drop(receiver);

    let result = responder.default();
    assert!(result.is_err(), "Should return error when send fails");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to respond to ACP"));

    Ok(())
}
