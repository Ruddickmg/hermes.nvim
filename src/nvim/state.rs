use crate::{ClientConfig, apc::connection::Assistant};
use agent_client_protocol::InitializeResponse;
use std::collections::HashMap;

pub struct PluginState {
    pub config: ClientConfig,
    pub agent_info: HashMap<Assistant, InitializeResponse>,
    pub agent: Assistant,
}

impl PluginState {
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

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
