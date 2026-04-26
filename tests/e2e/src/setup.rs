//! E2E test for setup API
//!
//! Verifies that setup can be called through the hermes plugin dictionary.

use hermes::nvim::hermes;
use nvim_oxi::{Object, conversion::FromObject};

/// Test: setup function exists and can be called via hermes dict
#[nvim_oxi::test]
fn e2e_setup_can_be_called_from_hermes_dict() -> nvim_oxi::Result<()> {
    let dict = hermes()?;
    let setup = dict
        .get("setup")
        .expect("setup function should exist")
        .clone();

    // Convert to Function and call it with empty config
    let func: nvim_oxi::Function<(), ()> = FromObject::from_object(setup)?;
    func.call(())?;

    Ok(())
}

/// Test: setup function handles invalid data gracefully via Poppable
#[nvim_oxi::test]
fn e2e_setup_handles_invalid_arg() -> nvim_oxi::Result<()> {
    let dict = hermes()?;
    let setup = dict
        .get("setup")
        .expect("setup function should exist")
        .clone();

    // Convert to Function that accepts any Object (invalid data)
    let func: nvim_oxi::Function<Object, ()> = FromObject::from_object(setup)?;

    // Pass a number instead of expected table/nil
    let result = func.call(Object::from(123i64));

    // Should succeed because Poppable returns default on invalid data
    assert!(result.is_ok(), "setup should succeed with invalid arg");

    Ok(())
}
