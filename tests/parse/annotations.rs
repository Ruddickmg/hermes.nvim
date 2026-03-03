use agent_client_protocol::Annotations;
use hermes::nvim::parse::annotations::parse_annotations;

#[test]
fn test_parse_annotations_empty() {
    let annotations = Annotations::new();
    let result = parse_annotations(annotations);

    assert!(result.is_empty());
}

#[test]
fn test_parse_annotations_with_audience() {
    let annotations = Annotations::new().audience(vec![agent_client_protocol::Role::User]);
    let result = parse_annotations(annotations);

    let audience = result.get("audience").unwrap();
    let expected = nvim_oxi::Array::from_iter([nvim_oxi::Object::from("User")]);
    assert_eq!(*audience, nvim_oxi::Object::from(expected));
}

#[test]
fn test_parse_annotations_with_multiple_roles() {
    let annotations = Annotations::new().audience(vec![
        agent_client_protocol::Role::User,
        agent_client_protocol::Role::Assistant,
    ]);
    let result = parse_annotations(annotations);

    let audience = result.get("audience").unwrap();
    let expected = nvim_oxi::Array::from_iter([
        nvim_oxi::Object::from("User"),
        nvim_oxi::Object::from("Assistant"),
    ]);
    assert_eq!(*audience, nvim_oxi::Object::from(expected));
}

#[test]
fn test_parse_annotations_with_priority() {
    let annotations = Annotations::new().priority(5.0);
    let result = parse_annotations(annotations);

    let priority = result.get("priority").unwrap();
    assert_eq!(*priority, nvim_oxi::Object::from(5.0));
}

#[test]
fn test_parse_annotations_with_last_modified() {
    let annotations = Annotations::new().last_modified("2024-01-01".to_string());
    let result = parse_annotations(annotations);

    let last_modified = result.get("lastModified").unwrap();
    assert_eq!(*last_modified, nvim_oxi::Object::from("2024-01-01"));
}

#[test]
fn test_parse_annotations_audience_and_priority() {
    let annotations = Annotations::new()
        .audience(vec![agent_client_protocol::Role::User])
        .priority(3.0);
    let result = parse_annotations(annotations);

    let audience = result.get("audience").unwrap();
    let expected_audience = nvim_oxi::Array::from_iter([nvim_oxi::Object::from("User")]);
    assert_eq!(*audience, nvim_oxi::Object::from(expected_audience));

    let priority = result.get("priority").unwrap();
    assert_eq!(*priority, nvim_oxi::Object::from(3.0));
}

#[test]
fn test_parse_annotations_all_fields() {
    let annotations = Annotations::new()
        .audience(vec![agent_client_protocol::Role::User])
        .last_modified("2024-06-15".to_string())
        .priority(10.0);
    let result = parse_annotations(annotations);

    let audience = result.get("audience").unwrap();
    let expected_audience = nvim_oxi::Array::from_iter([nvim_oxi::Object::from("User")]);
    assert_eq!(*audience, nvim_oxi::Object::from(expected_audience));

    let last_modified = result.get("lastModified").unwrap();
    assert_eq!(*last_modified, nvim_oxi::Object::from("2024-06-15"));

    let priority = result.get("priority").unwrap();
    assert_eq!(*priority, nvim_oxi::Object::from(10.0));
}

#[test]
fn test_parse_annotations_without_audience() {
    let annotations = Annotations::new()
        .last_modified("2024-01-01".to_string())
        .priority(1.0);
    let result = parse_annotations(annotations);

    assert!(result.get("audience").is_none());

    let last_modified = result.get("lastModified").unwrap();
    assert_eq!(*last_modified, nvim_oxi::Object::from("2024-01-01"));

    let priority = result.get("priority").unwrap();
    assert_eq!(*priority, nvim_oxi::Object::from(1.0));
}
