use crate::helpers::{mock_connection_manager, mock_runtime};
use async_lock::Mutex;
use hermes::{
    Handler, PluginState,
    api::{Api, DeleteSessionArg, DeleteSessionOptions},
    nvim::{hermes, requests::Requests},
    utilities::detect_project_storage_path,
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
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
fn delete_session_returns_ok_when_not_allowed() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state, logger);

    let result = block_on(api.delete_session((
        DeleteSessionArg::Single("test-session".to_string()),
        None::<DeleteSessionOptions>,
    )));

    assert!(result.is_ok());

    Ok(())
}

#[nvim_oxi::test]
fn delete_session_returns_error_when_no_connection() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();

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

    let agent = hermes::acp::connection::Assistant::from("test-agent");
    let info = agent_client_protocol::schema::v1::InitializeResponse::new(
        agent_client_protocol::schema::ProtocolVersion::V1,
    )
    .agent_capabilities(
        agent_client_protocol::schema::v1::AgentCapabilities::new().session_capabilities(
            agent_client_protocol::schema::v1::SessionCapabilities::new().delete(Some(
                agent_client_protocol::schema::v1::SessionDeleteCapabilities::new(),
            )),
        ),
    );
    block_on(handler.set_agent_info(agent.clone(), info));
    {
        let mut state_guard = block_on(plugin_state.lock());
        state_guard.agent_info.set_agent(agent);
    }

    let api = Api::new(plugin_state, logger, handler, requests, connection_manager);

    let result = block_on(api.delete_session((
        DeleteSessionArg::Single("test-session".to_string()),
        None::<DeleteSessionOptions>,
    )));

    assert!(
        result.is_err(),
        "Expected error when no connection exists, got: {:?}",
        result
    );

    Ok(())
}

#[nvim_oxi::test]
fn delete_session_multiple_returns_error_when_no_connection() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();

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

    let agent = hermes::acp::connection::Assistant::from("test-agent");
    let info = agent_client_protocol::schema::v1::InitializeResponse::new(
        agent_client_protocol::schema::ProtocolVersion::V1,
    )
    .agent_capabilities(
        agent_client_protocol::schema::v1::AgentCapabilities::new().session_capabilities(
            agent_client_protocol::schema::v1::SessionCapabilities::new().delete(Some(
                agent_client_protocol::schema::v1::SessionDeleteCapabilities::new(),
            )),
        ),
    );
    block_on(handler.set_agent_info(agent.clone(), info));
    {
        let mut state_guard = block_on(plugin_state.lock());
        state_guard.agent_info.set_agent(agent);
    }

    let api = Api::new(plugin_state, logger, handler, requests, connection_manager);

    let result = block_on(api.delete_session((
        DeleteSessionArg::Multiple(vec!["session-one".to_string(), "session-two".to_string()]),
        None::<DeleteSessionOptions>,
    )));

    assert!(
        result.is_err(),
        "Expected error when no connection exists, got: {:?}",
        result
    );

    Ok(())
}

#[nvim_oxi::test]
fn delete_session_lua_function_single_none() -> nvim_oxi::Result<()> {
    let dict: Dictionary = hermes()?;
    let delete_session: Function<(DeleteSessionArg, Option<DeleteSessionOptions>), ()> =
        FromObject::from_object(dict.get("delete_session").unwrap().clone())?;

    let result = delete_session.call((
        DeleteSessionArg::Single("test-session".to_string()),
        None::<DeleteSessionOptions>,
    ));

    assert!(
        result.is_ok(),
        "Function should return Ok even without connection"
    );
    Ok(())
}

#[nvim_oxi::test]
fn delete_session_lua_function_multi_none() -> nvim_oxi::Result<()> {
    let dict: Dictionary = hermes()?;
    let delete_session: Function<(DeleteSessionArg, Option<DeleteSessionOptions>), ()> =
        FromObject::from_object(dict.get("delete_session").unwrap().clone())?;

    let result = delete_session.call((
        DeleteSessionArg::Multiple(vec!["a".to_string(), "b".to_string()]),
        None::<DeleteSessionOptions>,
    ));

    assert!(
        result.is_ok(),
        "Function should return Ok even without connection"
    );
    Ok(())
}

#[nvim_oxi::test]
fn delete_session_lua_function_some_options() -> nvim_oxi::Result<()> {
    let dict: Dictionary = hermes()?;
    let delete_session: Function<(DeleteSessionArg, Option<DeleteSessionOptions>), ()> =
        FromObject::from_object(dict.get("delete_session").unwrap().clone())?;

    let result = delete_session.call((
        DeleteSessionArg::Single("test-session".to_string()),
        Some(DeleteSessionOptions { cancel: false }),
    ));

    assert!(
        result.is_ok(),
        "Function should return Ok even without connection"
    );
    Ok(())
}
