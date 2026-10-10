//! E2E tests for `authenticate()` routing between terminal and agent auth methods,
//! and for the `respond()` round trip that accepts or declines a terminal auth
//! challenge.
//!
//! The routing tests advertise exactly one auth method on the mock agent's
//! initialize response, then assert on the autocommand the matching code path
//! produces. The respond tests drive the TerminalAuthentication event through
//! `hermes.respond()` and assert on whether Authenticated fires.

use agent_client_protocol::schema::v1::{
    AuthMethod, AuthMethodAgent, AuthMethodTerminal, AuthenticateResponse, InitializeResponse,
};
use hermes::{
    api::{ConnectionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, Object, conversion::FromObject};
use pretty_assertions::assert_eq;
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand, mock_agent::MockAgent, mock_agent_handle::MockAgentHandle,
        test_helpers::connect_to_mock_agent,
    },
};

fn create_func<A, R>(plugin: Dictionary, name: &str) -> Function<A, R> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .unwrap_or_else(|_| panic!("Failed to create function for {}", name))
}

/// Mirror of the payload hermes sends on the TerminalAuthentication autocommand:
/// the selected auth method flattened alongside the id to pass to `respond()`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct TerminalAuthenticationData {
    #[serde(rename = "requestId")]
    request_id: String,
    #[serde(flatten)]
    method: AuthMethodTerminal,
}

/// Advertise a single auth method without dropping the mock agent's default capabilities.
fn mock_agent_advertising(method: AuthMethod) -> MockAgent {
    let agent = MockAgent::new();
    let mut config = agent.config().lock().unwrap();
    config.initialize_response = config
        .initialize_response
        .clone()
        .auth_methods(vec![method]);
    drop(config);
    agent
}

/// Test: `authenticate()` with a terminal method fires TerminalAuthentication
/// instead of sending an `authenticate` request to the agent.
#[nvim_oxi::test]
fn terminal_auth_method_fires_terminal_authentication_autocommand() -> Result<(), nvim_oxi::Error> {
    let agent = mock_agent_advertising(AuthMethod::Terminal(
        AuthMethodTerminal::new("tui-auth".to_string(), "Terminal Auth")
            .args(vec!["--device-code".to_string()]),
    ));
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");
    let authenticate: Function<String, ()> = create_func(dict.clone(), "authenticate");

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_terminal_authentication = autocommand::listen_for_autocommand::<
        TerminalAuthenticationData,
    >(Commands::TerminalAuthentication);

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    authenticate.call("tui-auth".to_string())?;
    let data = wait_for_terminal_authentication(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    let expected = TerminalAuthenticationData {
        request_id: Uuid::parse_str(&data.request_id)
            .expect("request_id should be a valid UUID")
            .to_string(),
        method: AuthMethodTerminal::new("tui-auth".to_string(), "Terminal Auth")
            .args(vec!["--device-code".to_string()]),
    };
    assert_eq!(data, expected);

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();
    Ok(())
}

/// Test: `authenticate()` with an agent method still takes the pre-existing
/// `authenticate` request path and fires the Authenticated autocommand.
#[nvim_oxi::test]
fn agent_auth_method_fires_authenticated_autocommand() -> Result<(), nvim_oxi::Error> {
    let agent = mock_agent_advertising(AuthMethod::Agent(AuthMethodAgent::new(
        "mock-auth".to_string(),
        "Mock Auth",
    )));
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");
    let authenticate: Function<String, ()> = create_func(dict.clone(), "authenticate");

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_authentication =
        autocommand::listen_for_autocommand::<AuthenticateResponse>(Commands::Authenticated);

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    authenticate.call("mock-auth".to_string())?;
    let response = wait_for_authentication(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    assert_eq!(response, AuthenticateResponse::default());

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();
    Ok(())
}

/// Everything the two `respond()` tests need once the mock agent is connected
/// and `authenticate("tui-auth")` has been issued.
struct RespondSetup {
    mock_handle: MockAgentHandle,
    disconnect: Function<DisconnectArgs, ()>,
    respond: Function<(String, Object), ()>,
    wait_for_terminal_authentication:
        Box<dyn Fn(Duration) -> Result<TerminalAuthenticationData, nvim_oxi::Error>>,
    wait_for_authentication: Box<dyn Fn(Duration) -> Result<AuthenticateResponse, nvim_oxi::Error>>,
}

/// Start a terminal-auth-capable mock agent, connect, wait for initialization,
/// and issue the authenticate call that produces a TerminalAuthentication event.
fn setup_respond_test() -> Result<RespondSetup, nvim_oxi::Error> {
    let agent = mock_agent_advertising(AuthMethod::Terminal(AuthMethodTerminal::new(
        "tui-auth".to_string(),
        "Terminal Auth",
    )));
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");
    let authenticate: Function<String, ()> = create_func(dict.clone(), "authenticate");
    let respond: Function<(String, Object), ()> = create_func(dict.clone(), "respond");

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_terminal_authentication = autocommand::listen_for_autocommand::<
        TerminalAuthenticationData,
    >(Commands::TerminalAuthentication);
    let wait_for_authentication =
        autocommand::listen_for_autocommand::<AuthenticateResponse>(Commands::Authenticated);

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    authenticate.call("tui-auth".to_string())?;

    Ok(RespondSetup {
        mock_handle,
        disconnect,
        respond,
        wait_for_terminal_authentication,
        wait_for_authentication,
    })
}

/// Test: `respond(id, true)` reconnects the agent and fires Authenticated.
#[nvim_oxi::test]
fn accepting_terminal_auth_reconnects_and_fires_authenticated() -> Result<(), nvim_oxi::Error> {
    let setup = setup_respond_test()?;

    let data = (setup.wait_for_terminal_authentication)(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    setup.respond.call((data.request_id, Object::from(true)))?;

    let response = (setup.wait_for_authentication)(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    assert_eq!(response, AuthenticateResponse::default());

    setup.disconnect.call(DisconnectArgs::All)?;
    setup.mock_handle.close();
    Ok(())
}

/// Test: `respond(id, false)` declines, so no reconnect happens and the
/// Authenticated autocommand never fires.
#[nvim_oxi::test]
fn declining_terminal_auth_does_not_fire_authenticated() -> Result<(), nvim_oxi::Error> {
    let setup = setup_respond_test()?;

    let data = (setup.wait_for_terminal_authentication)(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    setup.respond.call((data.request_id, Object::from(false)))?;

    let result = (setup.wait_for_authentication)(Duration::from_secs(5));

    assert!(
        result.is_err(),
        "Authenticated should not fire when terminal authentication is declined"
    );

    setup.disconnect.call(DisconnectArgs::All)?;
    setup.mock_handle.close();
    Ok(())
}
