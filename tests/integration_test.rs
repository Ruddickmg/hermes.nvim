//! Integration tests for the APC client
//!
//! These tests verify the complete integration of the APC client
//! with the agent-client-protocol library.

use agent_client_protocol::{
    Client, ContentBlock, ContentChunk, RequestPermissionRequest, RequestPermissionResponse,
    SessionId, SessionNotification, SessionUpdate, TextContent,
};
use async_trait::async_trait;
use hermes::{ClientConfig, Handler, PluginState};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct MockHandler;

#[async_trait(?Send)]
impl Client for MockHandler {
    async fn request_permission(
        &self,
        _args: RequestPermissionRequest,
    ) -> agent_client_protocol::Result<RequestPermissionResponse> {
        Err(agent_client_protocol::Error::method_not_found())
    }

    async fn session_notification(
        &self,
        _args: SessionNotification,
    ) -> agent_client_protocol::Result<()> {
        Ok(())
    }
}

/// Tests that a client can be created with default configuration
#[test]
fn test_create_client_with_defaults() {
    let state = Arc::new(Mutex::new(PluginState::new()));
    let client = Handler::new(state, MockHandler);

    assert!(client.can_write());
    assert!(client.can_read());
    assert!(client.can_access_terminal());
}

/// Tests that a client can be created with custom configuration
#[test]
fn test_create_client_with_custom_config() {
    let config = ClientConfig {
        fs_write_access: false,
        fs_read_access: true,
        terminal_access: false,
    };
    let state = Arc::new(Mutex::new(PluginState::with_config(config)));
    let client = Handler::new(state, MockHandler);

    assert!(!client.can_write());
    assert!(client.can_read());
    assert!(!client.can_access_terminal());
}

/// Tests that session notifications are handled correctly
#[tokio::test]
async fn test_handle_session_notification() {
    let state = Arc::new(Mutex::new(PluginState::new()));
    let client = Handler::new(state, MockHandler);

    let notification = SessionNotification::new(
        SessionId::new("test-session-123"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "Test message",
        )))),
    );

    let result = client.session_notification(notification).await;
    assert!(result.is_ok());
}

/// Tests that multiple session notifications can be handled sequentially
#[tokio::test]
async fn test_handle_multiple_notifications() {
    let state = Arc::new(Mutex::new(PluginState::new()));
    let client = Handler::new(state, MockHandler);

    // Send first notification
    let notif1 = SessionNotification::new(
        SessionId::new("session-1"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "First message",
        )))),
    );
    let result1 = client.session_notification(notif1).await;
    assert!(result1.is_ok());

    // Send second notification
    let notif2 = SessionNotification::new(
        SessionId::new("session-1"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "Second message",
        )))),
    );
    let result2 = client.session_notification(notif2).await;
    assert!(result2.is_ok());

    // Send third notification with different session
    let notif3 = SessionNotification::new(
        SessionId::new("session-2"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "Different session",
        )))),
    );
    let result3 = client.session_notification(notif3).await;
    assert!(result3.is_ok());
}

/// Tests that different types of session updates can be handled
#[tokio::test]
async fn test_handle_different_update_types() {
    let state = Arc::new(Mutex::new(PluginState::new()));
    let client = Handler::new(state, MockHandler);
    let session_id = SessionId::new("test-session");

    // Test agent message chunk
    let agent_msg = SessionNotification::new(
        session_id.clone(),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "Agent response",
        )))),
    );
    let result1 = client.session_notification(agent_msg).await;
    assert!(result1.is_ok());

    // Test user message chunk
    let user_msg = SessionNotification::new(
        session_id.clone(),
        SessionUpdate::UserMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "User input",
        )))),
    );
    let result2 = client.session_notification(user_msg).await;
    assert!(result2.is_ok());

    // Test agent thought chunk
    let thought = SessionNotification::new(
        session_id,
        SessionUpdate::AgentThoughtChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "Internal reasoning",
        )))),
    );
    let result3 = client.session_notification(thought).await;
    assert!(result3.is_ok());
}

/// Tests that client configuration affects capabilities
#[test]
fn test_client_capabilities() {
    // Client with all capabilities enabled
    let full_config = ClientConfig {
        fs_write_access: true,
        fs_read_access: true,
        terminal_access: true,
    };
    let full_state = Arc::new(Mutex::new(PluginState::with_config(full_config)));
    let full_client = Handler::new(full_state, MockHandler.clone());
    assert!(full_client.can_write());
    assert!(full_client.can_read());
    assert!(full_client.can_access_terminal());

    // Client with limited capabilities
    let limited_config = ClientConfig {
        fs_write_access: false,
        fs_read_access: false,
        terminal_access: false,
    };
    let limited_state = Arc::new(Mutex::new(PluginState::with_config(limited_config)));
    let limited_client = Handler::new(limited_state, MockHandler);
    assert!(!limited_client.can_write());
    assert!(!limited_client.can_read());
    assert!(!limited_client.can_access_terminal());
}

/// Tests that client is cloneable for sharing across tasks
#[tokio::test]
async fn test_client_cloneable() {
    let state = Arc::new(Mutex::new(PluginState::new()));
    let client = Handler::new(state, MockHandler);
    let client_clone = client.clone();

    // Both instances should work independently
    let notif = SessionNotification::new(
        SessionId::new("test"),
        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
            "Test",
        )))),
    );

    let result1: agent_client_protocol::Result<()> =
        client.session_notification(notif.clone()).await;
    assert!(result1.is_ok());

    let result2: agent_client_protocol::Result<()> = client_clone.session_notification(notif).await;
    assert!(result2.is_ok());
}
