use agent_client_protocol::{
    Content, ContentBlock, TextContent, ToolCallContent, ToolCallId, ToolCallUpdate,
    ToolCallUpdateFields,
};
use hermes::nvim::parse::tool_call_update_event;

#[test]
fn test_tool_call_update_event_ok() {
    let fields = ToolCallUpdateFields::new();
    let update = ToolCallUpdate::new(ToolCallId::new("call_001"), fields);

    let result = tool_call_update_event(update).unwrap();
    let id = result.get("id").unwrap();
    assert_eq!(*id, nvim_oxi::Object::from("call_001"));
}

#[test]
fn test_tool_call_update_event_id() {
    let fields = ToolCallUpdateFields::new();
    let update = ToolCallUpdate::new(ToolCallId::new("call_001"), fields);

    let result = tool_call_update_event(update).unwrap();
    let id = result.get("id").unwrap();
    assert_eq!(*id, nvim_oxi::Object::from("call_001"));
}

#[test]
fn test_tool_call_update_event_id_different_value() {
    let fields = ToolCallUpdateFields::new();
    let update = ToolCallUpdate::new(ToolCallId::new("call_999"), fields);

    let result = tool_call_update_event(update).unwrap();
    let id = result.get("id").unwrap();
    assert_eq!(*id, nvim_oxi::Object::from("call_999"));
}

#[test]
fn test_tool_call_update_event_without_fields_content() {
    let fields = ToolCallUpdateFields::new();
    let update = ToolCallUpdate::new(ToolCallId::new("call_002"), fields);

    let result = tool_call_update_event(update).unwrap();
    assert!(result.get("fields").is_none());
}

#[test]
fn test_tool_call_update_event_with_empty_content() {
    let fields = ToolCallUpdateFields::new().content(vec![]);
    let update = ToolCallUpdate::new(ToolCallId::new("call_003"), fields);

    let result = tool_call_update_event(update).unwrap();
    let fields_arr = result.get("fields").unwrap();
    assert_eq!(*fields_arr, nvim_oxi::Object::from(nvim_oxi::Array::new()));
}

#[test]
fn test_tool_call_update_event_with_text_content() {
    let text_content = TextContent::new("Hello, world!");
    let content_block = ContentBlock::Text(text_content);
    let content = Content::new(content_block);
    let fields = ToolCallUpdateFields::new().content(vec![ToolCallContent::Content(content)]);
    let update = ToolCallUpdate::new(ToolCallId::new("call_004"), fields);

    let result = tool_call_update_event(update).unwrap();
    let fields_arr = result.get("fields").unwrap();

    let mut expected_dict = nvim_oxi::Dictionary::new();
    expected_dict.insert("text", "Hello, world!");
    expected_dict.insert("type", "text");
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from(expected_dict)]);

    assert_eq!(*fields_arr, nvim_oxi::Object::from(expected));
}

#[test]
fn test_tool_call_update_event_without_meta() {
    let fields = ToolCallUpdateFields::new();
    let update = ToolCallUpdate::new(ToolCallId::new("call_005"), fields);

    let result = tool_call_update_event(update).unwrap();
    assert!(result.get("meta").is_none());
}

#[test]
fn test_tool_call_update_event_with_meta() {
    let fields = ToolCallUpdateFields::new();
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "agent"})
        .as_object()
        .unwrap()
        .clone();
    let update = ToolCallUpdate::new(ToolCallId::new("call_006"), fields).meta(meta);

    let result = tool_call_update_event(update).unwrap();
    let meta_obj = result.get("meta").unwrap();
    let mut expected_meta = nvim_oxi::Dictionary::new();
    expected_meta.insert("source", "agent");
    assert_eq!(*meta_obj, nvim_oxi::Object::from(expected_meta));
}
