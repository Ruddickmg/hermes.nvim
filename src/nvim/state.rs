use crate::{acp::connection::Assistant, nvim::configuration::ClientConfig};
use agent_client_protocol::InitializeResponse;
use tracing::instrument;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PluginState {
    pub config: ClientConfig,
    pub agent_info: HashMap<Assistant, InitializeResponse>,
    pub agent: Assistant,
}

impl PluginState {
    #[instrument(level = "trace")]
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

    #[instrument(level = "trace")]
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            config,
            agent_info: HashMap::new(),
            agent: Assistant::default(),
        }
    }
}

impl Default for PluginState {
    fn default() -> Self {
        Self::new()
    }
}
