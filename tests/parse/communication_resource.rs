use agent_client_protocol::{
    Annotations, BlobResourceContents, EmbeddedResource, EmbeddedResourceResource, Role,
    TextResourceContents,
};
use hermes::nvim::parse::resource_event;

#[test]
fn test_resource_event_ok() {
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt",
        "Hello world",
    ));
    let block = EmbeddedResource::new(resource);
    let (dict, _content_type) = resource_event(block);
    assert_eq!(dict.get("resource").is_some(), true);
}

#[test]
fn test_resource_event_text_resource_value() {
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "myfile.txt",
        "file content here",
    ));
    let block = EmbeddedResource::new(resource);
    let (dict, _) = resource_event(block);

    let resource_dict = dict.get("resource").unwrap();
    let expected_dict = {
        let mut d = nvim_oxi::Dictionary::new();
        d.insert("text", "myfile.txt");
        d.insert("uri", "file content here");
        d
    };
    assert_eq!(*resource_dict, nvim_oxi::Object::from(expected_dict));
}

#[test]
fn test_resource_event_blob_resource_value() {
    let resource = EmbeddedResourceResource::BlobResourceContents(BlobResourceContents::new(
        "base64data",
        "file:///blob.png",
    ));
    let block = EmbeddedResource::new(resource);
    let (dict, _) = resource_event(block);

    let resource_dict = dict.get("resource").unwrap();
    let expected_dict = {
        let mut d = nvim_oxi::Dictionary::new();
        d.insert("blob", "base64data");
        d.insert("uri", "file:///blob.png");
        d
    };
    assert_eq!(*resource_dict, nvim_oxi::Object::from(expected_dict));
}

#[test]
fn test_resource_event_content_type() {
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt", "content",
    ));
    let block = EmbeddedResource::new(resource);
    let (_dict, content_type) = resource_event(block);

    assert_eq!(content_type, "Resource");
}

#[test]
fn test_resource_event_without_annotations() {
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt", "content",
    ));
    let block = EmbeddedResource::new(resource);
    let (dict, _) = resource_event(block);

    assert_eq!(dict.get("annotations").is_some(), false);
}

#[test]
fn test_resource_event_without_meta() {
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt", "content",
    ));
    let block = EmbeddedResource::new(resource);
    let (dict, _) = resource_event(block);

    assert_eq!(dict.get("meta").is_some(), false);
}

#[test]
fn test_resource_event_with_empty_annotations() {
    let annotations = Annotations::new();
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt", "content",
    ));
    let block = EmbeddedResource::new(resource).annotations(annotations);
    let (dict, _) = resource_event(block);

    let annotations_dict = dict.get("annotations").unwrap();
    let expected_dict = nvim_oxi::Dictionary::new();
    assert_eq!(*annotations_dict, nvim_oxi::Object::from(expected_dict));
}

#[test]
fn test_resource_event_with_annotations_audience() {
    let annotations = Annotations::new().audience(vec![Role::User]);
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt", "content",
    ));
    let block = EmbeddedResource::new(resource).annotations(annotations);
    let (dict, _) = resource_event(block);

    let annotations_dict = dict.get("annotations").unwrap();
    let expected_dict = {
        let mut d = nvim_oxi::Dictionary::new();
        d.insert(
            "audience",
            nvim_oxi::Array::from_iter([nvim_oxi::Object::from("User")]),
        );
        d
    };
    assert_eq!(*annotations_dict, nvim_oxi::Object::from(expected_dict));
}

#[test]
fn test_resource_event_with_annotations_priority() {
    let annotations = Annotations::new().priority(5.0);
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt", "content",
    ));
    let block = EmbeddedResource::new(resource).annotations(annotations);
    let (dict, _) = resource_event(block);

    let annotations_dict = dict.get("annotations").unwrap();
    let expected_dict = {
        let mut d = nvim_oxi::Dictionary::new();
        d.insert("priority", 5.0);
        d
    };
    assert_eq!(*annotations_dict, nvim_oxi::Object::from(expected_dict));
}

#[test]
fn test_resource_event_with_annotations_last_modified() {
    let annotations = Annotations::new().last_modified("2024-01-01".to_string());
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt", "content",
    ));
    let block = EmbeddedResource::new(resource).annotations(annotations);
    let (dict, _) = resource_event(block);

    let annotations_dict = dict.get("annotations").unwrap();
    let expected_dict = {
        let mut d = nvim_oxi::Dictionary::new();
        d.insert("lastModified", "2024-01-01");
        d
    };
    assert_eq!(*annotations_dict, nvim_oxi::Object::from(expected_dict));
}

#[test]
fn test_resource_event_with_meta() {
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt", "content",
    ));
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    let block = EmbeddedResource::new(resource).meta(meta);
    let (dict, _) = resource_event(block);

    assert_eq!(dict.get("meta").is_some(), true);
}

#[test]
fn test_resource_event_with_annotations_and_meta() {
    let annotations = Annotations::new();
    let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
        "test.txt", "content",
    ));
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    let block = EmbeddedResource::new(resource)
        .annotations(annotations)
        .meta(meta);
    let (dict, _) = resource_event(block);

    let annotations_dict = dict.get("annotations").unwrap();
    let expected_annotations = nvim_oxi::Dictionary::new();
    assert_eq!(
        *annotations_dict,
        nvim_oxi::Object::from(expected_annotations)
    );

    assert_eq!(dict.get("meta").is_some(), true);
}
