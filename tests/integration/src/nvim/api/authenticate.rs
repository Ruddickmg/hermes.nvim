//! Integration tests for `Api::authenticate` routing decisions.
//!
//! The auth method lookup happens before any connection lookup, so both error
//! branches are reachable without an agent.

use crate::helpers::mock_runtime;
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{AuthMethod, AuthMethodTerminal, InitializeResponse};
use async_lock::Mutex;
use hermes::{
    Handler, PluginState, acp::connection::Assistant, api::Api, nvim::requests::Requests,
    utilities::detect_project_storage_path,
};
use std::rc::Rc;
use std::sync::Arc;

fn create_test_api(plugin_state: Arc<Mutex<PluginState>>) -> Api {
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let runtime = mock_runtime();
    let requests = Rc::new(
        Requests::new(runtime.clone(), plugin_state.clone()).expect("Failed to create requests"),
    );
    let handler = Arc::new(
        Handler::new(plugin_state.clone(), runtime.clone(), requests.clone())
            .expect("Failed to create handler"),
    );
    Api::new(plugin_state, logger, handler, requests)
}

fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

fn state_with_terminal_method(agent: Assistant) -> Arc<Mutex<PluginState>> {
    let state = Arc::new(Mutex::new(PluginState::new()));
    let mut guard = block_on(state.lock());
    guard.set_agent(agent.clone());
    guard.set_agent_info(
        agent,
        InitializeResponse::new(ProtocolVersion::V1).auth_methods(vec![AuthMethod::Terminal(
            AuthMethodTerminal::new("tui-auth".to_string(), "Terminal Auth"),
        )]),
    );
    drop(guard);
    state
}

/// Test: `authenticate` reports InvalidInput when the id is not advertised.
#[nvim_oxi::test]
fn authenticate_returns_invalid_input_when_auth_method_unknown() -> nvim_oxi::Result<()> {
    let api = create_test_api(Arc::new(Mutex::new(PluginState::new())));

    let result = block_on(api.authenticate("missing-id".to_string()));

    assert_eq!(
        result.unwrap_err().to_string(),
        "Invalid input provided: No auth method with id: \"missing-id\" found for agent: copilot"
    );
    Ok(())
}

/// Test: `authenticate` reports a connection error when a known method has no live connection.
#[nvim_oxi::test]
fn authenticate_returns_connection_error_when_no_connection() -> nvim_oxi::Result<()> {
    let api = create_test_api(state_with_terminal_method(Assistant::Opencode));

    let result = block_on(api.authenticate("tui-auth".to_string()));

    assert_eq!(
        result.unwrap_err().to_string(),
        "Connection error: No connection found"
    );
    Ok(())
}
