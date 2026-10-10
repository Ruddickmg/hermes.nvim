use crate::helpers::{mock_connection_manager, mock_runtime};
use async_lock::Mutex;
use hermes::{
    Handler, PluginState,
    api::{Api, SetModeArgs},
    nvim::requests::Requests,
    utilities::detect_project_storage_path,
};
use std::rc::Rc;
use std::sync::Arc;

fn create_test_api(
    plugin_state: Arc<Mutex<PluginState>>,
    logger: &'static hermes::utilities::Logger,
) -> hermes::api::Api {
    let runtime = mock_runtime();
    let requests = Rc::new(
        Requests::new(runtime.clone(), plugin_state.clone()).expect("Failed to create requests"),
    );
    let connection_manager = mock_connection_manager(&plugin_state);
    let handler = Arc::new(
        Handler::new(
            plugin_state.clone(),
            connection_manager.clone(),
            runtime.clone(),
            requests.clone(),
        )
        .expect("Failed to create handler"),
    );
    Api::new(plugin_state, logger, handler, requests, connection_manager)
}

fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

/// Test: set_mode returns SessionNotFound when session_info has no entry for the session_id.
#[nvim_oxi::test]
fn set_mode_returns_session_not_found_when_no_session_info() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.set_mode(SetModeArgs::from((
        "nonexistent-session".to_string(),
        "chat".to_string(),
    ))));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::SessionNotFound(_))
    ));

    Ok(())
}

/// Test: set_mode returns Unsupported when session exists but has no mode info.
#[nvim_oxi::test]
fn set_mode_returns_unsupported_when_session_has_no_mode_info() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    // Populate session_info with a session that has no modes
    {
        let mut state = block_on(plugin_state.lock());
        let session = agent_client_protocol::schema::v1::NewSessionResponse::new("test-session");
        state.set_session_info(&session);
    }

    let result = block_on(api.set_mode(SetModeArgs::from((
        "test-session".to_string(),
        "chat".to_string(),
    ))));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Unsupported(_))
    ));

    Ok(())
}
