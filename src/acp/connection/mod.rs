pub mod manager;
pub mod stdio;
pub mod tcp;
use std::thread::JoinHandle;
use tracing::warn;

pub use manager::*;

use crate::acp::{Result, error::Error};
use agent_client_protocol::{
    AuthenticateRequest, CancelNotification, ForkSessionRequest, InitializeRequest,
    ListSessionsRequest, LoadSessionRequest, NewSessionRequest, PromptRequest,
    ResumeSessionRequest, SetSessionConfigOptionRequest, SetSessionModeRequest,
    SetSessionModelRequest,
};
use tokio::sync::mpsc::Sender;

#[derive(PartialEq, Debug, Clone)]
pub enum UserRequest {
    Close,
    Initialize(InitializeRequest),
    Cancel(CancelNotification),
    CreateSession(NewSessionRequest),
    Prompt(PromptRequest),
    Authenticate(AuthenticateRequest),
    SetConfigOption(SetSessionConfigOptionRequest),
    SetMode(SetSessionModeRequest),
    LoadSession(LoadSessionRequest),
    ListSessions(ListSessionsRequest),
    ForkSession(ForkSessionRequest),
    ResumeSession(ResumeSessionRequest),
    SetSessionModel(SetSessionModelRequest),
}

#[derive(Debug)]
pub struct Connection {
    sender: Option<Sender<UserRequest>>,
    handle: Option<JoinHandle<Result<()>>>,
}

impl Connection {
    #[tracing::instrument(level = "trace", skip(self))]
    fn send(&self, request: UserRequest) -> Result<()> {
        if let Some(sender) = &self.sender {
            sender
                .blocking_send(request)
                .map_err(|e| Error::Internal(e.to_string()))
        } else {
            Err(Error::Internal(
                "Connection sender is not available".to_string(),
            ))
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(sender) = self.sender.take() {
            drop(sender);
        }
        self.join()?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn join(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            match handle.join() {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(e)) => Err(Error::Connection(format!(
                    "Error in connection thread for agent {:?}",
                    e
                ))),
                Err(e) => Err(Error::Internal(format!(
                    "Failed to join thread for agent {:?}",
                    e
                ))),
            }
        } else {
            Err(Error::Internal(
                "Connection handle is not available".to_string(),
            ))
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn close(&self) -> Result<()> {
        self.send(UserRequest::Close)?;
        Ok(())
    }

    #[tracing::instrument(level = "trace")]
    pub fn new(sender: Sender<UserRequest>, handle: JoinHandle<Result<()>>) -> Self {
        Self {
            sender: Some(sender),
            handle: Some(handle),
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn initialize(&self, request: InitializeRequest) -> Result<()> {
        self.send(UserRequest::Initialize(request))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn create_session(&self, session: NewSessionRequest) -> Result<()> {
        self.send(UserRequest::CreateSession(session))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn cancel(&self, notification: CancelNotification) -> Result<()> {
        self.send(UserRequest::Cancel(notification))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn prompt(&self, request: PromptRequest) -> Result<()> {
        self.send(UserRequest::Prompt(request))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn authenticate(&self, request: AuthenticateRequest) -> Result<()> {
        self.send(UserRequest::Authenticate(request))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn set_config_option(&self, request: SetSessionConfigOptionRequest) -> Result<()> {
        self.send(UserRequest::SetConfigOption(request))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn set_mode(&self, request: SetSessionModeRequest) -> Result<()> {
        self.send(UserRequest::SetMode(request))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn load_session(&self, request: LoadSessionRequest) -> Result<()> {
        self.send(UserRequest::LoadSession(request))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn list_sessions(&self, request: ListSessionsRequest) -> Result<()> {
        self.send(UserRequest::ListSessions(request))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn fork_session(&self, request: ForkSessionRequest) -> Result<()> {
        self.send(UserRequest::ForkSession(request))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn resume_session(&self, request: ResumeSessionRequest) -> Result<()> {
        self.send(UserRequest::ResumeSession(request))?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn set_session_model(&self, request: SetSessionModelRequest) -> Result<()> {
        self.send(UserRequest::SetSessionModel(request))?;
        Ok(())
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Err(e) = self.disconnect() {
            warn!("Failed to close connection on drop: {}", e);
        }
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
