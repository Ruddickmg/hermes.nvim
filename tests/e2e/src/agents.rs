//! E2E tests for user-configured agents.
//!
//! Verifies that agents defined via `setup({ agents = { ... } })` are used when
//! connecting by id without explicit per-connect connection options.

use agent_client_protocol::schema::v1::InitializeResponse;
use hermes::{
    api::{ConnectionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, Object, conversion::FromObject};
use pretty_assertions::assert_eq;
use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent},
};

/// Configure a single user-defined agent connection through `setup()`.
fn configure_agent(
    setup: &Function<Object, ()>,
    id: &str,
    protocol: &str,
    host: &str,
    port: u16,
) -> Result<(), nvim_oxi::Error> {
    let mut entry = Dictionary::new();
    entry.insert("protocol", protocol);
    entry.insert("host", host);
    entry.insert("port", port as i64);

    let mut agents = Dictionary::new();
    agents.insert(id, entry);

    let mut config = Dictionary::new();
    config.insert("agents", agents);

    setup
        .call(Object::from(config))
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))
}

#[nvim_oxi::test]
fn test_connect_uses_configured_agent_connection() -> Result<(), nvim_oxi::Error> {
    let dict = hermes()?;

    let setup: Function<Object, ()> = FromObject::from_object(
        dict.get("setup")
            .expect("setup function should exist")
            .clone(),
    )?;
    let connect: Function<ConnectionArgs, ()> = FromObject::from_object(
        dict.get("connect")
            .expect("connect function should exist")
            .clone(),
    )?;
    let disconnect: Function<DisconnectArgs, ()> = FromObject::from_object(
        dict.get("disconnect")
            .expect("disconnect function should exist")
            .clone(),
    )?;

    let agent = MockAgent::new();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    configure_agent(
        &setup,
        "custom-mock",
        "tcp",
        "localhost",
        mock_handle.port(),
    )?;

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);

    connect
        .call((nvim_oxi::String::from("custom-mock"), None))
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))?;

    let init_response = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        init_response
            .agent_info
            .unwrap()
            .name
            .to_lowercase()
            .as_str(),
        "mock-agent"
    );

    Ok(())
}
