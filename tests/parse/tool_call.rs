use agent_client_protocol::{
    Content, ContentBlock, TextContent, ToolCall, ToolCallContent, ToolCallId, ToolCallLocation,
    ToolKind,
};
use hermes::nvim::parse::tool_call_event;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_tool_call_event_ok() {
    let tool_call = ToolCall::new(ToolCallId::new("call_001"), "Reading file".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending);

    let result = tool_call_event(tool_call);
    assert!(result.is_ok());
}

#[test]
fn test_tool_call_event_contains_id() {
    let tool_call = ToolCall::new(ToolCallId::new("call_001"), "Reading file".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending);

    let result = tool_call_event(tool_call).unwrap();
    let id = result.get("id").unwrap();
    assert_eq!(*id, nvim_oxi::Object::from("call_001"));
}

#[test]
fn test_tool_call_event_with_input_output() {
    let tool_call = ToolCall::new(ToolCallId::new("call_002"), "Writing file".to_string())
        .kind(ToolKind::Edit)
        .status(agent_client_protocol::ToolCallStatus::InProgress)
        .raw_input(serde_json::json!({"path": "/test/file.txt"}))
        .raw_output(serde_json::json!({"bytes_written": 100}));

    let result = tool_call_event(tool_call);
    assert!(result.is_ok());
}

#[test]
fn test_tool_call_event_title() {
    let tool_call = ToolCall::new(ToolCallId::new("call_003"), "Analyzing code".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending);

    let result = tool_call_event(tool_call).unwrap();
    let title = result.get("title").unwrap();
    assert_eq!(*title, nvim_oxi::Object::from("Analyzing code"));
}

#[test]
fn test_tool_call_event_kind_read() {
    let tool_call = ToolCall::new(ToolCallId::new("call_004"), "Read file".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending);

    let result = tool_call_event(tool_call).unwrap();
    let kind = result.get("kind").unwrap();
    assert_eq!(*kind, nvim_oxi::Object::from("Read"));
}

#[test]
fn test_tool_call_event_kind_edit() {
    let tool_call = ToolCall::new(ToolCallId::new("call_005"), "Edit file".to_string())
        .kind(ToolKind::Edit)
        .status(agent_client_protocol::ToolCallStatus::Pending);

    let result = tool_call_event(tool_call).unwrap();
    let kind = result.get("kind").unwrap();
    assert_eq!(*kind, nvim_oxi::Object::from("Edit"));
}

#[test]
fn test_tool_call_event_status_pending() {
    let tool_call = ToolCall::new(ToolCallId::new("call_006"), "Task".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending);

    let result = tool_call_event(tool_call).unwrap();
    let status = result.get("status").unwrap();
    assert_eq!(*status, nvim_oxi::Object::from("Pending"));
}

#[test]
fn test_tool_call_event_status_in_progress() {
    let tool_call = ToolCall::new(ToolCallId::new("call_007"), "Task".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::InProgress);

    let result = tool_call_event(tool_call).unwrap();
    let status = result.get("status").unwrap();
    assert_eq!(*status, nvim_oxi::Object::from("InProgress"));
}

#[test]
fn test_tool_call_event_status_completed() {
    let tool_call = ToolCall::new(ToolCallId::new("call_008"), "Task".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Completed);

    let result = tool_call_event(tool_call).unwrap();
    let status = result.get("status").unwrap();
    assert_eq!(*status, nvim_oxi::Object::from("Completed"));
}

#[test]
fn test_tool_call_event_input() {
    let tool_call = ToolCall::new(ToolCallId::new("call_009"), "Task".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending)
        .raw_input(serde_json::json!({"file": "/path/to/file.txt"}));

    let result = tool_call_event(tool_call).unwrap();
    let input = result.get("input").unwrap();
    assert_eq!(
        *input,
        nvim_oxi::Object::from(r#"{"file":"/path/to/file.txt"}"#)
    );
}

#[test]
fn test_tool_call_event_output() {
    let tool_call = ToolCall::new(ToolCallId::new("call_010"), "Task".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Completed)
        .raw_output(serde_json::json!({"result": "success"}));

    let result = tool_call_event(tool_call).unwrap();
    let output = result.get("output").unwrap();
    assert_eq!(*output, nvim_oxi::Object::from(r#"{"result":"success"}"#));
}

#[test]
fn test_tool_call_event_contains_content_array() {
    let tool_call = ToolCall::new(ToolCallId::new("call_011"), "Task".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending);

    let result = tool_call_event(tool_call).unwrap();
    let content = result.get("content").unwrap();
    assert_eq!(*content, nvim_oxi::Object::from(nvim_oxi::Array::new()));
}

#[test]
fn test_tool_call_event_contains_locations_array() {
    let tool_call = ToolCall::new(ToolCallId::new("call_012"), "Task".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending);

    let result = tool_call_event(tool_call).unwrap();
    let locations = result.get("locations").unwrap();
    assert_eq!(*locations, nvim_oxi::Object::from(nvim_oxi::Array::new()));
}

#[test]
fn test_tool_call_event_with_text_content() {
    let text_content = TextContent::new("Hello, world!");
    let content_block = ContentBlock::Text(text_content);
    let content = Content::new(content_block);
    let tool_call = ToolCall::new(ToolCallId::new("call_013"), "Task".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending)
        .content(vec![ToolCallContent::Content(content)]);

    let result = tool_call_event(tool_call).unwrap();
    let content_arr = result.get("content").unwrap();

    let mut expected_dict = nvim_oxi::Dictionary::new();
    expected_dict.insert("text", "Hello, world!");
    expected_dict.insert("type", "text");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_dict)]);

    assert_eq!(*content_arr, nvim_oxi::Object::from(expected));
}

#[test]
fn test_tool_call_event_with_location() {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"file contents").unwrap();
    let path = temp_file.path().to_path_buf();

    let location = ToolCallLocation::new(path);
    let tool_call = ToolCall::new(ToolCallId::new("call_014"), "Task".to_string())
        .kind(ToolKind::Read)
        .status(agent_client_protocol::ToolCallStatus::Pending)
        .locations(vec![location]);

    let result = tool_call_event(tool_call).unwrap();
    let locations = result.get("locations").unwrap();

    let mut expected_dict = nvim_oxi::Dictionary::new();
    expected_dict.insert("path", "file contents");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_dict)]);

    assert_eq!(*locations, nvim_oxi::Object::from(expected));
}
