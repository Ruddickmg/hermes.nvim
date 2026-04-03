use crate::{acp::connection::Assistant, nvim::{configuration::ClientConfig, state::agent::AgentInfo}};
use agent_client_protocol::InitializeResponse;
use tracing::{debug, instrument};

pub mod agent;

#[derive(Debug)]
pub struct PluginState {
    pub config: ClientConfig,
    pub agent_info: AgentInfo,
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
            agent_info: AgentInfo::default(),
        }
    }

    #[instrument(level = "trace")]
    pub fn set_agent(&mut self, agent: Assistant) -> &mut Self {
        self.agent_info.set_agent(agent.clone());
        debug!("Updated current agent to: '{}'", agent);
        self
    }

    #[instrument(level = "trace")]
    pub fn set_agent_info(&mut self, agent: Assistant, info: InitializeResponse) -> &mut Self {
        self.agent_info.add_agent(agent.clone(), info.clone());
        debug!("Updated information for '{}': {:#?}", agent, info);
        self
    }
}

impl Default for PluginState {
    fn default() -> Self {
        Self::new()
    }
}
