pub mod manager;
pub mod stdio;
pub mod tcp;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

pub use manager::*;

use crate::acp::{Result, error::Error};
use agent_client_protocol::{
    AuthenticateRequest, CancelNotification, ForkSessionRequest, InitializeRequest,
    ListSessionsRequest, LoadSessionRequest, NewSessionRequest, PromptRequest,
    ResumeSessionRequest, SetSessionConfigOptionRequest, SetSessionModeRequest,
    SetSessionModelRequest,
};
use tokio::sync::mpsc::Sender;

/// Maximum time to wait for a connection thread to exit gracefully before force-killing
/// the child process.
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

/// Maximum time to wait for a connection thread to exit after force-killing the child process.
const FORCE_KILL_TIMEOUT: Duration = Duration::from_secs(2);

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
    /// Shared child process handle for stdio connections, enabling concurrent
    /// wait/kill. `None` for non-stdio connections (TCP, HTTP, etc.).
    child: Option<Arc<stdio::child::Child>>,
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

    /// Disconnect from the agent, using a multi-phase shutdown:
    /// 1. Drop the channel sender (signals the message loop to exit)
    /// 2. Wait for the thread to exit gracefully within a timeout
    /// 3. If still running, terminate the child process (SIGTERM) and wait again
    /// 4. If still running, force-kill the child process (SIGKILL) and wait again
    /// 5. If still running, abandon the thread (don't block Neovim)
    #[tracing::instrument(level = "trace", skip(self))]
    pub fn disconnect(&mut self) -> Result<()> {
        // Phase 1: Drop the channel sender to signal the message loop to exit
        if let Some(sender) = self.sender.take() {
            drop(sender);
        }

        // Phase 2: Wait for graceful exit
        if self.wait_for_thread(GRACEFUL_SHUTDOWN_TIMEOUT) {
            debug!("Connection thread exited gracefully");
            return Ok(());
        }

        // Phase 3: Terminate the child process (SIGTERM on Unix, TerminateProcess on Windows)
        if let Some(ref child) = self.child
            && let Err(e) = child.terminate_blocking()
        {
            warn!("Failed to send terminate signal to child: {}", e);
        }
        if self.wait_for_thread(FORCE_KILL_TIMEOUT) {
            debug!("Connection thread exited after terminate");
            return Ok(());
        }

        // Phase 4: Force-kill the child process (SIGKILL on Unix, TerminateProcess on Windows)
        if let Some(ref child) = self.child
            && let Err(e) = child.kill_blocking()
        {
            warn!("Failed to force-kill child: {}", e);
        }
        if self.wait_for_thread(FORCE_KILL_TIMEOUT) {
            debug!("Connection thread exited after force-kill");
            return Ok(());
        }

        // Phase 5: Abandon the thread - don't block Neovim
        error!(
            "Connection thread did not exit within timeout, abandoning. \
             The child process may still be running."
        );
        // Intentionally leak the JoinHandle to avoid blocking.
        // The thread will eventually exit when the child process dies or the OS cleans up.
        self.handle.take();
        Ok(())
    }

    /// Wait for the connection thread to finish within the given timeout.
    /// Returns true if the thread has finished, false if it's still running.
    fn wait_for_thread(&mut self, timeout: Duration) -> bool {
        let handle = match self.handle.as_ref() {
            Some(h) => h,
            None => return true, // No thread to wait for
        };

        let start = Instant::now();
        while start.elapsed() < timeout {
            if handle.is_finished() {
                // Thread is done, join it to collect the result
                if let Some(handle) = self.handle.take() {
                    match handle.join() {
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => warn!("Connection thread exited with error: {:?}", e),
                        Err(e) => warn!("Connection thread panicked: {:?}", e),
                    }
                }
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn close(&self) -> Result<()> {
        self.send(UserRequest::Close)?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(child))]
    pub fn new(
        sender: Sender<UserRequest>,
        handle: JoinHandle<Result<()>>,
        child: Option<Arc<stdio::child::Child>>,
    ) -> Self {
        Self {
            sender: Some(sender),
            handle: Some(handle),
            child,
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
        // Drop must never block Neovim. If disconnect() hangs, Neovim will freeze.
        // The disconnect() method already has timeout-based escalation and will
        // abandon the thread rather than block indefinitely.
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

    /// Creates a mock thread handle that immediately returns Ok for testing
    fn mock_handle() -> JoinHandle<Result<()>> {
        std::thread::spawn(|| Ok::<(), Error>(()))
    }

    #[tokio::test]
    async fn test_connection_initialize() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let connection = Arc::new(Connection::new(sender, mock_handle(), None));
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
        let connection = Arc::new(Connection::new(sender, mock_handle(), None));

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
