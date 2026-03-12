use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionRequest, RequestPermissionResponse, Result, SessionNotification,
    TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};

use crate::{Handler, nvim::autocommands::ResponseHandler};

#[async_trait::async_trait(?Send)]
impl<H: Client + ResponseHandler> Client for Handler<H> {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        if self.can_request_permissions().await {
            self.handler.request_permission(args).await
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn session_notification(&self, args: SessionNotification) -> Result<()> {
        self.handler.session_notification(args).await
    }

    async fn write_text_file(&self, args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
        if self.can_write().await {
            self.handler.write_text_file(args).await?;
            Ok(WriteTextFileResponse::new())
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn read_text_file(&self, _args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        if self.can_read().await {
            self.handler.read_text_file(_args).await
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn create_terminal(&self, args: CreateTerminalRequest) -> Result<CreateTerminalResponse> {
        if self.can_access_terminal().await {
            self.create_terminal(args).await
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn terminal_output(&self, args: TerminalOutputRequest) -> Result<TerminalOutputResponse> {
        if self.can_access_terminal().await {
            self.handler.terminal_output(args).await
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn wait_for_terminal_exit(
        &self,
        args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        if self.can_access_terminal().await {
            self.handler.wait_for_terminal_exit(args).await
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn release_terminal(
        &self,
        args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        if self.can_access_terminal().await {
            self.handler.release_terminal(args).await
        } else {
            Err(Error::method_not_found())
        }
    }
}

#[cfg(test)]
mod tests {
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
    struct MockClient {
        write_called: Arc<Mutex<bool>>,
        read_called: Arc<Mutex<bool>>,
        terminal_create_called: Arc<Mutex<bool>>,
    }

    impl MockClient {
        fn new() -> Self {
            Self {
                write_called: Arc::new(Mutex::new(false)),
                read_called: Arc::new(Mutex::new(false)),
                terminal_create_called: Arc::new(Mutex::new(false)),
            }
        }
    }

    #[async_trait::async_trait(?Send)]
    impl ResponseHandler for MockClient {
        async fn schedule_autocommand<
            T: std::fmt::Debug + ToString,
            S: std::fmt::Debug + serde::Serialize,
        >(
            &self,
            _command: T,
            _data: S,
        ) -> crate::acp::Result<()> {
            Ok(())
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
