use crate::helpers::{mock_connection_manager, mock_runtime};
use agent_client_protocol::schema::v1::{
    NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectOption,
};
use async_lock::Mutex;
use hermes::{
    Handler, PluginState, api::Api, nvim::requests::Requests,
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

fn session_with_model_option() -> NewSessionResponse {
    let option = SessionConfigOption::select(
        "model",
        "Model",
        "gpt4",
        vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
    )
    .category(SessionConfigOptionCategory::Model);
    NewSessionResponse::new("test-session").config_options(vec![option])
}

#[nvim_oxi::test]
fn models_returns_session_not_found_when_no_session_info() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.models("nonexistent-session".to_string()));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::SessionNotFound(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn models_returns_unsupported_when_session_has_no_model_info() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let session = NewSessionResponse::new("test-session");
        state.set_session_info(&session);
    }

    let result = block_on(api.models("test-session".to_string()));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Unsupported(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn models_returns_ok_with_config_options() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let session = session_with_model_option();
        state.set_session_info(&session);
    }

    let result = block_on(api.models("test-session".to_string()));

    assert!(
        result.is_ok(),
        "models should return Ok with config options"
    );

    Ok(())
}
