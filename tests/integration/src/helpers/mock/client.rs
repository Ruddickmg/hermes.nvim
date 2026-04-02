//! Mock implementations of Client trait for testing
use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse, Result,
    SessionNotification, TerminalExitStatus, TerminalOutputRequest, TerminalOutputResponse,
    WaitForTerminalExitRequest, WaitForTerminalExitResponse, WriteTextFileRequest,
    WriteTextFileResponse,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
#[allow(dead_code)]
pub struct MockClient {
    write_called: Arc<Mutex<bool>>,
    read_called: Arc<Mutex<bool>>,
    terminal_create_called: Arc<Mutex<bool>>,
    terminal_output_called: Arc<Mutex<bool>>,
    wait_for_terminal_exit_called: Arc<Mutex<bool>>,
    release_terminal_called: Arc<Mutex<bool>>,
    kill_terminal_called: Arc<Mutex<bool>>,
    pub notification_called: Arc<Mutex<bool>>,
}

impl Default for MockClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockClient {
    pub fn new() -> Self {
        Self {
            write_called: Arc::new(Mutex::new(false)),
            read_called: Arc::new(Mutex::new(false)),
            terminal_create_called: Arc::new(Mutex::new(false)),
            terminal_output_called: Arc::new(Mutex::new(false)),
            wait_for_terminal_exit_called: Arc::new(Mutex::new(false)),
            release_terminal_called: Arc::new(Mutex::new(false)),
            kill_terminal_called: Arc::new(Mutex::new(false)),
            notification_called: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait(?Send)]
impl Client for MockClient {
    async fn write_text_file(&self, _args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
        *self.write_called.lock().await = true;
        Ok(WriteTextFileResponse::new())
    }

    async fn read_text_file(&self, _args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        *self.read_called.lock().await = true;
        Ok(ReadTextFileResponse::new("content"))
    }

    async fn create_terminal(
        &self,
        _args: CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse> {
        *self.terminal_create_called.lock().await = true;
        Ok(CreateTerminalResponse::new("1"))
    }

    async fn request_permission(
        &self,
        _args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        Ok(RequestPermissionResponse::new(
            RequestPermissionOutcome::Cancelled,
        ))
    }

    async fn session_notification(&self, _args: SessionNotification) -> Result<()> {
        *self.notification_called.lock().await = true;
        Ok(())
    }

    async fn terminal_output(
        &self,
        _args: TerminalOutputRequest,
    ) -> Result<TerminalOutputResponse> {
        *self.terminal_output_called.lock().await = true;
        Ok(TerminalOutputResponse::new("output", false))
    }

    async fn wait_for_terminal_exit(
        &self,
        _args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        *self.wait_for_terminal_exit_called.lock().await = true;
        Ok(WaitForTerminalExitResponse::new(TerminalExitStatus::new()))
    }

    async fn release_terminal(
        &self,
        _args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        *self.release_terminal_called.lock().await = true;
        Ok(ReleaseTerminalResponse::new())
    }
}
