use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionRequest, RequestPermissionResponse, Result, SessionNotification,
    TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};

use crate::Handler;

#[async_trait::async_trait(?Send)]
impl<H: Client> Client for Handler<H> {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        self.handler.request_permission(args).await
    }

    async fn session_notification(&self, args: SessionNotification) -> Result<()> {
        self.handler.session_notification(args).await
    }

    async fn write_text_file(&self, args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
        if self.config.fs_write_access {
            self.handler.write_text_file(args).await?;
            Ok(WriteTextFileResponse::new())
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn read_text_file(&self, _args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        if self.config.fs_read_access {
            self.handler.read_text_file(_args).await
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn create_terminal(&self, args: CreateTerminalRequest) -> Result<CreateTerminalResponse> {
        if self.config.terminal_access {
            self.create_terminal(args).await
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn terminal_output(&self, args: TerminalOutputRequest) -> Result<TerminalOutputResponse> {
        if self.config.terminal_access {
            self.handler.terminal_output(args).await
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn wait_for_terminal_exit(
        &self,
        args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        if self.config.terminal_access {
            self.handler.wait_for_terminal_exit(args).await
        } else {
            Err(Error::method_not_found())
        }
    }

    async fn release_terminal(
        &self,
        args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        if self.config.terminal_access {
            self.handler.release_terminal(args).await
        } else {
            Err(Error::method_not_found())
        }
    }
}
