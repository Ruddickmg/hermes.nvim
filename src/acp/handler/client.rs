use crate::{
    Handler, acp,
    nvim::{autocommands::Commands, parse, requests::Responder},
};
use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error, KillTerminalCommandRequest,
    KillTerminalCommandResponse, ReadTextFileRequest, ReadTextFileResponse, ReleaseTerminalRequest,
    ReleaseTerminalResponse, RequestPermissionRequest, RequestPermissionResponse, Result,
    SessionNotification, SessionUpdate, TerminalExitStatus, TerminalOutputRequest,
    TerminalOutputResponse, WaitForTerminalExitRequest, WaitForTerminalExitResponse,
    WriteTextFileRequest, WriteTextFileResponse,
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
        let (sender, receiver) = oneshot::channel::<acp::Result<CreateTerminalResponse>>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalCreate,
            args.clone(),
            Responder::TerminalCreate(sender, args),
        )
        .await?;
        Ok(receiver.await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })??)
    }

    /// Gets the terminal output and exit status
    async fn terminal_output(&self, args: TerminalOutputRequest) -> Result<TerminalOutputResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = oneshot::channel::<acp::Result<TerminalOutputResponse>>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalOutput,
            args.clone(),
            Responder::TerminalOutput(sender, args),
        )
        .await?;
        Ok(receiver.await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })??)
    }

    /// Waits for a terminal command to exit
    async fn wait_for_terminal_exit(
        &self,
        args: WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = oneshot::channel::<acp::Result<(Option<u32>, Option<String>)>>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalOutput,
            args.clone(),
            Responder::TerminalExit(sender, args),
        )
        .await?;
        Ok(receiver
            .await
            .map_err(|_| Error::internal_error())?
            .and_then(|(exit_code, signal)| {
                // Validate that at least one field is present
                if exit_code.is_none() && signal.is_none() {
                    Err(acp::error::Error::InvalidInput(
                        "Both exit code and signal are undefined".to_string(),
                    ))
                } else {
                    let mut status = TerminalExitStatus::new();
                    if let Some(code) = exit_code {
                        status = status.exit_code(code);
                    }
                    if let Some(sig) = signal {
                        status = status.signal(sig);
                    }

                    Ok(WaitForTerminalExitResponse::new(status))
                }
            })?)
    }

    async fn kill_terminal_command(
        &self,
        args: KillTerminalCommandRequest,
    ) -> Result<KillTerminalCommandResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = oneshot::channel::<acp::Result<KillTerminalCommandResponse>>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalKill,
            args.clone(),
            Responder::TerminalKill(sender, args),
        )
        .await?;
        Ok(receiver.await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })??)
    }

    /// Releases a terminal resource
    async fn release_terminal(
        &self,
        args: ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse> {
        if !self.can_access_terminal().await {
            return Err(Error::method_not_found());
        }
        let (sender, receiver) = oneshot::channel::<acp::Result<ReleaseTerminalResponse>>();
        self.execute_autocommand_request(
            args.session_id.to_string(),
            Commands::TerminalRelease,
            args.clone(),
            Responder::TerminalRelease(sender, args),
        )
        .await?;
        Ok(receiver.await.map_err(|e| {
            error!("{:?}", e);
            Error::internal_error()
        })??)
    }
}
