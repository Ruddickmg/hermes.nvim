pub mod manager;
pub mod stdio;
pub mod tcp;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

use crate::acp::{Result, error::Error};
use agent_client_protocol::{
    AuthenticateRequest, CancelNotification, ForkSessionRequest, InitializeRequest,
    ListSessionsRequest, LoadSessionRequest, NewSessionRequest, PromptRequest,
    ResumeSessionRequest, SetSessionConfigOptionRequest, SetSessionModeRequest,
    SetSessionModelRequest,
};
use async_channel::Sender;
pub use manager::*;

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
    async fn send(&self, request: UserRequest) -> Result<()> {
        if let Some(sender) = &self.sender {
            sender
                .send(request)
                .await
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
    pub async fn disconnect(&mut self) -> Result<()> {
        // Phase 1: Drop the channel sender to signal the message loop to exit
        if let Some(sender) = self.sender.take() {
            drop(sender);
        }

        if let Some(ref handle) = self.handle {
            // Fast path: if the thread already finished, no async work needed
            if handle.is_finished() {
                debug!("Connection thread already exited");
                return Ok(());
            }

            // Phase 2: Wait for graceful exit
            if Self::wait_for_thread(handle, GRACEFUL_SHUTDOWN_TIMEOUT).await {
                debug!("Connection thread exited gracefully");
                return Ok(());
            }

            // Phase 3: Terminate the child process (SIGTERM on Unix, TerminateProcess on Windows)
            if let Some(ref child) = self.child {
                if let Err(e) = child.terminate().await {
                    warn!("Failed to send terminate signal to child: {}", e);
                }
            }
            if Self::wait_for_thread(handle, FORCE_KILL_TIMEOUT).await {
                debug!("Connection thread exited after terminate");
                return Ok(());
            }

            // Phase 4: Force-kill the child process (SIGKILL on Unix, TerminateProcess on Windows)
            if let Some(ref child) = self.child {
                if let Err(e) = child.kill().await {
                    warn!("Failed to force-kill child: {}", e);
                }
            }
            if Self::wait_for_thread(handle, FORCE_KILL_TIMEOUT).await {
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
        }
        Ok(())
    }

    /// Returns true if the thread finished within the timeout.
    async fn wait_for_thread(handle: &JoinHandle<Result<()>>, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if handle.is_finished() {
                return true;
            }
            async_io::Timer::after(Duration::from_millis(10)).await;
        }
        false
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn close(&self) -> Result<()> {
        self.send(UserRequest::Close).await?;
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
    pub async fn initialize(&self, request: InitializeRequest) -> Result<()> {
        self.send(UserRequest::Initialize(request)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn create_session(&self, session: NewSessionRequest) -> Result<()> {
        self.send(UserRequest::CreateSession(session)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn cancel(&self, notification: CancelNotification) -> Result<()> {
        self.send(UserRequest::Cancel(notification)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn prompt(&self, request: PromptRequest) -> Result<()> {
        self.send(UserRequest::Prompt(request)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn authenticate(&self, request: AuthenticateRequest) -> Result<()> {
        self.send(UserRequest::Authenticate(request)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_config_option(&self, request: SetSessionConfigOptionRequest) -> Result<()> {
        self.send(UserRequest::SetConfigOption(request)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_mode(&self, request: SetSessionModeRequest) -> Result<()> {
        self.send(UserRequest::SetMode(request)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn load_session(&self, request: LoadSessionRequest) -> Result<()> {
        self.send(UserRequest::LoadSession(request)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn list_sessions(&self, request: ListSessionsRequest) -> Result<()> {
        self.send(UserRequest::ListSessions(request)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn fork_session(&self, request: ForkSessionRequest) -> Result<()> {
        self.send(UserRequest::ForkSession(request)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn resume_session(&self, request: ResumeSessionRequest) -> Result<()> {
        self.send(UserRequest::ResumeSession(request)).await?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_session_model(&self, request: SetSessionModelRequest) -> Result<()> {
        self.send(UserRequest::SetSessionModel(request)).await?;
        Ok(())
    }
}

impl Drop for Connection {
    /// Best-effort synchronous cleanup.
    ///
    /// The full async `disconnect()` should be called explicitly before dropping.
    /// This `Drop` impl is a safety net that performs only synchronous operations
    /// to avoid panicking from a nested `block_on` on the `current_thread` runtime.
    fn drop(&mut self) {
        // Phase 1: Drop the sender to signal the message loop to exit.
        // When the receiver sees the channel closed, it breaks the loop and the
        // connection thread will begin shutting down on its own.
        if let Some(sender) = self.sender.take() {
            drop(sender);
        }

        // Phase 2: If the connection thread already exited, nothing more to do.
        if let Some(ref handle) = self.handle {
            if handle.is_finished() {
                debug!("Connection thread already exited during drop");
                return;
            }
        }

        // Phase 3: If there's a child process, send a kill signal synchronously.
        // This is non-blocking (just sends the signal). The child's own `Drop` impl
        // also calls `start_kill()` and the process was spawned with `kill_on_drop(true)`,
        // providing additional insurance.
        if let Some(ref child) = self.child {
            if let Err(e) = child.try_kill_sync() {
                warn!("Failed to kill child process during connection drop: {}", e);
            }
        }

        // We intentionally do NOT block waiting for the thread to exit.
        // The thread will exit on its own once the channel is closed and/or
        // the child process is killed. Blocking here would risk deadlocking
        // or panicking inside a nested `block_on`.
        if self.handle.is_some() {
            warn!(
                "Connection dropped without explicit disconnect. \
                 The connection thread may still be running briefly."
            );
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

    fn mock_runtime() -> smol::LocalExecutor<'static> {
        smol::LocalExecutor::new()
    }

    #[test]
    fn test_wait_for_thread_returns_true_when_finished() {
        let executor = mock_runtime();
        let handle = std::thread::spawn(|| Ok::<(), Error>(()));
        // Give thread time to finish
        std::thread::sleep(Duration::from_millis(10));
        let result = smol::block_on(executor.run(async {
            Connection::wait_for_thread(&handle, Duration::from_millis(500)).await
        }));
        assert!(result);
    }

    #[test]
    fn test_wait_for_thread_returns_false_on_timeout() {
        let executor = mock_runtime();
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let handle = std::thread::spawn(move || {
            let _ = rx.recv(); // Block until signaled
            Ok::<(), Error>(())
        });
        let result = smol::block_on(executor.run(async {
            Connection::wait_for_thread(&handle, Duration::from_millis(50)).await
        }));
        assert!(!result);
        // Cleanup: unblock the thread
        let _ = tx.send(());
        let _ = handle.join();
    }

    #[test]
    fn test_connection_initialize() {
        let executor = mock_runtime();
        let (sender, receiver) = async_channel::bounded(1);
        let connection = Arc::new(Connection::new(sender, mock_handle(), None));
        let request = InitializeRequest::new(ProtocolVersion::LATEST);

        smol::block_on(executor.run(async {
            connection.initialize(request.clone()).await.unwrap();
        }));

        drop(connection);

        smol::block_on(executor.run(async {
            if let Ok(UserRequest::Initialize(received)) = receiver.recv().await {
                assert_eq!(received.protocol_version, request.protocol_version);
            } else {
                panic!("Expected Initialize request");
            }
        }));
    }

    #[test]
    fn test_connection_create_session() {
        use agent_client_protocol::NewSessionRequest;
        let executor = mock_runtime();
        let (sender, receiver) = async_channel::bounded(1);
        let connection = Arc::new(Connection::new(sender, mock_handle(), None));

        let request = NewSessionRequest::new(std::path::PathBuf::from("/"));

        smol::block_on(executor.run(async {
            connection.create_session(request).await.unwrap();
        }));

        drop(connection);

        smol::block_on(executor.run(async {
            assert!(matches!(
                receiver.recv().await,
                Ok(UserRequest::CreateSession(_))
            ));
        }));
    }
}
