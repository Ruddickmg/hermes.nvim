use crate::helpers::{mock_connection_manager, mock_runtime};
use async_lock::Mutex;
use hermes::{
    Handler, PluginState,
    api::{Api, SetThoughtLevelArgs},
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

#[nvim_oxi::test]
fn thought_levels_returns_session_not_found_when_no_session_info() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.thought_levels("nonexistent-session".to_string()));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::SessionNotFound(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn thought_levels_returns_unsupported_when_session_has_no_thought_level_info()
-> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let session = agent_client_protocol::schema::v1::NewSessionResponse::new("test-session");
        state.set_session_info(&session);
    }

    let result = block_on(api.thought_levels("test-session".to_string()));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Unsupported(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn thought_levels_returns_ok_for_config_options() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let option = agent_client_protocol::schema::v1::SessionConfigOption::select(
            "thought_level",
            "Thought Level",
            "low",
            vec![
                agent_client_protocol::schema::v1::SessionConfigSelectOption::new("low", "Low"),
                agent_client_protocol::schema::v1::SessionConfigSelectOption::new(
                    "medium", "Medium",
                ),
            ],
        )
        .category(agent_client_protocol::schema::v1::SessionConfigOptionCategory::ThoughtLevel);
        let session = agent_client_protocol::schema::v1::NewSessionResponse::new("test-session")
            .config_options(vec![option]);
        state.set_session_info(&session);
    }

    let result = block_on(api.thought_levels("test-session".to_string()));

    assert!(
        result.is_ok(),
        "thought_levels should return Ok for config options"
    );

    Ok(())
}

#[nvim_oxi::test]
fn set_thought_level_returns_session_not_found_when_no_session_info() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.set_thought_level(SetThoughtLevelArgs::from((
        "nonexistent-session".to_string(),
        "low".to_string(),
    ))));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::SessionNotFound(_))
    ));

    Ok(())
}

#[nvim_oxi::test]
fn set_thought_level_returns_unsupported_when_session_has_no_thought_level_info()
-> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    {
        let mut state = block_on(plugin_state.lock());
        let session = agent_client_protocol::schema::v1::NewSessionResponse::new("test-session");
        state.set_session_info(&session);
    }

    let result = block_on(api.set_thought_level(SetThoughtLevelArgs::from((
        "test-session".to_string(),
        "low".to_string(),
    ))));

    assert!(matches!(
        result,
        Err(hermes::acp::error::Error::Unsupported(_))
    ));

    Ok(())
}
