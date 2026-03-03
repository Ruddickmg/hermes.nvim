use agent_client_protocol::ImageContent;
use hermes::nvim::parse::image_event;

#[test]
fn test_image_event_ok() {
    let image = ImageContent::new("base64data", "image/png");
    let (dict, _content_type) = image_event(image);
    assert_eq!(dict.get("data").is_some(), true);
}

#[test]
fn test_image_event_data_value() {
    let image = ImageContent::new("base64encoded", "image/png");
    let (dict, _content_type) = image_event(image);

    let data = dict.get("data").unwrap();
    assert_eq!(*data, nvim_oxi::Object::from("base64encoded"));
}

#[test]
fn test_image_event_mime_type_value() {
    let image = ImageContent::new("data", "image/jpeg");
    let (dict, _content_type) = image_event(image);

    let mime_type = dict.get("mimeType").unwrap();
    assert_eq!(*mime_type, nvim_oxi::Object::from("image/jpeg"));
}

#[test]
fn test_image_event_content_type() {
    let image = ImageContent::new("data", "image/png");
    let (_dict, content_type) = image_event(image);

    assert_eq!(content_type, "Image");
}

#[test]
fn test_image_event_without_uri() {
    let image = ImageContent::new("data", "image/png");
    let (dict, _) = image_event(image);

    assert_eq!(dict.get("uri").is_some(), false);
}

#[test]
fn test_image_event_with_uri() {
    let image = ImageContent::new("data", "image/png").uri("file:///image.png");
    let (dict, _) = image_event(image);

    let uri = dict.get("uri").unwrap();
    assert_eq!(*uri, nvim_oxi::Object::from("file:///image.png"));
}

#[test]
fn test_image_event_without_annotations() {
    let image = ImageContent::new("data", "image/png");
    let (dict, _) = image_event(image);

    assert_eq!(dict.get("annotations").is_some(), false);
}

#[test]
fn test_image_event_without_meta() {
    let image = ImageContent::new("data", "image/png");
    let (dict, _) = image_event(image);

    assert_eq!(dict.get("meta").is_some(), false);
}

#[test]
fn test_image_event_with_annotations() {
    use agent_client_protocol::Annotations;

    let annotations = Annotations::new();
    let image = ImageContent::new("data", "image/png").annotations(annotations);
    let (dict, _) = image_event(image);

    let annotations_value = dict.get("annotations").unwrap();
    let expected_dict = nvim_oxi::Dictionary::new();
    assert_eq!(*annotations_value, nvim_oxi::Object::from(expected_dict));
}

#[test]
fn test_image_event_with_meta() {
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    let image = ImageContent::new("data", "image/png").meta(meta);
    let (dict, _) = image_event(image);

    assert_eq!(dict.get("meta").is_some(), true);
}

#[test]
fn test_image_event_with_uri_and_meta() {
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    let image = ImageContent::new("data", "image/png")
        .uri("file:///image.png")
        .meta(meta);
    let (dict, _) = image_event(image);

    let uri = dict.get("uri").unwrap();
    assert_eq!(*uri, nvim_oxi::Object::from("file:///image.png"));

    assert_eq!(dict.get("meta").is_some(), true);
}
