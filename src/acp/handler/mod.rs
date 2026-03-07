pub mod client;
pub mod message;
pub mod response;

use crate::{PluginState, acp::connection::Assistant, nvim::autocommands::ResponseHandler};
use agent_client_protocol::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

#[derive(Clone)]
pub struct Handler<H: Client> {
    pub state: Arc<Mutex<PluginState>>,
    handler: H,
}

impl<H: Client + ResponseHandler> Handler<H> {
    #[instrument(level = "trace", skip(handler))]
    pub fn new(state: Arc<Mutex<PluginState>>, handler: H) -> Self {
        Self { state, handler }
    }

    #[instrument(level = "trace", skip(self))]
    pub fn can_write(&self) -> bool {
        self.state
            .blocking_lock()
            .config
            .permissions
            .fs_write_access
    }

    #[instrument(level = "trace", skip(self))]
    pub fn can_read(&self) -> bool {
        self.state.blocking_lock().config.permissions.fs_read_access
    }

    #[instrument(level = "trace", skip(self))]
    pub fn can_access_terminal(&self) -> bool {
        self.state
            .blocking_lock()
            .config
            .permissions
            .terminal_access
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
}
