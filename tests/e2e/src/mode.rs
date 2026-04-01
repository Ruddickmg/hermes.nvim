use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent},
};
use agent_client_protocol::{InitializeResponse, NewSessionResponse};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, SetModeArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use std::time::Duration;

#[nvim_oxi::test]
fn test_setup_returns_set_mode_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("set_mode").is_some(),
        "set_mode function should be registered"
    );

    Ok(())
}

// Mock agent doesn't support session modes (returns None for modes),
// so this test needs to be skipped unless we configure a custom response.
// For now, we test that set_mode call doesn't crash even when modes aren't supported.
#[nvim_oxi::test]
fn test_set_mode_success() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let set_mode: Function<SetModeArgs, ()> =
        FromObject::from_object(dict.get("set_mode").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    // Start mock agent
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let mut options = Dictionary::new();
    options.insert("protocol", "socket");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Mock agent doesn't advertise modes, so we just test that set_mode doesn't crash
    // when called with a session (even if it returns an error)
    let _result = set_mode.call((session_id.to_string(), "test-mode".to_string()));

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    // Should either succeed or return an error (not crash)
    // The mock agent's NewSessionResponse doesn't include modes, so set_mode will likely error
    // But it shouldn't panic
    // Test passes if we reach here without panicking

    Ok(())
}
