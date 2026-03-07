use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse, Result,
    SessionNotification, SessionUpdate, TerminalOutputRequest, TerminalOutputResponse,
    WaitForTerminalExitRequest, WaitForTerminalExitResponse, WriteTextFileRequest,
    WriteTextFileResponse,
};

use crate::nvim::{autocommands::AutoCommand, parse};

#[async_trait::async_trait(?Send)]
impl Client for AutoCommand {
    async fn request_permission(
        &self,
        args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        self.execute_autocommand("AgentPermissionRequest".to_string(), args)
            .await?;
        let outcome: RequestPermissionOutcome = RequestPermissionOutcome::Cancelled;
        Ok(RequestPermissionResponse::new(outcome))
    }

    async fn session_notification(&self, session_notification: SessionNotification) -> Result<()> {
        let command = match session_notification.update.clone() {
            SessionUpdate::UserMessageChunk(chunk) => {
                parse::communication(chunk.content).map(|s| format!("User{}Message", s))
            }
            SessionUpdate::AgentMessageChunk(chunk) => {
                parse::communication(chunk.content).map(|s| format!("Agent{}Message", s))
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                parse::communication(chunk.content).map(|s| format!("Agent{}Thought", s))
            }
            SessionUpdate::ToolCall(_) => Ok("AgentToolCall".to_string()),
            SessionUpdate::ToolCallUpdate(_) => Ok("AgentToolCallUpdate".to_string()),
            SessionUpdate::Plan(_) => Ok("AgentPlan".to_string()),
            SessionUpdate::AvailableCommandsUpdate(_) => Ok("AgentAvailableCommands".to_string()),
            SessionUpdate::CurrentModeUpdate(_) => Ok("AgentCurrentMode".to_string()),
            SessionUpdate::ConfigOptionUpdate(_) => Ok("AgentConfigOption".to_string()),
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
