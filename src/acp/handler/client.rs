use crate::{
    Handler,
    nvim::{autocommands::Commands, parse, requests::Responder},
};
use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionRequest, RequestPermissionResponse, Result, SessionNotification,
    SessionUpdate, TerminalExitStatus, TerminalOutputRequest, TerminalOutputResponse,
    WaitForTerminalExitRequest, WaitForTerminalExitResponse, WriteTextFileRequest,
    WriteTextFileResponse,
};
use tokio::sync::oneshot;
use tracing::error;

#[async_trait::async_trait(?Send)]
impl Client for Handler {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        if !self.can_request_permissions().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) =
            oneshot::channel::<agent_client_protocol::RequestPermissionOutcome>();

        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::PermissionRequest,
            args.clone(),
            Responder::PermissionResponse(sender),
        )
        .await?;
        receiver
            .await
            .map_err(|e| {
                error!("{:?}", e);
                Error::internal_error()
            })
            .map(RequestPermissionResponse::new)
    }

    async fn session_notification(&self, session_notification: SessionNotification) -> Result<()> {
        if !self.can_receive_notifications().await {
            return Err(Error::method_not_found());
        }
        let command = match session_notification.update.clone() {
            SessionUpdate::UserMessageChunk(chunk) => parse::communication(chunk.content)
                .map(|s| Commands::from(format!("User{}Message", s))),
            SessionUpdate::AgentMessageChunk(chunk) => parse::communication(chunk.content)
                .map(|s| Commands::from(format!("Agent{}Message", s))),
            SessionUpdate::AgentThoughtChunk(chunk) => parse::communication(chunk.content)
                .map(|s| Commands::from(format!("Agent{}Thought", s))),
            SessionUpdate::ToolCall(_) => Ok(Commands::ToolCall),
            SessionUpdate::ToolCallUpdate(_) => Ok(Commands::ToolCallUpdate),
            SessionUpdate::Plan(_) => Ok(Commands::Plan),
            SessionUpdate::AvailableCommandsUpdate(_) => Ok(Commands::AvailableCommands),
            SessionUpdate::CurrentModeUpdate(_) => Ok(Commands::ModeCurrent),
            SessionUpdate::ConfigOptionUpdate(_) => Ok(Commands::ConfigurationOption),
            _ => return Err(Error::method_not_found()),
        }?;

        Ok(self
            .execute_autocommand(command, session_notification)
            .await?)
    }

    async fn write_text_file(&self, args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
        if !self.can_write().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = oneshot::channel::<WriteTextFileResponse>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::WriteTextFile,
            args.clone(),
            Responder::WriteFileResponse(sender, args),
        )
        .await?;
        receiver.await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })
    }

    async fn read_text_file(&self, args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        if !self.can_read().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = oneshot::channel::<Result<ReadTextFileResponse>>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::ReadTextFile,
            args.clone(),
            Responder::ReadFileResponse(sender, args),
        )
        .await?;
        receiver.await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })?
    }

    async fn create_terminal(&self, args: CreateTerminalRequest) -> Result<CreateTerminalResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = oneshot::channel::<Result<CreateTerminalResponse>>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalCreate,
            args.clone(),
            Responder::TerminalCreate(sender, args),
        )
        .await?;
        receiver.await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })?
    }

    /// Gets the terminal output and exit status
    async fn terminal_output(&self, args: TerminalOutputRequest) -> Result<TerminalOutputResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = oneshot::channel::<Result<TerminalOutputResponse>>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalOutput,
            args.clone(),
            Responder::TerminalOutput(sender, args),
        )
        .await?;
        receiver.await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })?
    }

    /// Waits for a terminal command to exit
    async fn wait_for_terminal_exit(
        &self,
        args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = oneshot::channel::<(u32, String)>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalOutput,
            args.clone(),
            Responder::TerminalExit(sender, args),
        )
        .await?;
        receiver
            .await
            .map_err(|_| Error::internal_error())
            .map(|(exit_code, event)| {
                WaitForTerminalExitResponse::new(
                    TerminalExitStatus::new().signal(event).exit_code(exit_code),
                )
            })
    }

    /// Releases a terminal resource
    async fn release_terminal(
        &self,
        _args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        return Err(Error::method_not_found());
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::nvim::state::PluginState;
    use agent_client_protocol::{
        Client, CreateTerminalRequest, CreateTerminalResponse, ReadTextFileRequest,
        ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
        RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
        SessionNotification, TerminalExitStatus, TerminalOutputRequest, TerminalOutputResponse,
        WaitForTerminalExitRequest, WaitForTerminalExitResponse, WriteTextFileRequest,
        WriteTextFileResponse,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    pub struct MockClient {
        write_called: Arc<Mutex<bool>>,
        read_called: Arc<Mutex<bool>>,
        terminal_create_called: Arc<Mutex<bool>>,
        pub notification_called: Arc<Mutex<bool>>,
    }

    impl MockClient {
        pub fn new() -> Self {
            Self {
                write_called: Arc::new(Mutex::new(false)),
                read_called: Arc::new(Mutex::new(false)),
                terminal_create_called: Arc::new(Mutex::new(false)),
                notification_called: Arc::new(Mutex::new(false)),
            }
        }
    }

    #[async_trait::async_trait(?Send)]
    impl Client for MockClient {
        async fn write_text_file(
            &self,
            _args: WriteTextFileRequest,
        ) -> Result<WriteTextFileResponse> {
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
            Ok(TerminalOutputResponse::new("output", false))
        }
        async fn wait_for_terminal_exit(
            &self,
            _args: WaitForTerminalExitRequest,
        ) -> Result<WaitForTerminalExitResponse> {
            Ok(WaitForTerminalExitResponse::new(TerminalExitStatus::new()))
        }
        async fn release_terminal(
            &self,
            _args: ReleaseTerminalRequest,
        ) -> Result<ReleaseTerminalResponse> {
            Ok(ReleaseTerminalResponse::new())
        }
    }

    #[tokio::test]
    async fn test_write_text_file_permissions_allowed() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));

        let handler = Handler::new(state.clone(), mock.clone());

        let req = WriteTextFileRequest::new("session_id", "test.txt", "test");
        let res = handler.write_text_file(req.clone()).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_write_text_file_calls_handler() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));

        let handler = Handler::new(state.clone(), mock.clone());

        let req = WriteTextFileRequest::new("session_id", "test.txt", "test");
        let _ = handler.write_text_file(req.clone()).await;
        assert!(*mock.write_called.lock().await);
    }

    #[tokio::test]
    async fn test_write_text_file_permissions_denied() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.fs_write_access = false;

        let handler = Handler::new(state.clone(), mock.clone());

        let req = WriteTextFileRequest::new("session_id", "test.txt", "test");
        let res = handler.write_text_file(req).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_write_text_file_permissions_denied_does_not_call_handler() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.fs_write_access = false;

        let handler = Handler::new(state.clone(), mock.clone());

        let req = WriteTextFileRequest::new("session_id", "test.txt", "test");
        let _ = handler.write_text_file(req).await;
        assert!(!*mock.write_called.lock().await);
    }

    #[tokio::test]
    async fn test_read_text_file_permissions_allowed() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));

        let handler = Handler::new(state.clone(), mock.clone());

        let req = ReadTextFileRequest::new("session_id", "test.txt");
        let res = handler.read_text_file(req.clone()).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_read_text_file_calls_handler() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));

        let handler = Handler::new(state.clone(), mock.clone());

        let req = ReadTextFileRequest::new("session_id", "test.txt");
        let _ = handler.read_text_file(req.clone()).await;
        assert!(*mock.read_called.lock().await);
    }

    #[tokio::test]
    async fn test_read_text_file_permissions_denied() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.fs_read_access = false;

        let handler = Handler::new(state.clone(), mock.clone());

        let req = ReadTextFileRequest::new("session_id", "test.txt");
        let res = handler.read_text_file(req.clone()).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_read_text_file_permissions_denied_does_not_call_handler() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.fs_read_access = false;

        let handler = Handler::new(state.clone(), mock.clone());

        let req = ReadTextFileRequest::new("session_id", "test.txt");
        let _ = handler.read_text_file(req.clone()).await;
        assert!(!*mock.read_called.lock().await);
    }
}
