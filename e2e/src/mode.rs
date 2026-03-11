use crate::{utilities::autocommand, TIMEOUT_IN_SECONDS};
use agent_client_protocol::{InitializeResponse, NewSessionResponse, SetSessionModeResponse};
use hermes::{
    acp::connection::{Assistant, Protocol},
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, SetModeArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{conversion::FromObject, Dictionary, Function};
use pretty_assertions::assert_eq;
use std::time::Duration;

#[nvim_oxi::test]
fn test_setup_returns_set_mode_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("setMode").is_some(),
        "setMode function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_set_mode_success() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("createSession").unwrap().clone())?;
    let set_mode: Function<SetModeArgs, ()> =
        FromObject::from_object(dict.get("setMode").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::CreatedSession);
    let wait_for_mode_update =
        autocommand::listen_for_autocommand::<SetSessionModeResponse>(Commands::ModeUpdated);

    connect.call((nvim_oxi::String::from("opencode"), None))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;
    let modes = session
        .modes
        .expect("session did not include modes; mode selection is not supported for this session");
    let current_mode = modes.current_mode_id;
    let mode_id = modes
        .available_modes
        .into_iter()
        .find(|m| m.id != current_mode)
        .map(|m| m.id)
        .expect(
            "Expected at least one available mode different from the current mode for this test",
        );

    set_mode.call((session_id.to_string(), mode_id.to_string()))?;

    let mode_response = wait_for_mode_update(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;

    assert!(
        mode_response.is_ok(),
        "ModeUpdated autocommand should fire after setMode call"
    );

    Ok(())
}
