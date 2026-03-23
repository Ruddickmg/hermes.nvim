//! E2E test for setup API
//!
//! Verifies that setup can be called through the hermes plugin dictionary.

use hermes::nvim::hermes;
use nvim_oxi::conversion::FromObject;

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
