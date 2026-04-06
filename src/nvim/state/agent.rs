use std::collections::HashMap;

use agent_client_protocol::InitializeResponse;

use crate::acp::connection::Assistant;

#[derive(Clone, Debug, Default)]
pub struct AgentInfo {
    pub current: Assistant,
    agents: HashMap<Assistant, InitializeResponse>,
}

impl AgentInfo {
    fn notify_user(&self, allowed: bool, capability: &str) -> bool {
        if !allowed {
            tracing::warn!(
                "The {} agent does not support {}, skipping",
                self.current,
                capability
            );
        }
        allowed
    }
    pub fn get_current_info(&self) -> Option<&InitializeResponse> {
        self.agents.get(&self.current)
    }

    pub fn get_capabilities(&self) -> Option<&agent_client_protocol::AgentCapabilities> {
        self.get_current_info().map(|info| &info.agent_capabilities)
    }

    pub fn set_agent(&mut self, agent: Assistant) {
        self.current = agent;
    }

    pub fn add_agent(&mut self, agent: Assistant, info: InitializeResponse) {
        self.agents.insert(agent, info);
    }

    pub fn can_load_session(&self) -> bool {
        let allowed = self
            .get_capabilities()
            .map(|capabilities| capabilities.load_session)
            .unwrap_or(false);
        self.notify_user(allowed, "loading sessions")
    }

    pub fn can_send_images(&self) -> bool {
        let allowed = self
            .get_capabilities()
            .map(|capabilities| capabilities.prompt_capabilities.image)
            .unwrap_or(false);
        self.notify_user(allowed, "images")
    }

    pub fn can_send_audio(&self) -> bool {
        let allowed = self
            .get_capabilities()
            .map(|capabilities| capabilities.prompt_capabilities.audio)
            .unwrap_or(false);
        self.notify_user(allowed, "audio")
    }

    pub fn can_send_embedded_context(&self) -> bool {
        let allowed = self
            .get_capabilities()
            .map(|capabilities| capabilities.prompt_capabilities.embedded_context)
            .unwrap_or(false);
        self.notify_user(allowed, "embedded context")
    }

    pub fn can_connect_to_mcp_over_http(&self) -> bool {
        let allowed = self
            .get_capabilities()
            .map(|capabilities| capabilities.mcp_capabilities.http)
            .unwrap_or(false);
        self.notify_user(allowed, "connecting to MCP servers over HTTP")
    }

    pub fn can_connect_to_mcp_over_sse(&self) -> bool {
        let allowed = self
            .get_capabilities()
            .map(|capabilities| capabilities.mcp_capabilities.sse)
            .unwrap_or(false);
        self.notify_user(allowed, "connecting to MCP servers over SSE")
    }

    pub fn can_fork_sessions(&self) -> bool {
        let allowed = self
            .get_capabilities()
            .map(|capabilities| capabilities.session_capabilities.fork.is_some())
            .unwrap_or(false);
        self.notify_user(allowed, "forking sessions")
    }

    pub fn can_resume_sessions(&self) -> bool {
        let allowed = self
            .get_capabilities()
            .map(|capabilities| capabilities.session_capabilities.resume.is_some())
            .unwrap_or(false);
        self.notify_user(allowed, "resuming sessions")
    }

    pub fn can_list_sessions(&self) -> bool {
        let allowed = self
            .get_capabilities()
            .map(|capabilities| capabilities.session_capabilities.list.is_some())
            .unwrap_or(false);
        self.notify_user(allowed, "listing sessions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::{
        AgentCapabilities, McpCapabilities, PromptCapabilities, ProtocolVersion,
        SessionCapabilities, SessionForkCapabilities, SessionListCapabilities,
        SessionResumeCapabilities,
    };
    use pretty_assertions::assert_eq;

    fn create_test_response() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1)
    }

    fn create_agent_info_with_agent(agent: Assistant) -> AgentInfo {
        let mut info = AgentInfo {
            current: agent.clone(),
            ..Default::default()
        };
        info.add_agent(agent, create_test_response());
        info
    }

    #[test]
    fn test_default_agent_info() {
        let info = AgentInfo::default();
        assert_eq!(info.current, Assistant::default());
        assert!(info.get_current_info().is_none());
        assert!(info.get_capabilities().is_none());
    }

    #[test]
    fn test_set_agent_changes_current() {
        let mut info = AgentInfo::default();
        info.set_agent(Assistant::Opencode);
        assert_eq!(info.current, Assistant::Opencode);
        info.set_agent(Assistant::Copilot);
        assert_eq!(info.current, Assistant::Copilot);
    }

    #[test]
    fn test_add_agent_stores_info() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        let response = create_test_response();

        info.add_agent(agent.clone(), response);
        info.current = agent.clone();

        let stored = info.get_current_info();
        assert!(stored.is_some());
    }

    #[test]
    fn test_get_current_info_returns_none_for_unknown_agent() {
        let info = AgentInfo::default();
        assert!(info.get_current_info().is_none());
    }

    #[test]
    fn test_get_capabilities_returns_none_when_no_info() {
        let info = AgentInfo::default();
        assert!(info.get_capabilities().is_none());
    }

    #[test]
    fn test_get_capabilities_returns_some_when_info_exists() {
        let info = create_agent_info_with_agent(Assistant::Opencode);
        assert!(info.get_capabilities().is_some());
    }

    #[test]
    fn test_capabilities_return_false_by_default() {
        // Default response has all capabilities disabled
        let info = create_agent_info_with_agent(Assistant::Opencode);

        assert!(!info.can_load_session());
        assert!(!info.can_send_images());
        assert!(!info.can_send_audio());
        assert!(!info.can_send_embedded_context());
        assert!(!info.can_connect_to_mcp_over_http());
        assert!(!info.can_connect_to_mcp_over_sse());
        assert!(!info.can_fork_sessions());
        assert!(!info.can_resume_sessions());
        assert!(!info.can_list_sessions());
    }

    #[test]
    fn test_capabilities_return_false_when_no_info() {
        let info = AgentInfo::default();

        assert!(!info.can_load_session());
        assert!(!info.can_send_images());
        assert!(!info.can_send_audio());
        assert!(!info.can_send_embedded_context());
        assert!(!info.can_connect_to_mcp_over_http());
        assert!(!info.can_connect_to_mcp_over_sse());
        assert!(!info.can_fork_sessions());
        assert!(!info.can_resume_sessions());
        assert!(!info.can_list_sessions());
    }

    #[test]
    fn test_switching_agents_returns_correct_info() {
        let mut info = AgentInfo::default();

        // Add first agent
        let agent1 = Assistant::Opencode;
        info.add_agent(agent1.clone(), create_test_response());

        // Add second agent
        let agent2 = Assistant::Copilot;
        info.add_agent(agent2.clone(), create_test_response());

        // Switch to agent1
        info.set_agent(agent1.clone());
        assert!(info.get_current_info().is_some());

        // Switch to agent2
        info.set_agent(agent2.clone());
        assert!(info.get_current_info().is_some());
    }

    #[test]
    fn test_add_agent_updates_existing_agent() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;

        // Add agent
        info.add_agent(agent.clone(), create_test_response());
        info.current = agent.clone();

        assert!(info.get_current_info().is_some());

        // Update agent with new response
        let new_response = InitializeResponse::new(ProtocolVersion::LATEST);
        info.add_agent(agent.clone(), new_response);

        // Should still have the info
        assert!(info.get_current_info().is_some());
    }

    #[test]
    fn test_multiple_agents_stored_independently() {
        let mut info = AgentInfo::default();

        let agent1 = Assistant::Opencode;
        let agent2 = Assistant::Copilot;
        let agent3 = Assistant::Gemini;

        info.add_agent(agent1.clone(), create_test_response());
        info.add_agent(agent2.clone(), create_test_response());
        info.add_agent(agent3.clone(), create_test_response());

        // Verify all agents are stored
        info.set_agent(agent1);
        assert!(info.get_current_info().is_some());

        info.set_agent(agent2);
        assert!(info.get_current_info().is_some());

        info.set_agent(agent3);
        assert!(info.get_current_info().is_some());
    }

    #[test]
    fn test_current_agent_defaults_to_default_assistant() {
        let info = AgentInfo::default();
        // Default is Copilot based on Assistant::default()
        assert_eq!(info.current, Assistant::default());
    }

    #[test]
    fn test_get_current_info_after_adding_different_agent() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;

        // Add agent but don't change current
        info.add_agent(agent.clone(), create_test_response());

        // Current is still default, so get_current_info should return None
        assert!(info.get_current_info().is_none());

        // After setting current, should return Some
        info.set_agent(agent);
        assert!(info.get_current_info().is_some());
    }

    // Positive capability tests - verify filters return true when enabled
    fn create_response_with_load_session_enabled() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1)
            .agent_capabilities(AgentCapabilities::new().load_session(true))
    }

    #[test]
    fn test_can_load_session_returns_true_when_enabled() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        info.add_agent(agent.clone(), create_response_with_load_session_enabled());
        info.set_agent(agent);
        assert_eq!(info.can_load_session(), true);
    }

    fn create_response_with_images_enabled() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
            AgentCapabilities::new().prompt_capabilities(PromptCapabilities::new().image(true)),
        )
    }

    #[test]
    fn test_can_send_images_returns_true_when_enabled() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        info.add_agent(agent.clone(), create_response_with_images_enabled());
        info.set_agent(agent);
        assert_eq!(info.can_send_images(), true);
    }

    fn create_response_with_audio_enabled() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
            AgentCapabilities::new().prompt_capabilities(PromptCapabilities::new().audio(true)),
        )
    }

    #[test]
    fn test_can_send_audio_returns_true_when_enabled() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        info.add_agent(agent.clone(), create_response_with_audio_enabled());
        info.set_agent(agent);
        assert_eq!(info.can_send_audio(), true);
    }

    fn create_response_with_embedded_context_enabled() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
            AgentCapabilities::new()
                .prompt_capabilities(PromptCapabilities::new().embedded_context(true)),
        )
    }

    #[test]
    fn test_can_send_embedded_context_returns_true_when_enabled() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        info.add_agent(
            agent.clone(),
            create_response_with_embedded_context_enabled(),
        );
        info.set_agent(agent);
        assert_eq!(info.can_send_embedded_context(), true);
    }

    fn create_response_with_mcp_http_enabled() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
            AgentCapabilities::new().mcp_capabilities(McpCapabilities::new().http(true)),
        )
    }

    #[test]
    fn test_can_connect_to_mcp_over_http_returns_true_when_enabled() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        info.add_agent(agent.clone(), create_response_with_mcp_http_enabled());
        info.set_agent(agent);
        assert_eq!(info.can_connect_to_mcp_over_http(), true);
    }

    fn create_response_with_mcp_sse_enabled() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
            AgentCapabilities::new().mcp_capabilities(McpCapabilities::new().sse(true)),
        )
    }

    #[test]
    fn test_can_connect_to_mcp_over_sse_returns_true_when_enabled() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        info.add_agent(agent.clone(), create_response_with_mcp_sse_enabled());
        info.set_agent(agent);
        assert_eq!(info.can_connect_to_mcp_over_sse(), true);
    }

    fn create_response_with_fork_enabled() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
            AgentCapabilities::new().session_capabilities(
                SessionCapabilities::new().fork(Some(SessionForkCapabilities::new())),
            ),
        )
    }

    #[test]
    fn test_can_fork_sessions_returns_true_when_enabled() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        info.add_agent(agent.clone(), create_response_with_fork_enabled());
        info.set_agent(agent);
        assert_eq!(info.can_fork_sessions(), true);
    }

    fn create_response_with_resume_enabled() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
            AgentCapabilities::new().session_capabilities(
                SessionCapabilities::new().resume(Some(SessionResumeCapabilities::new())),
            ),
        )
    }

    #[test]
    fn test_can_resume_sessions_returns_true_when_enabled() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        info.add_agent(agent.clone(), create_response_with_resume_enabled());
        info.set_agent(agent);
        assert_eq!(info.can_resume_sessions(), true);
    }

    fn create_response_with_list_enabled() -> InitializeResponse {
        InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
            AgentCapabilities::new().session_capabilities(
                SessionCapabilities::new().list(Some(SessionListCapabilities::new())),
            ),
        )
    }

    #[test]
    fn test_can_list_sessions_returns_true_when_enabled() {
        let mut info = AgentInfo::default();
        let agent = Assistant::Opencode;
        info.add_agent(agent.clone(), create_response_with_list_enabled());
        info.set_agent(agent);
        assert_eq!(info.can_list_sessions(), true);
    }
}
