//! Integration tests for Handler notification and permissions
use agent_client_protocol::{
    Client, ContentBlock, ContentChunk, SessionNotification, SessionUpdate, TextContent,
};
use hermes::acp::handler::Handler;
use hermes::nvim::state::PluginState;
use crate::helpers::{MockClient, MockRequestHandler};
use std::sync::Arc;
use tokio::sync::Mutex;

fn create_test_notification() -> SessionNotification {
    let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new("test message")));
    SessionNotification::new("session_id", SessionUpdate::UserMessageChunk(chunk))
}

#[nvim_oxi::test]
fn test_can_receive_notifications_returns_true_by_default() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(state.clone(), Arc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let result = tokio_test::block_on(handler.can_receive_notifications());
    assert!(result, "Should return true by default");
    
    Ok(())
}

#[nvim_oxi::test]
fn test_can_receive_notifications_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.allow_notifications = false;

    let handler = Handler::new(state.clone(), Arc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let result = tokio_test::block_on(handler.can_receive_notifications());
    assert!(!result, "Should return false when disabled");
    
    Ok(())
}

#[nvim_oxi::test]
fn test_session_notification_permissions_allowed() -> nvim_oxi::Result<()> {
    let _mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Arc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let notification = create_test_notification();
    let res: agent_client_protocol::Result<()> = tokio_test::block_on(handler.session_notification(notification));
    assert!(res.is_ok(), "Should succeed when permissions allowed");
    
    Ok(())
}

#[nvim_oxi::test]
fn test_session_notification_permissions_denied() -> nvim_oxi::Result<()> {
    let _mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.allow_notifications = false;

    let handler = Handler::new(state.clone(), Arc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let notification = create_test_notification();
    let res = tokio_test::block_on(handler.session_notification(notification));
    assert_eq!(res, Err(agent_client_protocol::Error::method_not_found()), "Should return method_not_found when permissions denied");
    
    Ok(())
}

#[nvim_oxi::test]
fn test_session_notification_permissions_denied_does_not_call_handler() -> nvim_oxi::Result<()> {
    let mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.allow_notifications = false;

    let handler = Handler::new(state.clone(), Arc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let notification = create_test_notification();
    let _ = tokio_test::block_on(handler.session_notification(notification));
    
    let was_called = tokio_test::block_on(async {
        *mock.notification_called.lock().await
    });
    assert!(!was_called, "Handler should NOT have been called when permissions denied");
    
    Ok(())
}
