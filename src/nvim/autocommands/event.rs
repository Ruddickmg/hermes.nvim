use crate::nvim::{
    autocommands::{AutoCommand, Commands},
    parse,
    requests::{RequestHandler, Responder},
};
use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error as AcpError, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionRequest, RequestPermissionResponse, Result, SessionNotification,
    SessionUpdate, TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};
use tokio::sync::oneshot;
use tracing::error;

impl From<Responder> for Commands {
    fn from(responder: Responder) -> Self {
        match responder {
            Responder::ReadFileResponse(..) => Commands::ReadTextFile,
            Responder::PermissionResponse(..) => Commands::PermissionRequest,
            Responder::WriteFileResponse(..) => Commands::WriteTextFile,
        }
    }
}

#[async_trait::async_trait(?Send)]
impl<R: RequestHandler + 'static> Client for AutoCommand<R> {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
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
                AcpError::internal_error()
            })
            .map(RequestPermissionResponse::new)
    }

    async fn session_notification(&self, session_notification: SessionNotification) -> Result<()> {
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
            _ => return Err(AcpError::method_not_found()),
        }?;

        Ok(self
            .execute_autocommand(command, session_notification)
            .await?)
    }

    async fn write_text_file(&self, args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
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
            AcpError::internal_error()
        })
    }

    async fn read_text_file(&self, args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        let (sender, receiver) = oneshot::channel::<Result<ReadTextFileResponse>>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::WriteTextFile,
            args.clone(),
            Responder::ReadFileResponse(sender, args),
        )
        .await?;
        receiver.await.map_err(|e| {
            error!("{:?}", e);
            AcpError::internal_error()
        })?
    }

    async fn create_terminal(
        &self,
        _args: CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse> {
        Err(AcpError::method_not_found())
    }

    /// Gets the terminal output and exit status
    async fn terminal_output(
        &self,
        _args: TerminalOutputRequest,
    ) -> Result<TerminalOutputResponse> {
        Err(AcpError::method_not_found())
    }

    /// Waits for a terminal command to exit
    async fn wait_for_terminal_exit(
        &self,
        _args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        Err(AcpError::method_not_found())
    }

    /// Releases a terminal resource
    async fn release_terminal(
        &self,
        _args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        Err(AcpError::method_not_found())
    }
}
