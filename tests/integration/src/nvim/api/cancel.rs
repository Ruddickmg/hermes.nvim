//! Integration tests for the `cancel` API entry point.
//!
//! `Api::cancel` resolves the argument as a session id when it maps to a known
//! session, otherwise as a Hermes request UUID for a pending incoming request.
//! Both branches require an active connection, which cannot be established in
//! this crate, so the tests here cover the error path that does not need one.
use crate::helpers::{mock_connection_manager, mock_runtime};
use async_lock::Mutex;
use hermes::{
    Handler, PluginState, api::Api, nvim::requests::Requests,
    utilities::detect_project_storage_path,
};
use std::rc::Rc;
use std::sync::Arc;

fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

fn create_test_api(
    plugin_state: Arc<Mutex<PluginState>>,
    logger: &'static hermes::utilities::Logger,
) -> Api {
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

#[nvim_oxi::test]
fn cancel_returns_error_when_no_connection() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.cancel("test-session".to_string()));

    assert!(
        result.is_err(),
        "Expected an error when no connection exists, got: {:?}",
        result
    );
    Ok(())
}
