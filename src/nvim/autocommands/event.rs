use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse, Result,
    SessionNotification, SessionUpdate, TerminalOutputRequest, TerminalOutputResponse,
    WaitForTerminalExitRequest, WaitForTerminalExitResponse, WriteTextFileRequest,
    WriteTextFileResponse,
};
use tokio::sync::oneshot;
use tracing  ::error;

use crate::nvim::{
    autocommands::{AutoCommand, Commands},
    parse,
    requests::{RequestHandler, Responder},
};

#[async_trait::async_trait(?Send)]
impl<R: RequestHandler> Client for AutoCommand<R> {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        if !self.listener_attached(Commands::PermissionRequest).await? {
            // TODO: add default implementation
            return Err(Error::method_not_found());
        }

        let (sender, receiver) =
            oneshot::channel::<agent_client_protocol::RequestPermissionOutcome>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::PermissionRequest,
            args,
            Responder::PermissionResponse(sender),
        )
        .await?;

        let outcome: RequestPermissionOutcome = receiver.await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })?;

        Ok(RequestPermissionResponse::new(outcome))
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
            SessionUpdate::CurrentModeUpdate(_) => Ok(Commands::CurrentMode),
            SessionUpdate::ConfigOptionUpdate(_) => Ok(Commands::ConfigurationOption),
            _ => return Err(Error::method_not_found()),
        }?;

        Ok(self
            .execute_autocommand(command, session_notification)
            .await?)
    }

    async fn write_text_file(&self, _args: WriteTextFileRequest) -> Result<WriteTextFileResponse> {
        Err(Error::method_not_found())
    }

    async fn read_text_file(&self, _args: ReadTextFileRequest) -> Result<ReadTextFileResponse> {
        Err(Error::method_not_found())
    }

    async fn create_terminal(
        &self,
        _args: CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse> {
        Err(Error::method_not_found())
    }

    /// Gets the terminal output and exit status
    async fn terminal_output(
        &self,
        _args: TerminalOutputRequest,
    ) -> Result<TerminalOutputResponse> {
        Err(Error::method_not_found())
    }

    /// Waits for a terminal command to exit
    async fn wait_for_terminal_exit(
        &self,
        _args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        Err(Error::method_not_found())
    }

    /// Releases a terminal resource
    async fn release_terminal(
        &self,
        _args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        Err(Error::method_not_found())
    }
}
