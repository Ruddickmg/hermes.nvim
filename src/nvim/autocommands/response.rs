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
    async fn initialized(&self, info: InitializeResponse) {
        let data = response::initialize_response(info);
        self.schedule_autocommand(Commands::AgentConnectionInitialized, data.into()).await;
        println!("did the thing");
    }

    async fn session_created(&self, response: NewSessionResponse) {
        let data = response::new_session_response(response);
        self.schedule_autocommand(Commands::CreatedSession, data.into()).await;
    }

    async fn prompted(&self, response: PromptResponse) {
        let data = response::prompt_response(response);
        self.schedule_autocommand("AgentPrompted", data.into()).await
    }

    async fn authenticated(&self, response: AuthenticateResponse) {
        let data = response::authenticate_response(response);
        self.schedule_autocommand("ClientAuthenticated", data.into()).await
    }

    async fn config_option_set(&self, response: SetSessionConfigOptionResponse) {
        let data = response::config_option_response(response);
        self.schedule_autocommand("AgentConfigUpdated", data.into()).await
    }

    async fn mode_set(&self, response: SetSessionModeResponse) {
        let data = response::mode_response(response);
        self.schedule_autocommand("ModeUpdated", data.into()).await
    }

    async fn session_loaded(&self, response: LoadSessionResponse) {
        let data = response::session_loaded_response(response);
        self.schedule_autocommand("LoadedSession", data.into()).await
    }

    async fn custom_command_executed(&self, _response: ExtResponse) {}

    async fn sessions_listed(&self, response: ListSessionsResponse) {
        let data = response::sessions_listed_response(response);
        self.schedule_autocommand("ListedSessions", data.into()).await
    }

    async fn session_forked(&self, response: ForkSessionResponse) {
        let data = response::session_forked_response(response);
        self.schedule_autocommand("ForkedSession", data.into()).await
    }

    async fn session_resumed(&self, response: ResumeSessionResponse) {
        let data = response::session_resumed_response(response);
        self.schedule_autocommand("ResumedSession", data.into()).await
    }

    async fn session_model_set(&self, response: SetSessionModelResponse) {
        let data = response::session_model_response(response);
        self.schedule_autocommand("SessionModelUpdated", data.into()).await
    }
}
