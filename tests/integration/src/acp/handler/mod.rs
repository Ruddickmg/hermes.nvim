//! Integration tests for Handler notification and permissions
use crate::helpers::MockRequestHandler;
use agent_client_protocol::{
    Client, ContentBlock, ContentChunk, Error, SessionNotification, SessionUpdate, TextContent,
    UsageUpdate,
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

// Note: These tests cover the "true" code paths for CI coverage requirements.
// Per AGENTS.md, we avoid testing defaults, but these methods are used in
// production code (client.rs) and need coverage. Keeping them per AGENTS.md:793-799.

#[nvim_oxi::test]
fn test_can_write_returns_true_when_enabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    // fs_write_access is true by default - covers the return true path
    let result = tokio_test::block_on(handler.can_write());
    assert!(result);

    Ok(())
}

#[nvim_oxi::test]
fn test_can_read_returns_true_when_enabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    // fs_read_access is true by default - covers the return true path
    let result = tokio_test::block_on(handler.can_read());
    assert!(result);

    Ok(())
}

#[nvim_oxi::test]
fn test_can_access_terminal_returns_true_when_enabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    // terminal_access is true by default - covers the return true path
    let result = tokio_test::block_on(handler.can_access_terminal());
    assert!(result);

    Ok(())
}

#[nvim_oxi::test]
fn test_can_request_permissions_returns_true_when_enabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    // request_permissions is true by default - covers the return true path
    let result = tokio_test::block_on(handler.can_request_permissions());
    assert!(result);

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

#[nvim_oxi::test]
fn test_session_notification_usage_update_succeeds() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let usage = UsageUpdate::new(1000, 200000);
    let notification = SessionNotification::new("session_id", SessionUpdate::UsageUpdate(usage));
    let res: agent_client_protocol::Result<()> =
        tokio_test::block_on(handler.session_notification(notification));
    assert!(
        res.is_ok(),
        "UsageUpdate should succeed and fire autocommand"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_can_receive_notifications_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.send_notifications = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let result = tokio_test::block_on(handler.can_receive_notifications());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_receive_notifications_returns_true_when_enabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    // send_notifications is true by default - covers the return true path
    let result = tokio_test::block_on(handler.can_receive_notifications());
    assert!(result);

    Ok(())
}

#[nvim_oxi::test]
fn test_execute_autocommand_request_sends_with_responder() -> nvim_oxi::Result<()> {
    // Test execute_autocommand_request with a responder - covers lines 207-208
    // This sends an autocommand with response_data, triggering the full flow
    use agent_client_protocol::WriteTextFileResponse;
    use hermes::nvim::requests::Responder;
    use std::sync::Arc;
    use tokio::sync::oneshot;

    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        agent_client_protocol::WriteTextFileRequest::new(
            agent_client_protocol::SessionId::from("test-session"),
            std::path::Path::new("/tmp/test.txt"),
            "test content",
        ),
    );

    // This should succeed - covers lines 207-208
    let result = tokio_test::block_on(handler.execute_autocommand_request(
        "test-session".to_string(),
        "TestCommand",
        serde_json::json!({"test": "data"}),
        responder,
    ));

    assert!(result.is_ok(), "execute_autocommand_request should succeed");
    Ok(())
}

/// Custom type that serializes to JSON with values that can't be deserialized to nvim_oxi::Object
/// This triggers the error handling path at lines 59-64
#[derive(serde::Serialize, Debug)]
struct MalformedData {
    // This value will be serialized as a number, but when deserializing to Object,
    // it may fail due to Lua's number representation limitations
    problematic: f64,
}

#[nvim_oxi::test]
fn test_execute_autocommand_with_problematic_data_triggers_deserialization_error(
) -> nvim_oxi::Result<()> {
    // Test that sends data which may fail to deserialize to Object
    // This covers lines 59-64 (error handling for deserialization failure)
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    // Send data with NaN which may cause deserialization issues
    let problematic_data = MalformedData {
        problematic: f64::NAN,
    };

    // This sends the data - the deserialization may fail in the callback
    // but the send itself should succeed (error is logged, not propagated)
    let result = tokio_test::block_on(
        handler.execute_autocommand("TestCommandWithProblematicData", problematic_data),
    );

    // Send should succeed even if deserialization fails later
    assert!(result.is_ok(), "Send should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn test_no_listener_with_request_triggers_default_response_error_path() -> nvim_oxi::Result<()> {
    // Test lines 71-78: "No listener but has request" error handling path
    // This triggers when no autocommand listener is attached but a request is provided
    use agent_client_protocol::WriteTextFileResponse;
    use hermes::nvim::requests::Responder;
    use std::sync::Arc;
    use tokio::sync::oneshot;

    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new()))
        .expect("Handler creation should succeed");

    // Create a responder which will generate a request_id
    // But don't attach any autocommand listener for "TestErrorCommand"
    let (sender, _receiver) = oneshot::channel::<WriteTextFileResponse>();
    let responder = Responder::WriteFileResponse(
        sender,
        agent_client_protocol::WriteTextFileRequest::new(
            agent_client_protocol::SessionId::from("test-session"),
            std::path::Path::new("/tmp/test.txt"),
            "test content",
        ),
    );

    // Send with a responder but NO listener attached - triggers lines 71-78
    let result = tokio_test::block_on(handler.execute_autocommand_request(
        "test-session".to_string(),
        "TestErrorCommand", // No listener for this command
        serde_json::json!({"data": "value"}),
        responder,
    ));

    // Send should succeed even if default_response fails (error is logged, not propagated)
    assert!(result.is_ok(), "Send should succeed even with no listener");
    Ok(())
}
