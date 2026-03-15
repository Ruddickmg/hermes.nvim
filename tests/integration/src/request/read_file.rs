//! Integration tests for Responder::ReadFileResponse
//!
//! Each test verifies exactly ONE behavior with exactly ONE assertion.
//! Setup code and .expect() calls don't count as assertions.
use crate::helpers::ui::wait_for;
use agent_client_protocol::{ReadTextFileRequest, ReadTextFileResponse, SessionId};
use assert_fs::prelude::*;
use assert_fs::NamedTempFile;
use hermes::nvim::requests::{RequestHandler, Requests, Responder};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use uuid::Uuid;

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
    oneshot::Receiver<agent_client_protocol::Result<ReadTextFileResponse>>,
) {
    let requests = Arc::new(Requests::new().unwrap());
    let (sender, receiver) =
        oneshot::channel::<agent_client_protocol::Result<ReadTextFileResponse>>();
    let responder = Responder::ReadFileResponse(sender, create_read_request(path, start, limit));
    let request_id = requests.add_request("test-session".to_string(), responder);
    (requests, request_id, receiver)
}

// === Basic read operations ===

#[nvim_oxi::test]
fn read_file_default_response_succeeds() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, _receiver) = setup_read_request(&path, None, None);

    let result = requests.default_response(&request_id, serde_json::Value::Null);
    assert!(result.is_ok());

    Ok(())
}

#[nvim_oxi::test]
fn read_file_returns_all_content() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, mut receiver) = setup_read_request(&path, None, None);

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert_eq!(response.content, "line0\nline1\nline2\nline3\nline4\n");

    Ok(())
}

#[nvim_oxi::test]
fn read_file_creates_request_in_pending() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, _receiver) = setup_read_request(&path, None, None);

    assert!(requests.get_request(&request_id).is_some());

    Ok(())
}

#[nvim_oxi::test]
fn read_file_gets_removed_from_pending() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, _receiver) = setup_read_request(&path, None, None);

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
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

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

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

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

// === Line ranges ===

#[nvim_oxi::test]
fn read_file_line_range_returns_correct_lines() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(1), Some(4));

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert_eq!(response.content, "line1\nline2\nline3\n");

    Ok(())
}

#[nvim_oxi::test]
fn read_file_line_range_cleanup_works() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(5);
    let (requests, request_id, _receiver) = setup_read_request(&path, Some(1), Some(4));

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
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

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert!(response.content.contains("MODIFIED1") && response.content.contains("MODIFIED2"));

    Ok(())
}

// === Error handling ===

#[nvim_oxi::test]
fn read_file_missing_file_returns_error() -> nvim_oxi::Result<()> {
    let requests = Arc::new(Requests::new().unwrap());
    let (sender, mut receiver) = oneshot::channel::<Result<ReadTextFileResponse, _>>();
    let responder = Responder::ReadFileResponse(
        sender,
        create_read_request(PathBuf::from("/nonexistent/file.txt").as_path(), None, None),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let response = receiver.try_recv().expect("Should receive response");
    assert!(response.is_err());

    Ok(())
}

#[nvim_oxi::test]
fn read_file_missing_file_cleanup_works() -> nvim_oxi::Result<()> {
    let requests = Arc::new(Requests::new().unwrap());
    let (sender, _receiver) = oneshot::channel::<Result<ReadTextFileResponse, _>>();
    let responder = Responder::ReadFileResponse(
        sender,
        create_read_request(PathBuf::from("/nonexistent/file.txt").as_path(), None, None),
    );
    let request_id = requests.add_request("test-session".to_string(), responder);

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
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

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

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

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

#[nvim_oxi::test]
fn read_file_start_beyond_length_returns_empty() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(3);
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(10), None);

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert!(response.content.is_empty());

    Ok(())
}

#[nvim_oxi::test]
fn read_file_start_beyond_length_cleanup_works() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(3);
    let (requests, request_id, _receiver) = setup_read_request(&path, Some(10), None);

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

// === Respond path ===

#[nvim_oxi::test]
fn read_file_respond_path_sends_custom_content() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(5);
    let requests = Arc::new(Requests::new().unwrap());
    let (sender, mut receiver) = oneshot::channel::<Result<ReadTextFileResponse, _>>();
    let responder = Responder::ReadFileResponse(sender, create_read_request(&path, None, None));
    let request_id = requests.add_request("test-session".to_string(), responder);

    let custom_content = "Custom content from user";
    let response_obj = nvim_oxi::Object::from(custom_content);
    requests.handle_response(&request_id, response_obj).ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert_eq!(response.content, custom_content);

    Ok(())
}

#[nvim_oxi::test]
fn read_file_respond_path_cleanup_works() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(5);
    let requests = Arc::new(Requests::new().unwrap());
    let (sender, _receiver) = oneshot::channel::<Result<ReadTextFileResponse, _>>();
    let responder = Responder::ReadFileResponse(sender, create_read_request(&path, None, None));
    let request_id = requests.add_request("test-session".to_string(), responder);

    let response_obj = nvim_oxi::Object::from("test content");
    requests.handle_response(&request_id, response_obj).ok();

    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}

// === Large file ===

#[nvim_oxi::test]
fn read_file_large_returns_correct_range() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(300);
    let (requests, request_id, mut receiver) = setup_read_request(&path, Some(100), Some(200));

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let response = receiver
        .try_recv()
        .expect("Should receive response")
        .expect("Should be Ok");
    assert!(response.content.contains("line100") && response.content.contains("line199"));

    Ok(())
}

#[nvim_oxi::test]
fn read_file_large_cleanup_works() -> nvim_oxi::Result<()> {
    let (temp_file, path) = create_file_with_lines(300);
    let (requests, request_id, _receiver) = setup_read_request(&path, Some(100), Some(200));

    requests
        .default_response(&request_id, serde_json::Value::Null)
        .ok();

    let cleaned_up = wait_for(
        || requests.get_request(&request_id).is_none(),
        Duration::from_millis(500),
    );
    assert!(cleaned_up);

    Ok(())
}
