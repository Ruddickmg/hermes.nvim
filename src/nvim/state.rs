use crate::{acp::connection::Assistant, nvim::configuration::ClientConfig};
use agent_client_protocol::InitializeResponse;
use std::collections::HashMap;
use tracing::{debug, instrument};

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

    #[instrument(level = "trace")]
    pub fn set_agent(&mut self, agent: Assistant) -> &mut Self {
        self.agent = agent.clone();
        debug!("Updated current agent to: '{}'", agent);
        self
    }

    #[instrument(level = "trace")]
    pub fn set_agent_info(&mut self, agent: Assistant, info: InitializeResponse) -> &mut Self {
        self.agent_info.insert(agent.clone(), info.clone());
        debug!("Upated information for '{}': {:?}", agent, info);
        self
    }
}

impl Default for PluginState {
    fn default() -> Self {
        Self::new()
    }
}
