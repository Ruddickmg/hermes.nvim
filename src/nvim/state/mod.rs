use crate::acp::registry::Registry;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::acp::connection::Assistant;
use crate::{
    acp::session_info::SessionDetails,
    nvim::{configuration::ClientConfig, state::agent::AgentInfo},
};
use agent_client_protocol::schema::v1::{InitializeResponse, NewSessionResponse};
use tracing::{debug, instrument};

pub mod agent;

#[derive(Debug)]
pub struct PluginState {
    pub config: ClientConfig,
    pub prompt: HashMap<String, String>,
    pub agent_info: AgentInfo,
    pub session_info: HashMap<String, SessionDetails>,
    pub registry: Option<Registry>,
}

impl PluginState {
    #[instrument(level = "trace")]
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

    #[instrument(level = "trace")]
    pub fn with_storage_path(mut self, storage_path: PathBuf) -> Self {
        let history_path = storage_path.join("history");
        self.agent_info.set_history(history_path);
        let binary_path = storage_path.join("binaries");
        self.config.distributions.binary.path = binary_path.to_string_lossy().to_string();
        self
    }

    #[instrument(level = "trace")]
    pub fn with_registry(mut self, registry: Option<Registry>) -> Self {
        self.registry = registry;
        self
    }

    #[instrument(level = "trace")]
    pub fn set_session_info(&mut self, session: &NewSessionResponse) -> &mut Self {
        self.session_info
            .insert(session.session_id.to_string(), SessionDetails::new(session));
        self
    }

    #[instrument(level = "trace")]
    pub fn get_session_info(&self, session_id: &str) -> Option<&SessionDetails> {
        self.session_info.get(session_id)
    }

    #[instrument(level = "trace")]
    pub fn get_session_info_mut(&mut self, session_id: &str) -> Option<&mut SessionDetails> {
        self.session_info.get_mut(session_id)
    }

    #[instrument(level = "trace")]
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            config,
            prompt: HashMap::new(),
            session_info: HashMap::new(),
            agent_info: AgentInfo::new(),
            registry: None,
        }
    }

    #[instrument(level = "trace")]
    pub fn update_session_prompt_id(&mut self, session_id: String, prompt_id: String) -> &mut Self {
        self.prompt.insert(session_id.clone(), prompt_id.clone());
        debug!("Updated prompt for session '{}'", session_id);
        self
    }

    #[instrument(level = "trace")]
    pub fn get_session_prompt(&mut self, session_id: &str) -> String {
        if let Some(prompt_id) = self.prompt.get(session_id) {
            prompt_id.to_string()
        } else {
            let prompt_id = uuid::Uuid::new_v4().to_string();
            self.prompt
                .insert(session_id.to_string(), prompt_id.to_string());
            prompt_id
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

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{
        NewSessionResponse, SessionConfigOption, SessionConfigOptionCategory,
        SessionConfigSelectOption, SessionMode, SessionModeState,
    };
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[test]
    fn get_session_prompt_generates_and_caches_uuid_for_new_session() {
        let mut state = PluginState::default();

        let first_id = state.get_session_prompt("session-1");
        let second_id = state.get_session_prompt("session-1");

        assert_eq!(
            first_id, second_id,
            "Should return the same cached id on repeated calls"
        );
    }

    #[test]
    fn set_session_info_stores_legacy_mode_details() {
        let mut state = PluginState::default();

        let mode = SessionMode::new("chat", "Chat");
        let modes = SessionModeState::new("chat", vec![mode]);
        let session = NewSessionResponse::new("test-session").modes(modes);

        state.set_session_info(&session);

        let details = state.session_info.get("test-session").unwrap();
        assert_eq!(details.mode_is_legacy(), Some(true));
    }

    #[test]
    fn set_session_info_stores_new_mode_details() {
        let mut state = PluginState::default();

        let option = SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode);

        let session = NewSessionResponse::new("test-session").config_options(vec![option]);

        state.set_session_info(&session);

        let details = state.session_info.get("test-session").unwrap();
        assert_eq!(details.mode_is_legacy(), Some(false));
    }

    #[test]
    fn set_session_info_stores_none_when_no_modes() {
        let mut state = PluginState::default();

        let session = NewSessionResponse::new("test-session");
        state.set_session_info(&session);

        let details = state.session_info.get("test-session").unwrap();
        assert_eq!(details.mode_is_legacy(), None);
    }

    #[test]
    fn with_storage_path_initializes_history_writer() {
        let temp_dir = TempDir::new().unwrap();
        let state = PluginState::new().with_storage_path(temp_dir.path().to_path_buf());

        state
            .agent_info
            .history
            .write_keyed("agent/session.jsonl", "test");
    }

    #[test]
    fn new_plugin_state_registry_is_none() {
        let state = PluginState::new();
        assert!(state.registry.is_none());
    }

    #[test]
    fn with_registry_none_leaves_registry_unset() {
        let state = PluginState::new().with_registry(None);
        assert!(state.registry.is_none());
    }

    #[test]
    fn get_session_info_returns_none_for_unknown_session() {
        let state = PluginState::default();
        assert!(state.get_session_info("unknown-session").is_none());
    }

    #[test]
    fn get_session_info_returns_some_for_known_session() {
        let mut state = PluginState::default();
        let session = NewSessionResponse::new("known-session");
        state.set_session_info(&session);
        assert!(state.get_session_info("known-session").is_some());
    }

    #[test]
    fn set_agent_changes_current_agent() {
        let mut state = PluginState::default();
        state.set_agent(Assistant::Opencode);
        assert_eq!(state.agent_info.current, Assistant::Opencode);
    }
}
