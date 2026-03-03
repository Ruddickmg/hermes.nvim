use agent_client_protocol::{Annotations, ResourceLink, Role};
use hermes::nvim::parse::resource_link_event;

#[test]
fn test_resource_link_event_ok() {
    let block = ResourceLink::new("test.txt", "file:///test.txt");
    let (dict, _) = resource_link_event(block);
    assert_eq!(dict.get("name").is_some(), true);
}

#[test]
fn test_resource_link_event_name_value() {
    let block = ResourceLink::new("myfile.txt", "file:///test.txt");
    let (dict, _) = resource_link_event(block);

    let name = dict.get("name").unwrap();
    assert_eq!(*name, nvim_oxi::Object::from("myfile.txt"));
}

#[test]
fn test_resource_link_event_uri_value() {
    let block = ResourceLink::new("test.txt", "file:///path/to/file.txt");
    let (dict, _) = resource_link_event(block);

    let uri = dict.get("uri").unwrap();
    assert_eq!(*uri, nvim_oxi::Object::from("file:///path/to/file.txt"));
}

#[test]
fn test_resource_link_event_content_type() {
    let block = ResourceLink::new("test.txt", "file:///test.txt");
    let (_, content_type) = resource_link_event(block);

    assert_eq!(content_type, "ResourceLink");
}

#[test]
fn test_resource_link_event_without_description() {
    let block = ResourceLink::new("test.txt", "file:///test.txt");
    let (dict, _) = resource_link_event(block);

    assert_eq!(dict.get("description").is_some(), false);
}

#[test]
fn test_resource_link_event_with_description() {
    let block =
        ResourceLink::new("test.txt", "file:///test.txt").description("A test file".to_string());
    let (dict, _) = resource_link_event(block);

    let description = dict.get("description").unwrap();
    assert_eq!(*description, nvim_oxi::Object::from("A test file"));
}

#[test]
fn test_resource_link_event_without_mime_type() {
    let block = ResourceLink::new("test.txt", "file:///test.txt");
    let (dict, _) = resource_link_event(block);

    assert_eq!(dict.get("mimeType").is_some(), false);
}

#[test]
fn test_resource_link_event_with_mime_type() {
    let block =
        ResourceLink::new("test.txt", "file:///test.txt").mime_type("text/plain".to_string());
    let (dict, _) = resource_link_event(block);

    let mime_type = dict.get("mimeType").unwrap();
    assert_eq!(*mime_type, nvim_oxi::Object::from("text/plain"));
}

#[test]
fn test_resource_link_event_without_size() {
    let block = ResourceLink::new("test.txt", "file:///test.txt");
    let (dict, _) = resource_link_event(block);

    assert_eq!(dict.get("size").is_some(), false);
}

#[test]
fn test_resource_link_event_with_size() {
    let block = ResourceLink::new("test.txt", "file:///test.txt").size(100);
    let (dict, _) = resource_link_event(block);

    let size = dict.get("size").unwrap();
    assert_eq!(*size, nvim_oxi::Object::from(100));
}

#[test]
fn test_resource_link_event_without_title() {
    let block = ResourceLink::new("test.txt", "file:///test.txt");
    let (dict, _) = resource_link_event(block);

    assert_eq!(dict.get("title").is_some(), false);
}

#[test]
fn test_resource_link_event_with_title() {
    let block = ResourceLink::new("test.txt", "file:///test.txt").title("Test File".to_string());
    let (dict, _) = resource_link_event(block);

    let title = dict.get("title").unwrap();
    assert_eq!(*title, nvim_oxi::Object::from("Test File"));
}

#[test]
fn test_resource_link_event_without_annotations() {
    let block = ResourceLink::new("test.txt", "file:///test.txt");
    let (dict, _) = resource_link_event(block);

    assert_eq!(dict.get("annotations").is_some(), false);
}

#[test]
fn test_resource_link_event_with_empty_annotations() {
    let annotations = Annotations::new();
    let block = ResourceLink::new("test.txt", "file:///test.txt").annotations(annotations);
    let (dict, _) = resource_link_event(block);

    let annotations_dict = dict.get("annotations").unwrap();
    let expected_dict = nvim_oxi::Dictionary::new();
    assert_eq!(*annotations_dict, nvim_oxi::Object::from(expected_dict));
}

#[test]
fn test_resource_link_event_with_annotations_audience() {
    let annotations = Annotations::new().audience(vec![Role::User]);
    let block = ResourceLink::new("test.txt", "file:///test.txt").annotations(annotations);
    let (dict, _) = resource_link_event(block);

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
fn test_resource_link_event_without_meta() {
    let block = ResourceLink::new("test.txt", "file:///test.txt");
    let (dict, _) = resource_link_event(block);

    assert_eq!(dict.get("meta").is_some(), false);
}

#[test]
fn test_resource_link_event_with_meta() {
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    let block = ResourceLink::new("test.txt", "file:///test.txt").meta(meta);
    let (dict, _) = resource_link_event(block);

    assert_eq!(dict.get("meta").is_some(), true);
}

#[test]
fn test_resource_link_event_with_all_optional_fields() {
    let annotations = Annotations::new().priority(5.0);
    let meta: serde_json::Map<String, serde_json::Value> = serde_json::json!({"source": "test"})
        .as_object()
        .unwrap()
        .clone();
    let block = ResourceLink::new("test.txt", "file:///test.txt")
        .description("Description".to_string())
        .mime_type("text/plain".to_string())
        .size(100)
        .title("Title".to_string())
        .annotations(annotations)
        .meta(meta);
    let (dict, _) = resource_link_event(block);

    let name = dict.get("name").unwrap();
    assert_eq!(*name, nvim_oxi::Object::from("test.txt"));

    let uri = dict.get("uri").unwrap();
    assert_eq!(*uri, nvim_oxi::Object::from("file:///test.txt"));

    let description = dict.get("description").unwrap();
    assert_eq!(*description, nvim_oxi::Object::from("Description"));

    let mime_type = dict.get("mimeType").unwrap();
    assert_eq!(*mime_type, nvim_oxi::Object::from("text/plain"));

    let size = dict.get("size").unwrap();
    assert_eq!(*size, nvim_oxi::Object::from(100));

    let title = dict.get("title").unwrap();
    assert_eq!(*title, nvim_oxi::Object::from("Title"));

    let annotations_dict = dict.get("annotations").unwrap();
    let expected_annotations = {
        let mut d = nvim_oxi::Dictionary::new();
        d.insert("priority", 5.0);
        d
    };
    assert_eq!(
        *annotations_dict,
        nvim_oxi::Object::from(expected_annotations)
    );

    assert_eq!(dict.get("meta").is_some(), true);
}
