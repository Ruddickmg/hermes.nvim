use agent_client_protocol::{
    AuthenticateResponse, ExtResponse, ForkSessionResponse, InitializeResponse,
    ListSessionsResponse, LoadSessionResponse, NewSessionResponse, PromptResponse,
    ResumeSessionResponse, SetSessionConfigOptionResponse, SetSessionModeResponse,
    SetSessionModelResponse,
};

use crate::acp::error::Error;
use crate::{Handler, nvim::autocommands::ResponseHandler};

impl<H: agent_client_protocol::Client + ResponseHandler> Handler<H> {
    pub async fn initialized(&self, info: InitializeResponse) -> Result<(), Error> {
        let mut config = self.state.lock().await;
        let agent = config.agent.clone();
        config.agent_info.insert(agent, info.clone());
        drop(config);
        self.handler.initialized(info).await
    }
    pub async fn session_created(&self, _response: NewSessionResponse) -> Result<(), Error> {
        Ok(())
    }

    pub async fn prompted(&self, _response: PromptResponse) -> Result<(), Error> {
        Ok(())
    }

    pub async fn authenticated(&self, _response: AuthenticateResponse) -> Result<(), Error> {
        Ok(())
    }

    pub async fn config_option_set(
        &self,
        _response: SetSessionConfigOptionResponse,
    ) -> Result<(), Error> {
        Ok(())
    }

    pub async fn mode_set(&self, _response: SetSessionModeResponse) -> Result<(), Error> {
        Ok(())
    }

    pub async fn session_loaded(&self, _response: LoadSessionResponse) -> Result<(), Error> {
        Ok(())
    }

    pub async fn custom_command_executed(&self, _response: ExtResponse) -> Result<(), Error> {
        Ok(())
    }

    pub async fn sessions_listed(&self, _response: ListSessionsResponse) -> Result<(), Error> {
        Ok(())
    }

    pub async fn session_forked(&self, _response: ForkSessionResponse) -> Result<(), Error> {
        Ok(())
    }

    pub async fn session_resumed(&self, _response: ResumeSessionResponse) -> Result<(), Error> {
        Ok(())
    }

    pub async fn session_model_set(&self, _response: SetSessionModelResponse) -> Result<(), Error> {
        Ok(())
    }
}
