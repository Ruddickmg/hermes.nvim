//! Test helpers for working with the MockAgent in E2E tests

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{autocommand, mock_agent::MockAgent, mock_agent_handle::MockAgentHandle},
};
use agent_client_protocol::{InitializeResponse, NewSessionResponse};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::lua::Pushable;
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use std::time::Duration;

/// Setup result containing all the common test resources
pub struct MockAgentSetup {
    pub dict: Dictionary,
    pub mock_handle: MockAgentHandle,
    pub connect: Function<ConnectionArgs, ()>,
    pub disconnect: Function<DisconnectArgs, ()>,
    pub create_session: Function<CreateSessionArgs, ()>,
}

/// Start a mock agent and connect to it
///
/// This helper function:
/// 1. Starts a mock agent
/// 2. Connects to it via socket protocol
/// 3. Waits for initialization
/// 4. Returns all the common functions and handles needed for tests
pub fn setup_mock_agent() -> Result<MockAgentSetup, nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    // Start mock agent
    let (agent, conn_rx) = MockAgent::new();
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    // Setup autocommand listeners
    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);

    // Connect to mock agent
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;

    // Wait for initialization
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    Ok(MockAgentSetup {
        dict,
        mock_handle,
        connect,
        disconnect,
        create_session,
    })
}

/// Setup result containing session-related resources
pub struct MockAgentWithSession {
    pub setup: MockAgentSetup,
    pub session_id: agent_client_protocol::SessionId,
}

/// Start a mock agent, connect, and create a session
///
/// This helper function:
/// 1. Sets up a mock agent connection
/// 2. Creates a default session
/// 3. Returns the session ID along with all other resources
pub fn setup_mock_agent_with_session() -> Result<MockAgentWithSession, nvim_oxi::Error> {
    let setup = setup_mock_agent()?;

    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    setup.create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    Ok(MockAgentWithSession { setup, session_id })
}

/// Cleanup helper that disconnects and closes the mock agent
pub fn cleanup_mock_agent(setup: MockAgentSetup) -> Result<(), nvim_oxi::Error> {
    setup.disconnect.call(DisconnectArgs::All)?;
    setup.mock_handle.close();
    Ok(())
}

/// Cleanup helper for MockAgentWithSession
pub fn cleanup_mock_agent_with_session(setup: MockAgentWithSession) -> Result<(), nvim_oxi::Error> {
    cleanup_mock_agent(setup.setup)
}

/// Helper to get a function from the hermes dictionary
pub fn get_func<A>(dict: &Dictionary, name: &str) -> Result<Function<A, ()>, nvim_oxi::Error>
where
    A: Pushable,
{
    let obj = dict
        .get(name)
        .ok_or_else(|| {
            nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
                "Function {} not found",
                name
            )))
        })?
        .clone();

    FromObject::from_object(obj)
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))
}
