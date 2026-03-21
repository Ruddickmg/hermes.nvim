//! Integration tests for Handler notification and permissions
use crate::helpers::MockRequestHandler;
use agent_client_protocol::{
    Client, ContentBlock, ContentChunk, Error, SessionNotification, SessionUpdate, TextContent,
};
use hermes::acp::handler::Handler;
use hermes::nvim::state::PluginState;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

fn create_test_notification() -> SessionNotification {
    let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new("test message")));
    SessionNotification::new("session_id", SessionUpdate::UserMessageChunk(chunk))
}

#[nvim_oxi::test]
fn test_session_notification_permissions_denied() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.send_notifications = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let notification = create_test_notification();
    let res = tokio_test::block_on(handler.session_notification(notification));
    assert_eq!(
        res.unwrap_err(),
        Error::method_not_found(),
        "Should return MethodNotFound when permissions denied"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_session_notification_permissions_allowed() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let notification = create_test_notification();
    let res: agent_client_protocol::Result<()> =
        tokio_test::block_on(handler.session_notification(notification));
    assert!(res.is_ok(), "Should succeed when permissions allowed");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_write_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.fs_write_access = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let result = tokio_test::block_on(handler.can_write());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_read_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.fs_read_access = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let result = tokio_test::block_on(handler.can_read());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_access_terminal_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.terminal_access = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let result = tokio_test::block_on(handler.can_access_terminal());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_request_permissions_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.request_permissions = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let result = tokio_test::block_on(handler.can_request_permissions());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_set_agent_info_updates_agent_information() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let agent = hermes::acp::connection::Assistant::from("test-agent");
    let info = agent_client_protocol::InitializeResponse::new(
        agent_client_protocol::ProtocolVersion::LATEST,
    );

    tokio_test::block_on(handler.set_agent_info(agent.clone(), info.clone()));

    // Verify agent info was set
    let stored_info = state.blocking_lock().agent_info.get(&agent).cloned();
    assert!(stored_info.is_some(), "Agent info should be stored");

    Ok(())
}
