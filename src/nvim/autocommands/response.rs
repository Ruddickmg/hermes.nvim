use crate::apc::{self, Result};
use crate::nvim::autocommands::{AutoCommands, Commands};
use crate::nvim::parse::response;
use agent_client_protocol::{
    AuthenticateResponse, ExtResponse, ForkSessionResponse, InitializeResponse,
    ListSessionsResponse, LoadSessionResponse, NewSessionResponse, PromptResponse,
    ResumeSessionResponse, SetSessionConfigOptionResponse, SetSessionModeResponse,
    SetSessionModelResponse,
};

#[async_trait::async_trait(?Send)]
pub trait ResponseHandler {
    async fn initialized(&self, info: InitializeResponse) -> Result<()>;
    async fn session_created(&self, response: NewSessionResponse) -> Result<()>;
    async fn prompted(&self, response: PromptResponse) -> Result<()>;
    async fn authenticated(&self, response: AuthenticateResponse) -> Result<()>;
    async fn config_option_set(&self, response: SetSessionConfigOptionResponse) -> Result<()>;
    async fn mode_set(&self, response: SetSessionModeResponse) -> Result<()>;
    async fn session_loaded(&self, response: LoadSessionResponse) -> Result<()>;
    async fn custom_command_executed(&self, response: ExtResponse) -> Result<()>;
    async fn sessions_listed(&self, response: ListSessionsResponse) -> Result<()>;
    async fn session_forked(&self, response: ForkSessionResponse) -> Result<()>;
    async fn session_resumed(&self, response: ResumeSessionResponse) -> Result<()>;
    async fn session_model_set(&self, response: SetSessionModelResponse) -> Result<()>;
}

#[async_trait::async_trait(?Send)]
impl ResponseHandler for AutoCommands {
    async fn initialized(&self, info: InitializeResponse) -> Result<()> {
        let c = info.clone();
        // TODO: figure out a better way to deal with the deserialization issue with the protocol version
        let value = serde_json::json!({
            "protocolVersion": info.protocol_version.to_string(),
            "agentCapabilities": {
                "loadSession": info.agent_capabilities.load_session,
                "promptCapabilities": {
                    "image": info.agent_capabilities.prompt_capabilities.image,
                    "audio": info.agent_capabilities.prompt_capabilities.audio,
                    "embeddedContext": info.agent_capabilities.prompt_capabilities.embedded_context,
                },
                "mcpCapabilities": {
                    "http": info.agent_capabilities.mcp_capabilities.http,
                    "sse": info.agent_capabilities.mcp_capabilities.sse,
                },
                "sessionCapabilities": {
                    "list": info.agent_capabilities.session_capabilities.list,
                    "fork": info.agent_capabilities.session_capabilities.fork,
                    "resume": info.agent_capabilities.session_capabilities.resume,
                },
            },
            "authMethods": info.auth_methods.iter().map(|m| serde_json::json!({
                "id": m.id.0,
                "name": m.name,
                "description": m.description,
            })).collect::<Vec<_>>(),
            "agentInfo": info.agent_info.map(|i| serde_json::json!({
                "name": i.name,
                "version": i.version,
                "title": i.title,
            })),
        });
        self.schedule_autocommand(Commands::AgentConnectionInitialized, c).await?;
        Ok(())
    }

    async fn session_created(&self, response: NewSessionResponse) -> Result<()> {
        self.schedule_autocommand(Commands::CreatedSession, response).await
    }

    async fn prompted(&self, response: PromptResponse) -> Result<()> {
        self.schedule_autocommand(Commands::AgentPrompted, response).await
    }

    async fn authenticated(&self, response: AuthenticateResponse) -> Result<()> {
        self.schedule_autocommand(Commands::ClientAuthenticated, response).await
    }

    async fn config_option_set(&self, response: SetSessionConfigOptionResponse) -> Result<()> {
        self.schedule_autocommand(Commands::AgentConfigUpdated, response).await
    }

    async fn mode_set(&self, response: SetSessionModeResponse) -> Result<()> {
        self.schedule_autocommand(Commands::ModeUpdated, response).await
    }

    async fn session_loaded(&self, response: LoadSessionResponse) -> Result<()> {
        self.schedule_autocommand(Commands::LoadedSession, response).await
    }

    async fn custom_command_executed(&self, _response: ExtResponse) -> Result<()> {
        Ok(())
    }

    async fn sessions_listed(&self, response: ListSessionsResponse) -> Result<()> {
        self.schedule_autocommand(Commands::ListedSessions, response).await
    }

    async fn session_forked(&self, response: ForkSessionResponse) -> Result<()> {
        self.schedule_autocommand(Commands::ForkedSession, response).await
    }

    async fn session_resumed(&self, response: ResumeSessionResponse) -> Result<()> {
        self.schedule_autocommand(Commands::ResumedSession, response).await
    }

    async fn session_model_set(&self, response: SetSessionModelResponse) -> Result<()> {
        self.schedule_autocommand(Commands::SessionModelUpdated, response).await
    }
}
