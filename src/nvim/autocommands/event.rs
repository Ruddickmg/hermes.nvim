use agent_client_protocol::{
    Client, CreateTerminalRequest, CreateTerminalResponse, Error, ReadTextFileRequest,
    ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionRequest, RequestPermissionResponse, Result, SessionNotification,
    SessionUpdate, TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};

use crate::nvim::{autocommands::AutoCommands, parse};

#[async_trait::async_trait(?Send)]
impl Client for AutoCommands {
    async fn request_permission(
        &self,
        _args: RequestPermissionRequest,
    ) -> Result<RequestPermissionResponse> {
        Err(Error::method_not_found())
    }

    async fn session_notification(&self, args: SessionNotification) -> Result<()> {
        let (mut data, command) =
            match args.update {
                SessionUpdate::UserMessageChunk(chunk) => parse::communication(chunk.content)
                    .map(|(dict, t)| (dict, format!("User{}Message", t))),
                SessionUpdate::AgentMessageChunk(chunk) => parse::communication(chunk.content)
                    .map(|(dict, t)| (dict, format!("Agent{}Message", t))),
                SessionUpdate::AgentThoughtChunk(chunk) => parse::communication(chunk.content)
                    .map(|(dict, t)| (dict, format!("Agent{}Thought", t))),
                SessionUpdate::ToolCall(tool_call) => parse::tool_call_event(tool_call)
                    .map(|dict| (dict, "AgentToolCall".to_string())),
                SessionUpdate::ToolCallUpdate(update) => parse::tool_call_update_event(update)
                    .map(|dict| (dict, "AgentToolCallUpdate".to_string())),
                SessionUpdate::Plan(plan) => {
                    parse::plan_event(plan).map(|dict| (dict, "AgentPlan".to_string()))
                }
                SessionUpdate::AvailableCommandsUpdate(update) => {
                    parse::available_commands_event(update)
                        .map(|dict| (dict, "AgentAvailableCommands".to_string()))
                }
                SessionUpdate::CurrentModeUpdate(update) => parse::current_mode_event(update)
                    .map(|dict| (dict, "AgentCurrentMode".to_string())),
                SessionUpdate::ConfigOptionUpdate(update) => parse::config_option_event(update)
                    .map(|dict| (dict, "AgentConfigOption".to_string())),
                _ => return Err(Error::method_not_found()),
            }?;

        data.insert("sessionId", args.session_id.to_string());
        self.schedule_autocommand(command, data.into());
        Ok(())
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
