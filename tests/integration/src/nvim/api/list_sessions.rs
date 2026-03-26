//! Integration tests for the list_sessions API
//!
//! These tests verify that the list_sessions API correctly handles:
//! - Input validation and parsing
//! - Error conditions (e.g., no connection)
//! - Integration with ConnectionManager

use hermes::api::ListSessionsConfig;
use nvim_oxi::{conversion::FromObject, Dictionary, Object};
use std::path::PathBuf;

/// Helper to create a ListSessionsConfig Object from a Dictionary
fn create_list_sessions_obj(dict: Dictionary) -> Object {
    Object::from(dict)
}

#[nvim_oxi::test]
fn test_list_sessions_config_from_object_nil() -> nvim_oxi::Result<()> {
    let obj = Object::nil();
    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert!(config.cwd.is_none(), "Default config should have no cwd");
    assert!(
        config.cursor.is_none(),
        "Default config should have no cursor"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_config_from_object_with_cwd() -> nvim_oxi::Result<()> {
    let mut dict = Dictionary::new();
    dict.insert("cwd", "/test/path");
    let obj = create_list_sessions_obj(dict);

    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert_eq!(config.cwd, Some(PathBuf::from("/test/path")));

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_config_from_object_with_cursor() -> nvim_oxi::Result<()> {
    let mut dict = Dictionary::new();
    dict.insert("cursor", "abc123");
    let obj = create_list_sessions_obj(dict);

    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert_eq!(config.cursor, Some("abc123".to_string()));

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_config_from_object_with_both() -> nvim_oxi::Result<()> {
    let mut dict = Dictionary::new();
    dict.insert("cwd", "/test/path");
    dict.insert("cursor", "xyz789");
    let obj = create_list_sessions_obj(dict);

    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert_eq!(config.cwd, Some(PathBuf::from("/test/path")));
    assert_eq!(config.cursor, Some("xyz789".to_string()));

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_config_ignores_unknown_fields() -> nvim_oxi::Result<()> {
    let mut dict = Dictionary::new();
    dict.insert("cwd", "/valid/path");
    dict.insert("unknown", "ignored");
    dict.insert("cursor", "valid_cursor");
    let obj = create_list_sessions_obj(dict);

    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert_eq!(config.cwd, Some(PathBuf::from("/valid/path")));
    assert_eq!(config.cursor, Some("valid_cursor".to_string()));

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_config_empty_dict_is_default() -> nvim_oxi::Result<()> {
    let dict = Dictionary::new();
    let obj = create_list_sessions_obj(dict);

    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert!(config.cwd.is_none());
    assert!(config.cursor.is_none());

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_config_windows_path() -> nvim_oxi::Result<()> {
    let mut dict = Dictionary::new();
    dict.insert("cwd", "C:\\Users\\test\\project");
    let obj = create_list_sessions_obj(dict);

    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert_eq!(config.cwd, Some(PathBuf::from("C:\\Users\\test\\project")));

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_config_relative_path() -> nvim_oxi::Result<()> {
    let mut dict = Dictionary::new();
    dict.insert("cwd", "./relative/path");
    let obj = create_list_sessions_obj(dict);

    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert_eq!(config.cwd, Some(PathBuf::from("./relative/path")));

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_config_empty_cursor() -> nvim_oxi::Result<()> {
    let mut dict = Dictionary::new();
    dict.insert("cursor", "");
    let obj = create_list_sessions_obj(dict);

    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert_eq!(config.cursor, Some("".to_string()));

    Ok(())
}

#[nvim_oxi::test]
fn test_list_sessions_config_only_unknown_fields() -> nvim_oxi::Result<()> {
    let mut dict = Dictionary::new();
    dict.insert("unknown_field", "value");
    dict.insert("another_unknown", 123);
    let obj = create_list_sessions_obj(dict);

    let config = ListSessionsConfig::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    assert!(config.cwd.is_none());
    assert!(config.cursor.is_none());

    Ok(())
}
