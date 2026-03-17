use core::fmt;
use std::fmt::{Debug, Display};

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
            "CreateTerminal" => Commands::TerminalCreate,
            "TerminalKill" => Commands::TerminalKill,
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
