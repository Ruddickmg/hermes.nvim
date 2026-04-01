use core::fmt;
use std::fmt::{Debug, Display};

use crate::acp::error::Error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Commands {
    // Permission and tool commands
    TerminalRelease,
    TerminalKill,
    TerminalExit,
    TerminalOutput,
    TerminalCreate,
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
    UsageUpdate,

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

impl TryFrom<&str> for Commands {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            // Permission and tool commands
            "CreateTerminal" => Ok(Commands::TerminalCreate),
            "TerminalCreate" => Ok(Commands::TerminalCreate),
            "TerminalRelease" => Ok(Commands::TerminalRelease),
            "TerminalKill" => Ok(Commands::TerminalKill),
            "TerminalExit" => Ok(Commands::TerminalExit),
            "TerminalOutput" => Ok(Commands::TerminalOutput),
            "WriteTextFile" => Ok(Commands::WriteTextFile),
            "ReadTextFile" => Ok(Commands::ReadTextFile),
            "PermissionRequest" => Ok(Commands::PermissionRequest),
            "ToolCall" => Ok(Commands::ToolCall),
            "ToolCallUpdate" => Ok(Commands::ToolCallUpdate),
            "Plan" => Ok(Commands::Plan),
            "AvailableCommands" => Ok(Commands::AvailableCommands),
            "ModeCurrent" => Ok(Commands::ModeCurrent),
            "ConfigurationOption" => Ok(Commands::ConfigurationOption),

            // Session lifecycle commands
            "ConnectionInitialized" => Ok(Commands::ConnectionInitialized),
            "SessionCreated" => Ok(Commands::SessionCreated),
            "Prompted" => Ok(Commands::Prompted),
            "Authenticated" => Ok(Commands::Authenticated),
            "ConfigurationUpdated" => Ok(Commands::ConfigurationUpdated),
            "ModeUpdated" => Ok(Commands::ModeUpdated),
            "SessionLoaded" => Ok(Commands::SessionLoaded),
            "SessionsListed" => Ok(Commands::SessionsListed),
            "SessionForked" => Ok(Commands::SessionForked),
            "SessionResumed" => Ok(Commands::SessionResumed),
            "SessionModelUpdated" => Ok(Commands::SessionModelUpdated),
            "UsageUpdate" => Ok(Commands::UsageUpdate),

            // User message commands
            "UserResourceMessage" => Ok(Commands::UserResourceMessage),
            "UserResourceLinkMessage" => Ok(Commands::UserResourceLinkMessage),
            "UserImageMessage" => Ok(Commands::UserImageMessage),
            "UserTextMessage" => Ok(Commands::UserTextMessage),

            // Agent message commands
            "AgentResourceMessage" => Ok(Commands::AgentResourceMessage),
            "AgentResourceLinkMessage" => Ok(Commands::AgentResourceLinkMessage),
            "AgentImageMessage" => Ok(Commands::AgentImageMessage),
            "AgentTextMessage" => Ok(Commands::AgentTextMessage),

            // Agent thought commands
            "AgentResourceThought" => Ok(Commands::AgentResourceThought),
            "AgentResourceLinkThought" => Ok(Commands::AgentResourceLinkThought),
            "AgentImageThought" => Ok(Commands::AgentImageThought),
            "AgentTextThought" => Ok(Commands::AgentTextThought),

            _ => Err(Error::InvalidInput(format!("Unknown command: {}", value))),
        }
    }
}

impl TryFrom<String> for Commands {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
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
            Commands::try_from("PermissionRequest").unwrap(),
            Commands::PermissionRequest
        );
        assert_eq!(Commands::try_from("ToolCall").unwrap(), Commands::ToolCall);
        assert_eq!(
            Commands::try_from("ConnectionInitialized").unwrap(),
            Commands::ConnectionInitialized
        );
        assert_eq!(
            Commands::try_from("UserTextMessage").unwrap(),
            Commands::UserTextMessage
        );
        assert_eq!(
            Commands::try_from("AgentImageMessage").unwrap(),
            Commands::AgentImageMessage
        );
        // Test UsageUpdate
        assert_eq!(
            Commands::try_from("UsageUpdate").unwrap(),
            Commands::UsageUpdate
        );
    }

    #[test]
    fn test_commands_from_str_unknown_command_returns_error() {
        let result = Commands::try_from("InvalidCommand");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown command"));
    }

    #[test]
    fn test_commands_from_string_delegates_to_str() {
        assert_eq!(
            Commands::try_from("ToolCall".to_string()).unwrap(),
            Commands::ToolCall
        );
    }

    #[test]
    fn test_commands_from_string_unknown_returns_error() {
        let result = Commands::try_from("NotARealCommand".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("NotARealCommand"));
    }
}
