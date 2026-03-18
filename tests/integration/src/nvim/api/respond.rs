//! Integration tests for respond API
//!
//! Tests verify UUID parsing logic used by the respond function.

use uuid::Uuid;

/// Test that valid UUID format is parsed correctly
#[nvim_oxi::test]
fn respond_parses_valid_uuid_format() -> nvim_oxi::Result<()> {
    let valid_uuid = Uuid::new_v4().to_string();
    let parsed = Uuid::parse_str(&valid_uuid);

    assert!(parsed.is_ok(), "Should parse valid UUID format");

    Ok(())
}

/// Test that invalid UUID format returns error
#[nvim_oxi::test]
fn respond_rejects_invalid_uuid_format() -> nvim_oxi::Result<()> {
    let invalid_uuid = "not-a-valid-uuid";
    let parsed = Uuid::parse_str(invalid_uuid);

    assert!(parsed.is_err(), "Should reject invalid UUID format");

    Ok(())
}

/// Test that UUID with dashes is parsed correctly
#[nvim_oxi::test]
fn respond_parses_uuid_with_dashes() -> nvim_oxi::Result<()> {
    let uuid_with_dashes = "550e8400-e29b-41d4-a716-446655440000";
    let parsed = Uuid::parse_str(uuid_with_dashes);

    assert!(
        parsed.is_ok(),
        "Should parse UUID with standard dashes format"
    );

    Ok(())
}

/// Test that empty string is rejected as UUID
#[nvim_oxi::test]
fn respond_rejects_empty_string_as_uuid() -> nvim_oxi::Result<()> {
    let empty = "";
    let parsed = Uuid::parse_str(empty);

    assert!(parsed.is_err(), "Should reject empty string as UUID");

    Ok(())
}
