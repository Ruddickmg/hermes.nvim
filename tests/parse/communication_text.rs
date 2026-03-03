use agent_client_protocol::TextContent;
use hermes::nvim::parse::text_event;

#[test]
fn test_text_event_ok() {
    let text = TextContent::new("Hello, world!");
    let (dict, _content_type) = text_event(text);
    let text_value = dict.get("text").unwrap();
    assert_eq!(*text_value, nvim_oxi::Object::from("Hello, world!"));
}

#[test]
fn test_text_event_text_value() {
    let text = TextContent::new("Hello, world!");
    let (dict, _content_type) = text_event(text);

    let text_value = dict.get("text").unwrap();
    assert_eq!(*text_value, nvim_oxi::Object::from("Hello, world!"));
}

#[test]
fn test_text_event_content_type() {
    let text = TextContent::new("Test");
    let (_dict, content_type) = text_event(text);

    assert_eq!(content_type, "Text");
}

#[test]
fn test_text_event_without_annotations() {
    let text = TextContent::new("No annotations");
    let (dict, _) = text_event(text);

    assert!(dict.get("annotations").is_none());
}

#[test]
fn test_text_event_without_meta() {
    let text = TextContent::new("No meta");
    let (dict, _) = text_event(text);

    assert!(dict.get("meta").is_none());
}

#[test]
fn test_text_event_with_annotations() {
    use agent_client_protocol::Annotations;

    let annotations = Annotations::new();
    let text = TextContent::new("With annotations").annotations(annotations);
    let (dict, _) = text_event(text);

    let annotations_value = dict.get("annotations").unwrap();
    let expected_dict = nvim_oxi::Dictionary::new();
    assert_eq!(*annotations_value, nvim_oxi::Object::from(expected_dict));
}

#[test]
fn test_text_event_with_meta() {
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    let text = TextContent::new("With meta").meta(meta);
    let (dict, _) = text_event(text);

    let meta_obj = dict.get("meta").unwrap();
    let mut expected_meta = nvim_oxi::Dictionary::new();
    expected_meta.insert("source", "test");
    assert_eq!(*meta_obj, nvim_oxi::Object::from(expected_meta));
}

#[test]
fn test_text_event_with_annotations_and_meta() {
    use agent_client_protocol::Annotations;

    let annotations = Annotations::new();
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    let text = TextContent::new("Full").annotations(annotations).meta(meta);
    let (dict, _) = text_event(text);

    let text_value = dict.get("text").unwrap();
    assert_eq!(*text_value, nvim_oxi::Object::from("Full"));

    let annotations_value = dict.get("annotations").unwrap();
    let expected_annotations = nvim_oxi::Dictionary::new();
    assert_eq!(
        *annotations_value,
        nvim_oxi::Object::from(expected_annotations)
    );

    let meta_obj = dict.get("meta").unwrap();
    let mut expected_meta = nvim_oxi::Dictionary::new();
    expected_meta.insert("source", "test");
    assert_eq!(*meta_obj, nvim_oxi::Object::from(expected_meta));
}
