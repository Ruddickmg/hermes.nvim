use crate::{
    acp::{Result, error::Error},
    nvim::{
        GROUP,
        requests::{RequestHandler, Responder},
    },
};
use core::fmt;
use nvim_oxi::{
    Array, Dictionary, Object,
    api::{opts::ExecAutocmdsOpts},
    libuv::AsyncHandle,
};
use serde::Serialize;
use std::{
    fmt::{Debug, Display},
    sync::Arc,
};
use tokio::sync::mpsc::{Sender, channel};
use tracing::{debug, error, instrument, trace, warn};
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
                } else {
                    warn!("No listener attached for command '{}'", command);
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

    #[instrument(level = "trace", skip(self))]
    pub async fn execute_autocommand<C: Debug + ToString, S: Debug + Serialize>(
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
                error!("Error calling nvim_get_autocmds: {:?}", e);
                e
            })
            // if we can't tell whether an autocommand is registered, we might as well try to trigger it just in case.
            .unwrap_or(true)
    }

    #[instrument(level = "trace", skip(self, sender))]
    pub async fn execute_autocommand_request<C: Debug + ToString, S: Debug + Serialize>(
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
    WriteTextFile,
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
