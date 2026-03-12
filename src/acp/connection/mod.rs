pub mod manager;
pub mod stdio;
pub use manager::*;

use crate::acp::{Result, error::Error};
use agent_client_protocol::{
    AuthenticateRequest, CancelNotification, ExtNotification, ExtRequest, ForkSessionRequest,
    InitializeRequest, ListSessionsRequest, LoadSessionRequest, NewSessionRequest, PromptRequest,
    ResumeSessionRequest, SetSessionConfigOptionRequest, SetSessionModeRequest,
    SetSessionModelRequest,
};
use tokio::sync::mpsc::Sender;

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

#[derive(Debug)]
pub struct Connection {
    sender: Sender<UserRequest>,
}

impl Connection {
    fn send(&self, request: UserRequest) -> Result<()> {
        self.sender
            .blocking_send(request)
            .map_err(|e| Error::Internal(e.to_string()))
    }
    pub fn new(sender: Sender<UserRequest>) -> Self {
        Self { sender }
    }
    pub fn initialize(&self, request: InitializeRequest) -> Result<()> {
        self.send(UserRequest::Initialize(request))?;
        Ok(())
    }
    pub fn create_session(&self, session: NewSessionRequest) -> Result<()> {
        self.send(UserRequest::CreateSession(session))?;
        Ok(())
    }
    pub fn cancel(&self, notification: CancelNotification) -> Result<()> {
        self.send(UserRequest::Cancel(notification))?;
        Ok(())
    }
    pub fn prompt(&self, request: PromptRequest) -> Result<()> {
        self.send(UserRequest::Prompt(request))?;
        Ok(())
    }
    pub fn authenticate(&self, request: AuthenticateRequest) -> Result<()> {
        self.send(UserRequest::Authenticate(request))?;
        Ok(())
    }
    pub fn set_config_option(&self, request: SetSessionConfigOptionRequest) -> Result<()> {
        self.send(UserRequest::SetConfigOption(request))?;
        Ok(())
    }
    pub fn set_mode(&self, request: SetSessionModeRequest) -> Result<()> {
        self.send(UserRequest::SetMode(request))?;
        Ok(())
    }
    pub fn load_session(&self, request: LoadSessionRequest) -> Result<()> {
        self.send(UserRequest::LoadSession(request))?;
        Ok(())
    }
    pub fn custom_command(&self, request: ExtRequest) -> Result<()> {
        self.send(UserRequest::CustomCommand(request))?;
        Ok(())
    }
    pub fn custom_notification(&self, notification: ExtNotification) -> Result<()> {
        self.send(UserRequest::CustomNotification(notification))?;
        Ok(())
    }
    pub fn list_sessions(&self, request: ListSessionsRequest) -> Result<()> {
        self.send(UserRequest::ListSessions(request))?;
        Ok(())
    }
    pub fn fork_session(&self, request: ForkSessionRequest) -> Result<()> {
        self.send(UserRequest::ForkSession(request))?;
        Ok(())
    }
    pub fn resume_session(&self, request: ResumeSessionRequest) -> Result<()> {
        self.send(UserRequest::ResumeSession(request))?;
        Ok(())
    }
    pub fn set_session_model(&self, request: SetSessionModelRequest) -> Result<()> {
        self.send(UserRequest::SetSessionModel(request))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use agent_client_protocol::{InitializeRequest, ProtocolVersion};
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_connection_initialize() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let connection = Arc::new(Connection::new(sender));
        let request = InitializeRequest::new(ProtocolVersion::LATEST);

        // Spawn blocking task because Connection uses blocking_send
        let conn_clone = connection.clone();
        let req_clone = request.clone();
        tokio::task::spawn_blocking(move || {
            conn_clone.initialize(req_clone).unwrap();
        })
        .await
        .unwrap();

        if let Some(UserRequest::Initialize(received)) = receiver.recv().await {
            assert_eq!(received.protocol_version, request.protocol_version);
        } else {
            panic!("Expected Initialize request");
        }
    }

    #[tokio::test]
    async fn test_connection_create_session() {
        use agent_client_protocol::NewSessionRequest;
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let connection = Arc::new(Connection::new(sender));

        let conn_clone = connection.clone();
        let request = NewSessionRequest::new(std::path::PathBuf::from("/"));

        tokio::task::spawn_blocking(move || {
            conn_clone.create_session(request).unwrap();
        })
        .await
        .unwrap();

        assert!(matches!(
            receiver.recv().await,
            Some(UserRequest::CreateSession(_))
        ));
    }
}
