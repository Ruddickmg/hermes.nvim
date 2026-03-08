use std::time::Duration;

use crate::utilities::autocommand;
use agent_client_protocol::{InitializeResponse, NewSessionResponse};
use hermes::{
    acp::connection::{Assistant, Protocol},
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};

#[nvim_oxi::test]
fn test_default_session_creation() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<Option<ConnectionArgs>, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("createSession").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::CreatedSession);

    connect.call(Some(ConnectionArgs {
        agent: Some(Assistant::Opencode),
        protocol: Some(Protocol::Stdio),
    }))?;

    wait_for_initialization(Duration::from_secs(2))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(2));

    println!("Session creation response: {:#?}", session);

    disconnect.call(DisconnectArgs::All)?;

    assert!(session.is_ok());

    Ok(())
}
