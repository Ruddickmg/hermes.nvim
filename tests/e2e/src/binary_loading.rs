//! Simple E2E test for binary loading
//!
//! Tests that the binary loading path works correctly

use hermes::nvim::hermes;
use nvim_oxi::conversion::FromObject;

/// Test that hermes loads correctly (which triggers binary loading)
#[nvim_oxi::test]
fn test_hermes_loads() -> nvim_oxi::Result<()> {
    let dict = hermes()?;

    // Check that setup function exists
    let setup = dict
        .get("setup")
        .expect("setup function should exist")
        .clone();

    // Convert to Function and call it
    let func: nvim_oxi::Function<(), ()> = FromObject::from_object(setup)?;
    func.call(())?;

    Ok(())
}
