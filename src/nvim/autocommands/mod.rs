use crate::{
    acp::{Result, error::Error},
    nvim::{
        GROUP,
        requests::{RequestHandler, Responder},
    },
    utilities::{ TransmitToNvim, NvimHandler },
};
use core::fmt;
use nvim_oxi::{Array, Dictionary, Object, api::opts::ExecAutocmdsOpts};
use serde::Serialize;
use std::{
    fmt::{Debug, Display},
    sync::Arc,
};
use tracing::{debug, error, instrument, warn};
use uuid::Uuid;

mod event;
mod response;

pub use response::*;

type NvimHandleArgs = (String, serde_json::Value, Option<Uuid>);

pub struct AutoCommand<R: RequestHandler> {
    requests: Arc<R>,
    channel: NvimHandler<NvimHandleArgs>,
}

impl<R: RequestHandler + 'static> AutoCommand<R> {
    #[instrument(level = "trace", skip_all)]
    pub fn new(requests: Arc<R>) -> Result<Self> {
        let nvim_requests = requests.clone();
        let channel = NvimHandler::<NvimHandleArgs>::initialize(
            move |(command, data, request_id)| {
                debug!("Received autocommand: {}, with data: {:#?}", command, data);
                if Self::listener_attached(command.to_string()) {
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
                } else if let Some(id) = request_id {
                    warn!(
                        "No listener attached for command '{}'. Using default implementation",
                        command
                    );
                    nvim_requests
                        .default_response(&id, data)
                        .map_err(|e| {
                            error!(
                                "Failed to send default response for command '{}': {:#?}",
                                command, e
                            )
                        })
                        .ok();
                } else {
                    warn!("No listener attached for command '{}'", command);
                }
            }
        )?;
        Ok(Self { channel, requests })
    }

    #[instrument(level = "trace", skip(self))]
    async fn send_autocommand<C: Debug + ToString, S: Debug + Serialize>(
        &self,
        command: C,
        data: S,
        request_id: Option<Uuid>,
    ) -> Result<()> {
        let serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        debug!("Serialized data: {:#?}", serialized);
        self.channel
            .send((command.to_string(), serialized, request_id))
            .await
            .map_err(|e| Error::Internal(e.to_string()))
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn execute_autocommand<C: Debug + ToString, S: Debug + Serialize>(
        &self,
        command: C,
        data: S,
    ) -> Result<()> {
        self.send_autocommand(command, data, None).await
    }

    #[instrument(level = "trace")]
    pub fn listener_attached<S: Display + Debug>(pattern: S) -> bool {
        let command = pattern.to_string();

        // Workaround for nvim-oxi bug: use call_function with properly constructed arguments
        // The builder pattern sends buffer=nil which Neovim rejects

        let mut opts_dict = Dictionary::default();
        opts_dict.insert("group", GROUP);
        opts_dict.insert("event", Array::from((Object::from("User"),)));
        opts_dict.insert("pattern", Array::from((Object::from(command.clone()),)));

        nvim_oxi::api::call_function::<(Object,), Array>("nvim_get_autocmds", (opts_dict.into(),))
            .map(|commands| !commands.is_empty())
            .map_err(|e| {
                error!("Error detecting autocommand: {:?}", e);
                e
            })
            .unwrap_or(false)
    }

    #[instrument(level = "trace", skip(self, responder))]
    pub async fn execute_autocommand_request<C: Debug + ToString, S: Debug + Serialize>(
        &self,
        session_id: String,
        command: C,
        data: S,
        responder: Responder,
    ) -> Result<()> {
        let mut serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        let request_id = self.requests.add_request(session_id, responder);
        serialized["requestId"] = serde_json::Value::String(request_id.to_string());
        self.send_autocommand(command, serialized, Some(request_id))
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Commands {
    // Permission and tool commands
    WriteTextFile,
    ReadTextFile,
    PermissionRequest,
    ToolCall,
    ToolCallUpdate,
    Plan,
    AvailableCommands,
    ModeCurrent,
    ConfigurationOption,

    // Session lifecycle commands
    ConnectionInitialized,
    SessionCreated,
    Prompted,
    Authenticated,
    ConfigurationUpdated,
    ModeUpdated,
    SessionLoaded,
    SessionsListed,
    SessionForked,
    SessionResumed,
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
            "WriteTextFile" => Commands::WriteTextFile,
            "ReadTextFile" => Commands::ReadTextFile,
            "PermissionRequest" => Commands::PermissionRequest,
            "ToolCall" => Commands::ToolCall,
            "ToolCallUpdate" => Commands::ToolCallUpdate,
            "Plan" => Commands::Plan,
            "AvailableCommands" => Commands::AvailableCommands,
            "ModeCurrent" => Commands::ModeCurrent,
            "ConfigurationOption" => Commands::ConfigurationOption,

            // Session lifecycle commands
            "ConnectionInitialized" => Commands::ConnectionInitialized,
            "SessionCreated" => Commands::SessionCreated,
            "Prompted" => Commands::Prompted,
            "Authenticated" => Commands::Authenticated,
            "ConfigurationUpdated" => Commands::ConfigurationUpdated,
            "ModeUpdated" => Commands::ModeUpdated,
            "SessionLoaded" => Commands::SessionLoaded,
            "SessionsListed" => Commands::SessionsListed,
            "SessionForked" => Commands::SessionForked,
            "SessionResumed" => Commands::SessionResumed,
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
    use pretty_assertions::assert_eq;

    #[test]
    fn test_commands_from_str_basic_variants() {
        // Test predefined variants from different categories
        assert_eq!(
            Commands::from("PermissionRequest"),
            Commands::PermissionRequest
        );
        assert_eq!(Commands::from("ToolCall"), Commands::ToolCall);
        assert_eq!(
            Commands::from("ConnectionInitialized"),
            Commands::ConnectionInitialized
        );
        assert_eq!(Commands::from("UserTextMessage"), Commands::UserTextMessage);
        assert_eq!(
            Commands::from("AgentImageMessage"),
            Commands::AgentImageMessage
        );
    }

    #[test]
    #[should_panic(expected = "Unknown command: InvalidCommand")]
    fn test_commands_from_str_unknown_command_panics() {
        let _ = Commands::from("InvalidCommand");
    }
}
