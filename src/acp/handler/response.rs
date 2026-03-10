use agent_client_protocol::{
    AuthenticateResponse, ExtResponse, ForkSessionResponse, InitializeResponse,
    ListSessionsResponse, LoadSessionResponse, NewSessionResponse, PromptResponse,
    ResumeSessionResponse, SetSessionConfigOptionResponse, SetSessionModeResponse,
    SetSessionModelResponse,
};
use tracing::instrument;

use crate::acp::connection::Assistant;
use crate::acp::error::Error;
use crate::nvim::autocommands::Commands;
use crate::{Handler, nvim::autocommands::ResponseHandler};

impl<H: agent_client_protocol::Client + ResponseHandler> Handler<H> {
    #[instrument(level = "trace", skip(self))]
    pub async fn initialized(
        &self,
        agent: &Assistant,
        info: InitializeResponse,
    ) -> Result<(), Error> {
        self.set_agent_info(agent.clone(), info.clone()).await;

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
        self.handler
            .schedule_autocommand(Commands::ConnectionInitialized, value)
            .await
    }
    #[instrument(level = "trace", skip(self))]
    pub async fn session_created(&self, session: NewSessionResponse) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::CreatedSession, session)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn prompted(&self, response: PromptResponse) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::Prompted, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn authenticated(&self, response: AuthenticateResponse) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::Authenticated, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn config_option_set(
        &self,
        response: SetSessionConfigOptionResponse,
    ) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::ConfigurationUpdated, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn mode_set(&self, response: SetSessionModeResponse) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::ModeUpdated, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_loaded(&self, response: LoadSessionResponse) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::LoadedSession, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn custom_command_executed(&self, _response: ExtResponse) -> Result<(), Error> {
        Ok(())
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn sessions_listed(&self, response: ListSessionsResponse) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::ListedSessions, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_forked(&self, response: ForkSessionResponse) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::ForkedSession, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_resumed(&self, response: ResumeSessionResponse) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::ResumedSession, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_model_set(&self, response: SetSessionModelResponse) -> Result<(), Error> {
        self.handler
            .schedule_autocommand(Commands::SessionModelUpdated, response)
            .await
    }
}
