use crate::helpers::{mock_connection_manager, mock_runtime};
use agent_client_protocol::schema::v1::{
    NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectOption,
};
use async_lock::Mutex;
use hermes::{
    Handler, PluginState,
    api::{Api, ConfigureModelConfig},
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

fn session_with_model_config() -> NewSessionResponse {
    let option = SessionConfigOption::select(
        "mc-1",
        "Model Config",
        "val1",
        vec![SessionConfigSelectOption::new("val1", "Value 1")],
    )
    .category(SessionConfigOptionCategory::ModelConfig);
    NewSessionResponse::new("test-session").config_options(vec![option])
}

#[nvim_oxi::test]
fn configure_model_returns_session_not_found() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let config = ConfigureModelConfig {
        id: "mc-1".to_string(),
        value: "val1".to_string(),
    };
    let result = block_on(api.configure_model(("nonexistent".to_string(), config)));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::SessionNotFound(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn configure_model_returns_error_for_empty_id() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let config = ConfigureModelConfig {
        id: String::new(),
        value: "val1".to_string(),
    };
    let result = block_on(api.configure_model(("test-session".to_string(), config)));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Internal(ref msg))
        if msg.contains("Invalid configure_model argument")
    ));

    Ok(())
}

#[nvim_oxi::test]
fn configure_model_returns_error_for_empty_value() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let config = ConfigureModelConfig {
        id: "mc-1".to_string(),
        value: String::new(),
    };
    let result = block_on(api.configure_model(("test-session".to_string(), config)));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Internal(ref msg))
        if msg.contains("Invalid configure_model argument")
    ));

    Ok(())
}

#[nvim_oxi::test]
fn configure_model_returns_config_not_found() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let session = session_with_model_config();
        state.set_session_info(&session);
    }

    let config = ConfigureModelConfig {
        id: "nonexistent-config".to_string(),
        value: "val1".to_string(),
    };
    let result = block_on(api.configure_model(("test-session".to_string(), config)));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Internal(_))
    ));

    Ok(())
}
