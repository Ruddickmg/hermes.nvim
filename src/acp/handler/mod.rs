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
