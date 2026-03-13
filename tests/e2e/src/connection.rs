use agent_client_protocol::{AuthenticateResponse, InitializeResponse};
use hermes::{
    api::{ConnectionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use pretty_assertions::assert_eq;
use std::time::Duration;
use tracing::warn;

use crate::{TIMEOUT_IN_SECONDS, utilities::autocommand};

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
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(connect_obj.clone())?;

    connect.call((nvim_oxi::String::from("opencode"), None))?;

    Ok(())
}

#[nvim_oxi::test]
fn test_initialization() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;

    let wait_for_response =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);

    connect.call((nvim_oxi::String::from("opencode"), None))?;

    let response = wait_for_response(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;

    assert_eq!(
        response.agent_info.unwrap().name.to_lowercase().as_str(),
        "opencode"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_authenticate_flow() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let authenticate: Function<String, ()> =
        FromObject::from_object(dict.get("authenticate").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_authentication = autocommand::listen_for_autocommand::<AuthenticateResponse>(
        Commands::Authenticated,
    );

    connect.call((nvim_oxi::String::from("copilot"), None))?;

    let mut init_response = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    if let Some(auth_method) = init_response.auth_methods.pop() {
        authenticate.call(auth_method.id.to_string())?;
        let auth_response = wait_for_authentication(Duration::from_secs(TIMEOUT_IN_SECONDS));
        println!(
            "Authentication successful, received response: {:?}",
            auth_response
        );
        assert!(
            auth_response.is_ok(),
            "Authentication should succeed"
        );
    } else {
        warn!("No authentication methods available from agent, skipping auth test");
    }

    disconnect.call(DisconnectArgs::All)?;

    Ok(())
}
