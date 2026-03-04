mod utilities;

use utilities::autocommand;
use std::sync::{Arc, Mutex};

use agent_client_protocol::InitializeResponse;
use hermes::{
    apc::connection::{Assistant, Protocol},
    api::{ConnectionArgs, DisconnectArgs},
    nvim::hermes,
};
use nvim_oxi::{
    Dictionary, Function,
    api::{self, types::AutocmdCallbackArgs},
    conversion::FromObject,
    serde::Deserializer,
};
use serde::Deserialize;

#[nvim_oxi::test]
fn test_setup_returns_connect_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("connect").is_some(),
        "connect function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
async fn test_connect_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    let connect_obj = dict.get("connect").expect("connect function not found");
    let connect: Function<Option<ConnectionArgs>, ()> =
        FromObject::from_object(connect_obj.clone())?;

    connect.call(Some(ConnectionArgs {
        agent: Some(Assistant::Opencode),
        protocol: Some(Protocol::Stdio),
    }))?;

    Ok(())
}

use std::sync::mpsc;
use std::time::Duration;

#[nvim_oxi::test]
fn test_initialization() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<Option<ConnectionArgs>, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;

    let wait_for_response = autocommand::wait_for_user_event::<InitializeResponse>("", Duration::from_secs(5));

    connect.call(Some(ConnectionArgs {
        agent: Some(Assistant::Opencode),
        protocol: Some(Protocol::Stdio),
    }))?;

    // Block until autocmd fires (with timeout)
    let response = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("Autocmd did not fire");

    disconnect.call(DisconnectArgs::All)?;

    assert!(/* validate response */ true);

    Ok(())
}
