use crate::{
    acp::{Result, error::Error},
    nvim::{GROUP, requests::{RequestHandler, Responder}},
};
use core::fmt;
use nvim_oxi::{Object, api::opts::ExecAutocmdsOpts, libuv::AsyncHandle};
use serde::Serialize;
use std::{
    fmt::{Debug, Display},
    sync::Arc,
};
use tokio::sync::mpsc::{Sender, channel};
use tracing::{debug, error, instrument, trace};
use uuid::Uuid;

mod event;
mod response;

pub use response::*;

pub struct AutoCommand<R: RequestHandler> {
    handle: AsyncHandle,
    requests: Arc<R>,
    channel: Sender<(String, serde_json::Value)>,
}

impl<R: RequestHandler> AutoCommand<R> {
    #[instrument(level = "trace", skip_all)]
    pub fn new(requests: Arc<R>) -> Result<Self> {
        let (sender, mut receiver) = channel::<(String, serde_json::Value)>(100);
        let handle = nvim_oxi::libuv::AsyncHandle::new(move || {
            while let Ok((command, data)) = receiver.try_recv() {
                debug!("Received autocommand: {}, with data: {:#?}", command, data);
                match serde_json::from_value::<Object>(data) {
                    Ok(obj) => {
                        let opts = ExecAutocmdsOpts::builder()
                            .patterns(command.to_string())
                            .data(obj)
                            .group(GROUP)
                            .build();
                        debug!(
                            "Executing autocommand: {} with options: {:#?}",
                            command, opts
                        );
                        if let Err(err) = nvim_oxi::api::exec_autocmds(["User"], &opts) {
                            error!("Error executing autocommand: '{}': {:#?}", command, err);
                        }
                    }
                    Err(e) => error!(
                        "Failed to deserialize autocommand data for '{}': {:#?}",
                        command, e
                    ),
                }
            }
        })
        .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(Self {
            channel: sender,
            handle,
            requests,
        })
    }

    async fn execute_autocommand<C: Debug + ToString, S: Debug + Serialize>(
        &self,
        command: C,
        data: S,
    ) -> Result<()> {
        let serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        debug!("Serialized data: {:#?}", serialized);
        self.channel
            .send((command.to_string(), serialized))
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
        trace!("Triggering callback in Neovim thread");
        self.handle
            .send()
            .map_err(|e| Error::Internal(e.to_string()))
    }

    /// Check if an autocommand is registered for the given pattern
    /// Uses nvim_oxi::api::get_autocmds to check for existing autocommands
    pub async fn listener_attached<S: Display>(&self, pattern: S) -> Result<bool> {
        use nvim_oxi::api::opts::GetAutocmdsOpts;
        
        let opts = GetAutocmdsOpts::builder()
            .group(GROUP)
            .patterns([pattern.to_string().as_str()])
            .build();
        
        match nvim_oxi::api::get_autocmds(&opts) {
            Ok(autocmds) => Ok(autocmds.len() > 0),
            Err(e) => {
                error!("Failed to get autocommands for pattern '{}': {:?}", pattern, e);
                Err(Error::Internal(format!("Failed to check autocommand: {}", e)))
            }
        }
    }

    async fn execute_autocommand_request<C: Debug + ToString, S: Debug + Serialize>(
        &self,
        session_id: String,
        command: C,
        data: S,
        sender: Responder,
    ) -> Result<()> {
        let request_id = Uuid::new_v4();
        let mut serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        serialized["requestId"] = serde_json::Value::String(request_id.to_string());
        self.execute_autocommand(command, serialized).await?;
        self.requests.add_request(session_id, request_id, sender);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Commands {
    // Permission and tool commands
    PermissionRequest,
    ToolCall,
    ToolCallUpdate,
    Plan,
    AvailableCommands,
    CurrentMode,
    ConfigurationOption,

    // Session lifecycle commands
    ConnectionInitialized,
    CreatedSession,
    Prompted,
    Authenticated,
    ConfigurationUpdated,
    ModeUpdated,
    LoadedSession,
    ListedSessions,
    ForkedSession,
    ResumedSession,
    SessionModelUpdated,

    // User message commands (format: User{ContentType}Message)
    UserResourceMessage,
    UserResourceLinkMessage,
    UserImageMessage,
    UserTextMessage,

    // Agent message commands (format: Agent{ContentType}Message)
    AgentResourceMessage,
    AgentResourceLinkMessage,
    AgentImageMessage,
    AgentTextMessage,

    // Agent thought commands (format: Agent{ContentType}Thought)
    AgentResourceThought,
    AgentResourceLinkThought,
    AgentImageThought,
    AgentTextThought,
}

impl From<&str> for Commands {
    fn from(value: &str) -> Self {
        match value {
            // Permission and tool commands
            "PermissionRequest" => Commands::PermissionRequest,
            "ToolCall" => Commands::ToolCall,
            "ToolCallUpdate" => Commands::ToolCallUpdate,
            "Plan" => Commands::Plan,
            "AvailableCommands" => Commands::AvailableCommands,
            "CurrentMode" => Commands::CurrentMode,
            "ConfigurationOption" => Commands::ConfigurationOption,

            // Session lifecycle commands
            "ConnectionInitialized" => Commands::ConnectionInitialized,
            "CreatedSession" => Commands::CreatedSession,
            "Prompted" => Commands::Prompted,
            "Authenticated" => Commands::Authenticated,
            "ConfigUpdated" => Commands::ConfigurationUpdated,
            "ModeUpdated" => Commands::ModeUpdated,
            "LoadedSession" => Commands::LoadedSession,
            "ListedSessions" => Commands::ListedSessions,
            "ForkedSession" => Commands::ForkedSession,
            "ResumedSession" => Commands::ResumedSession,
            "SessionModelUpdated" => Commands::SessionModelUpdated,

            // User message commands
            "UserResourceMessage" => Commands::UserResourceMessage,
            "UserResourceLinkMessage" => Commands::UserResourceLinkMessage,
            "UserImageMessage" => Commands::UserImageMessage,
            "UserTextMessage" => Commands::UserTextMessage,

            // Agent message commands
            "AgentResourceMessage" => Commands::AgentResourceMessage,
            "AgentResourceLinkMessage" => Commands::AgentResourceLinkMessage,
            "AgentImageMessage" => Commands::AgentImageMessage,
            "AgentTextMessage" => Commands::AgentTextMessage,

            // Agent thought commands
            "AgentResourceThought" => Commands::AgentResourceThought,
            "AgentResourceLinkThought" => Commands::AgentResourceLinkThought,
            "AgentImageThought" => Commands::AgentImageThought,
            "AgentTextThought" => Commands::AgentTextThought,

            _ => panic!("Unknown command: {}", value),
        }
    }
}

impl From<String> for Commands {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Permission and tool commands
    #[test]
    fn test_permission_request_from_str() {
        assert_eq!(Commands::from("PermissionRequest"), Commands::PermissionRequest);
    }

    #[test]
    fn test_tool_call_from_str() {
        assert_eq!(Commands::from("ToolCall"), Commands::ToolCall);
    }

    #[test]
    fn test_tool_call_update_from_str() {
        assert_eq!(Commands::from("ToolCallUpdate"), Commands::ToolCallUpdate);
    }

    #[test]
    fn test_plan_from_str() {
        assert_eq!(Commands::from("Plan"), Commands::Plan);
    }

    #[test]
    fn test_available_commands_from_str() {
        assert_eq!(Commands::from("AvailableCommands"), Commands::AvailableCommands);
    }

    #[test]
    fn test_current_mode_from_str() {
        assert_eq!(Commands::from("CurrentMode"), Commands::CurrentMode);
    }

    #[test]
    fn test_configuration_option_from_str() {
        assert_eq!(Commands::from("ConfigurationOption"), Commands::ConfigurationOption);
    }

    // Session lifecycle commands
    #[test]
    fn test_connection_initialized_from_str() {
        assert_eq!(Commands::from("ConnectionInitialized"), Commands::ConnectionInitialized);
    }

    #[test]
    fn test_created_session_from_str() {
        assert_eq!(Commands::from("CreatedSession"), Commands::CreatedSession);
    }

    #[test]
    fn test_prompted_from_str() {
        assert_eq!(Commands::from("Prompted"), Commands::Prompted);
    }

    #[test]
    fn test_authenticated_from_str() {
        assert_eq!(Commands::from("Authenticated"), Commands::Authenticated);
    }

    #[test]
    fn test_configuration_updated_from_str() {
        assert_eq!(Commands::from("ConfigUpdated"), Commands::ConfigurationUpdated);
    }

    #[test]
    fn test_mode_updated_from_str() {
        assert_eq!(Commands::from("ModeUpdated"), Commands::ModeUpdated);
    }

    #[test]
    fn test_loaded_session_from_str() {
        assert_eq!(Commands::from("LoadedSession"), Commands::LoadedSession);
    }

    #[test]
    fn test_listed_sessions_from_str() {
        assert_eq!(Commands::from("ListedSessions"), Commands::ListedSessions);
    }

    #[test]
    fn test_forked_session_from_str() {
        assert_eq!(Commands::from("ForkedSession"), Commands::ForkedSession);
    }

    #[test]
    fn test_resumed_session_from_str() {
        assert_eq!(Commands::from("ResumedSession"), Commands::ResumedSession);
    }

    #[test]
    fn test_session_model_updated_from_str() {
        assert_eq!(Commands::from("SessionModelUpdated"), Commands::SessionModelUpdated);
    }

    // User message commands
    #[test]
    fn test_user_resource_message_from_str() {
        assert_eq!(Commands::from("UserResourceMessage"), Commands::UserResourceMessage);
    }

    #[test]
    fn test_user_resource_link_message_from_str() {
        assert_eq!(Commands::from("UserResourceLinkMessage"), Commands::UserResourceLinkMessage);
    }

    #[test]
    fn test_user_image_message_from_str() {
        assert_eq!(Commands::from("UserImageMessage"), Commands::UserImageMessage);
    }

    #[test]
    fn test_user_text_message_from_str() {
        assert_eq!(Commands::from("UserTextMessage"), Commands::UserTextMessage);
    }

    // Agent message commands
    #[test]
    fn test_agent_resource_message_from_str() {
        assert_eq!(Commands::from("AgentResourceMessage"), Commands::AgentResourceMessage);
    }

    #[test]
    fn test_agent_resource_link_message_from_str() {
        assert_eq!(Commands::from("AgentResourceLinkMessage"), Commands::AgentResourceLinkMessage);
    }

    #[test]
    fn test_agent_image_message_from_str() {
        assert_eq!(Commands::from("AgentImageMessage"), Commands::AgentImageMessage);
    }

    #[test]
    fn test_agent_text_message_from_str() {
        assert_eq!(Commands::from("AgentTextMessage"), Commands::AgentTextMessage);
    }

    // Agent thought commands
    #[test]
    fn test_agent_resource_thought_from_str() {
        assert_eq!(Commands::from("AgentResourceThought"), Commands::AgentResourceThought);
    }

    #[test]
    fn test_agent_resource_link_thought_from_str() {
        assert_eq!(Commands::from("AgentResourceLinkThought"), Commands::AgentResourceLinkThought);
    }

    #[test]
    fn test_agent_image_thought_from_str() {
        assert_eq!(Commands::from("AgentImageThought"), Commands::AgentImageThought);
    }

    #[test]
    fn test_agent_text_thought_from_str() {
        assert_eq!(Commands::from("AgentTextThought"), Commands::AgentTextThought);
    }

    // String and Display trait tests
    #[test]
    fn test_commands_from_string_delegates_to_str() {
        let string_value = String::from("PermissionRequest");
        assert_eq!(Commands::from(string_value), Commands::PermissionRequest);
    }

    #[test]
    fn test_commands_display_outputs_debug_format() {
        let command = Commands::ToolCall;
        let display_output = format!("{}", command);
        let debug_output = format!("{:?}", command);
        assert_eq!(display_output, debug_output);
    }

    #[test]
    #[should_panic(expected = "Unknown command: InvalidCommand")]
    fn test_commands_from_str_unknown_command_panics() {
        Commands::from("InvalidCommand");
    }
}
