use agent_client_protocol::{
    AuthenticateResponse, ExtResponse, ForkSessionResponse, ListSessionsResponse,
    LoadSessionResponse, NewSessionResponse, PromptResponse, ResumeSessionResponse,
    SetSessionConfigOptionResponse, SetSessionModeResponse, SetSessionModelResponse,
};

use crate::Handler;
use crate::apc::error::Error;

impl<H: agent_client_protocol::Client> Handler<H> {
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
