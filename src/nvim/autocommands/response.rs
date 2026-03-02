use crate::nvim::autocommands::AutoCommands;
use agent_client_protocol::{
    AuthenticateResponse, ExtResponse, ForkSessionResponse, InitializeResponse,
    ListSessionsResponse, LoadSessionResponse, NewSessionResponse, PromptResponse, Result,
    ResumeSessionResponse, SetSessionConfigOptionResponse, SetSessionModeResponse,
    SetSessionModelResponse,
};

#[async_trait::async_trait(?Send)]
pub trait ResponseHandler {
    async fn initialized(&self, info: InitializeResponse) -> ();
    async fn session_created(&self, response: NewSessionResponse) -> ();
    async fn prompted(&self, response: PromptResponse) -> ();
    async fn authenticated(&self, response: AuthenticateResponse) -> ();
    async fn config_option_set(&self, response: SetSessionConfigOptionResponse) -> ();
    async fn mode_set(&self, response: SetSessionModeResponse) -> ();
    async fn session_loaded(&self, response: LoadSessionResponse) -> ();
    async fn custom_command_executed(&self, response: ExtResponse) -> ();
    async fn sessions_listed(&self, response: ListSessionsResponse) -> ();
    async fn session_forked(&self, response: ForkSessionResponse) -> ();
    async fn session_resumed(&self, response: ResumeSessionResponse) -> ();
    async fn session_model_set(&self, response: SetSessionModelResponse) -> ();
}

#[async_trait::async_trait(?Send)]
impl ResponseHandler for AutoCommands {
    async fn initialized(&self, info: InitializeResponse) -> () {
        let data = Object::from(info);
        self.schedule_autocommand("ConnectionInitialized", data)
    }
    async fn session_created(&self, _response: NewSessionResponse) -> () {
        Ok(())
    }

    async fn prompted(&self, _response: PromptResponse) -> () {
        Ok(())
    }

    async fn authenticated(&self, _response: AuthenticateResponse) -> () {
        Ok(())
    }

    async fn config_option_set(&self, _response: SetSessionConfigOptionResponse) -> () {
        Ok(())
    }

    async fn mode_set(&self, _response: SetSessionModeResponse) -> () {
        Ok(())
    }

    async fn session_loaded(&self, _response: LoadSessionResponse) -> () {
        Ok(())
    }

    async fn custom_command_executed(&self, _response: ExtResponse) -> () {
        Ok(())
    }

    async fn sessions_listed(&self, _response: ListSessionsResponse) -> () {
        Ok(())
    }

    async fn session_forked(&self, _response: ForkSessionResponse) -> () {
        Ok(())
    }

    async fn session_resumed(&self, _response: ResumeSessionResponse) -> () {
        Ok(())
    }

    async fn session_model_set(&self, _response: SetSessionModelResponse) -> () {
        Ok(())
    }
}
