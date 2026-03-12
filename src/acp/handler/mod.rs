pub mod client;
pub mod message;
pub mod response;

use crate::{PluginState, acp::connection::Assistant, nvim::autocommands::ResponseHandler};
use agent_client_protocol::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

#[derive(Clone)]
pub struct Handler<H: Client + ResponseHandler> {
    pub state: Arc<Mutex<PluginState>>,
    handler: H,
}

impl<H: Client + ResponseHandler> Handler<H> {
    #[instrument(level = "trace", skip(handler))]
    pub fn new(state: Arc<Mutex<PluginState>>, handler: H) -> Self {
        Self { state, handler }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_write(&self) -> bool {
        let state = self.state.lock().await;
        let write_access = state.config.permissions.fs_write_access;
        drop(state);
        write_access
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_read(&self) -> bool {
        let state = self.state.lock().await;
        let read_access = state.config.permissions.fs_read_access;
        drop(state);
        read_access
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_access_terminal(&self) -> bool {
        let state = self.state.lock().await;
        let terminal_access = state.config.permissions.terminal_access;
        drop(state);
        terminal_access
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn get_agent(&self) -> Assistant {
        let state = self.state.lock().await;
        let agent = state.agent.clone();
        drop(state);
        agent
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn set_agent_info(
        &self,
        agent: Assistant,
        info: agent_client_protocol::InitializeResponse,
    ) {
        let mut config = self.state.lock().await;
        config.set_agent_info(agent.clone(), info.clone());
        drop(config);
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_request_permissions(&self) -> bool {
        let config = self.state.lock().await;
        let can_request_permissions = config.config.permissions.can_request_permissions;
        drop(config);
        can_request_permissions
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_receive_notifications(&self) -> bool {
        let config = self.state.lock().await;
        let allow_notifications = config.config.permissions.allow_notifications;
        drop(config);
        allow_notifications
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::handler::client::tests::MockClient;
    use crate::nvim::state::PluginState;
    use agent_client_protocol::{
        ContentBlock, ContentChunk, SessionNotification, SessionUpdate, TextContent,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn create_test_notification() -> SessionNotification {
        let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new("test message")));
        SessionNotification::new("session_id", SessionUpdate::UserMessageChunk(chunk))
    }

    #[tokio::test]
    async fn test_can_receive_notifications_returns_true_by_default() {
        let state = Arc::new(Mutex::new(PluginState::default()));
        let handler = Handler::new(state.clone(), MockClient::new());

        let result = handler.can_receive_notifications().await;
        assert_eq!(result, true);
    }

    #[tokio::test]
    async fn test_can_receive_notifications_returns_false_when_disabled() {
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.allow_notifications = false;

        let handler = Handler::new(state.clone(), MockClient::new());

        let result = handler.can_receive_notifications().await;
        assert_eq!(result, false);
    }

    #[tokio::test]
    async fn test_session_notification_permissions_allowed() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));

        let handler = Handler::new(state.clone(), mock.clone());

        let notification = create_test_notification();
        let res: Result<(), agent_client_protocol::Error> = handler.session_notification(notification).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_session_notification_calls_handler() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));

        let handler = Handler::new(state.clone(), mock.clone());

        let notification = create_test_notification();
        let _ = handler.session_notification(notification).await;
        assert!(*mock.notification_called.lock().await);
    }

    #[tokio::test]
    async fn test_session_notification_permissions_denied() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.allow_notifications = false;

        let handler = Handler::new(state.clone(), mock.clone());

        let notification = create_test_notification();
        let res = handler.session_notification(notification).await;
        assert_eq!(res, Err(agent_client_protocol::Error::method_not_found()));
    }

    #[tokio::test]
    async fn test_session_notification_permissions_denied_does_not_call_handler() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.allow_notifications = false;

        let handler = Handler::new(state.clone(), mock.clone());

        let notification = create_test_notification();
        let _ = handler.session_notification(notification).await;
        assert!(!*mock.notification_called.lock().await);
    }
}
