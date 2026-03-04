mod utilities;
use std::time::Duration;
use agent_client_protocol::InitializeResponse;
use hermes::{
    acp::connection::{Assistant, Protocol},
    api::{ConnectionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{
    Dictionary, Function,
    conversion::FromObject,
};
use utilities::autocommand;

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

    let wait_for_response = autocommand::listen_for_autocommand::<InitializeResponse>(
        Commands::AgentConnectionInitialized,
    );

    connect.call(Some(ConnectionArgs {
        agent: Some(Assistant::Opencode),
        protocol: Some(Protocol::Stdio),
    }))?;

    let response = wait_for_response(Duration::from_secs(2))?;

    disconnect.call(DisconnectArgs::All)?;

    assert_eq!(response.agent_info.unwrap().name.to_lowercase().as_str(), "opencode");

    Ok(())
}
