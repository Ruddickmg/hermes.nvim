pub mod manager;
pub mod stdio;
pub use manager::*;

use crate::apc::error::Error;
use agent_client_protocol::{
    AuthenticateRequest, CancelNotification, ExtNotification, ExtRequest, ForkSessionRequest,
    InitializeRequest, ListSessionsRequest, LoadSessionRequest, NewSessionRequest, PromptRequest,
    ResumeSessionRequest, SetSessionConfigOptionRequest, SetSessionModeRequest,
    SetSessionModelRequest,
};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum UserRequest {
    Initialize(InitializeRequest),
    Cancel(CancelNotification),
    CreateSession(NewSessionRequest),
    Prompt(PromptRequest),
    Authenticate(AuthenticateRequest),
    SetConfigOption(SetSessionConfigOptionRequest),
    SetMode(SetSessionModeRequest),
    LoadSession(LoadSessionRequest),
    CustomCommand(ExtRequest),
    CustomNotification(ExtNotification),
    ListSessions(ListSessionsRequest),
    ForkSession(ForkSessionRequest),
    ResumeSession(ResumeSessionRequest),
    SetSessionModel(SetSessionModelRequest),
}

#[derive(Debug, Clone)]
pub struct Connection {
    sender: Sender<UserRequest>,
}

impl Connection {
    pub fn new(sender: Sender<UserRequest>) -> Self {
        Self { sender }
    }
    pub fn initialize(&self, request: InitializeRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::Initialize(request))?;
        Ok(())
    }
    pub fn create_session(&self, session: NewSessionRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::CreateSession(session))?;
        Ok(())
    }
    pub fn cancel(&self, notification: CancelNotification) -> Result<(), Error> {
        self.sender.send(UserRequest::Cancel(notification))?;
        Ok(())
    }
    pub fn prompt(&self, request: PromptRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::Prompt(request))?;
        Ok(())
    }
    pub fn authenticate(&self, request: AuthenticateRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::Authenticate(request))?;
        Ok(())
    }
    pub fn set_config_option(&self, request: SetSessionConfigOptionRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::SetConfigOption(request))?;
        Ok(())
    }
    pub fn set_mode(&self, request: SetSessionModeRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::SetMode(request))?;
        Ok(())
    }
    pub fn load_session(&self, request: LoadSessionRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::LoadSession(request))?;
        Ok(())
    }
    pub fn custom_command(&self, request: ExtRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::CustomCommand(request))?;
        Ok(())
    }
    pub fn custom_notification(&self, notification: ExtNotification) -> Result<(), Error> {
        self.sender
            .send(UserRequest::CustomNotification(notification))?;
        Ok(())
    }
    pub fn list_sessions(&self, request: ListSessionsRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::ListSessions(request))?;
        Ok(())
    }
    pub fn fork_session(&self, request: ForkSessionRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::ForkSession(request))?;
        Ok(())
    }
    pub fn resume_session(&self, request: ResumeSessionRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::ResumeSession(request))?;
        Ok(())
    }
    pub fn set_session_model(&self, request: SetSessionModelRequest) -> Result<(), Error> {
        self.sender.send(UserRequest::SetSessionModel(request))?;
        Ok(())
    }
}
