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

    // Tests for Permission and Tool commands
    #[test]
    fn test_commands_terminal_create_variants() {
        assert_eq!(
            Commands::try_from("TerminalCreate").unwrap(),
            Commands::TerminalCreate
        );
        assert_eq!(
            Commands::try_from("CreateTerminal").unwrap(),
            Commands::TerminalCreate
        );
    }

    #[test]
    fn test_commands_terminal_release() {
        assert_eq!(
            Commands::try_from("TerminalRelease").unwrap(),
            Commands::TerminalRelease
        );
    }

    #[test]
    fn test_commands_terminal_kill() {
        assert_eq!(
            Commands::try_from("TerminalKill").unwrap(),
            Commands::TerminalKill
        );
    }

    #[test]
    fn test_commands_terminal_exit() {
        assert_eq!(
            Commands::try_from("TerminalExit").unwrap(),
            Commands::TerminalExit
        );
    }

    #[test]
    fn test_commands_terminal_output() {
        assert_eq!(
            Commands::try_from("TerminalOutput").unwrap(),
            Commands::TerminalOutput
        );
    }

    #[test]
    fn test_commands_write_text_file() {
        assert_eq!(
            Commands::try_from("WriteTextFile").unwrap(),
            Commands::WriteTextFile
        );
    }

    #[test]
    fn test_commands_read_text_file() {
        assert_eq!(
            Commands::try_from("ReadTextFile").unwrap(),
            Commands::ReadTextFile
        );
    }

    #[test]
    fn test_commands_tool_call() {
        assert_eq!(Commands::try_from("ToolCall").unwrap(), Commands::ToolCall);
    }

    #[test]
    fn test_commands_tool_call_update() {
        assert_eq!(
            Commands::try_from("ToolCallUpdate").unwrap(),
            Commands::ToolCallUpdate
        );
    }

    #[test]
    fn test_commands_plan() {
        assert_eq!(Commands::try_from("Plan").unwrap(), Commands::Plan);
    }

    #[test]
    fn test_commands_available_commands() {
        assert_eq!(
            Commands::try_from("AvailableCommands").unwrap(),
            Commands::AvailableCommands
        );
    }

    #[test]
    fn test_commands_mode_current() {
        assert_eq!(
            Commands::try_from("ModeCurrent").unwrap(),
            Commands::ModeCurrent
        );
    }

    #[test]
    fn test_commands_configuration_option() {
        assert_eq!(
            Commands::try_from("ConfigurationOption").unwrap(),
            Commands::ConfigurationOption
        );
    }

    // Tests for Session lifecycle commands
    #[test]
    fn test_commands_connection_initialized() {
        assert_eq!(
            Commands::try_from("ConnectionInitialized").unwrap(),
            Commands::ConnectionInitialized
        );
    }

    #[test]
    fn test_commands_session_created() {
        assert_eq!(
            Commands::try_from("SessionCreated").unwrap(),
            Commands::SessionCreated
        );
    }

    #[test]
    fn test_commands_prompted() {
        assert_eq!(Commands::try_from("Prompted").unwrap(), Commands::Prompted);
    }

    #[test]
    fn test_commands_authenticated() {
        assert_eq!(
            Commands::try_from("Authenticated").unwrap(),
            Commands::Authenticated
        );
    }

    #[test]
    fn test_commands_configuration_updated() {
        assert_eq!(
            Commands::try_from("ConfigurationUpdated").unwrap(),
            Commands::ConfigurationUpdated
        );
    }

    #[test]
    fn test_commands_mode_updated() {
        assert_eq!(
            Commands::try_from("ModeUpdated").unwrap(),
            Commands::ModeUpdated
        );
    }

    #[test]
    fn test_commands_session_loaded() {
        assert_eq!(
            Commands::try_from("SessionLoaded").unwrap(),
            Commands::SessionLoaded
        );
    }

    #[test]
    fn test_commands_sessions_listed() {
        assert_eq!(
            Commands::try_from("SessionsListed").unwrap(),
            Commands::SessionsListed
        );
    }

    #[test]
    fn test_commands_session_forked() {
        assert_eq!(
            Commands::try_from("SessionForked").unwrap(),
            Commands::SessionForked
        );
    }

    #[test]
    fn test_commands_session_resumed() {
        assert_eq!(
            Commands::try_from("SessionResumed").unwrap(),
            Commands::SessionResumed
        );
    }

    #[test]
    fn test_commands_session_model_updated() {
        assert_eq!(
            Commands::try_from("SessionModelUpdated").unwrap(),
            Commands::SessionModelUpdated
        );
    }

    #[test]
    fn test_commands_usage_update() {
        assert_eq!(
            Commands::try_from("UsageUpdate").unwrap(),
            Commands::UsageUpdate
        );
    }

    // Tests for User message commands
    #[test]
    fn test_commands_user_resource_message() {
        assert_eq!(
            Commands::try_from("UserResourceMessage").unwrap(),
            Commands::UserResourceMessage
        );
    }

    #[test]
    fn test_commands_user_resource_link_message() {
        assert_eq!(
            Commands::try_from("UserResourceLinkMessage").unwrap(),
            Commands::UserResourceLinkMessage
        );
    }

    #[test]
    fn test_commands_user_image_message() {
        assert_eq!(
            Commands::try_from("UserImageMessage").unwrap(),
            Commands::UserImageMessage
        );
    }

    #[test]
    fn test_commands_user_text_message() {
        assert_eq!(
            Commands::try_from("UserTextMessage").unwrap(),
            Commands::UserTextMessage
        );
    }

    // Tests for Agent message commands
    #[test]
    fn test_commands_agent_resource_message() {
        assert_eq!(
            Commands::try_from("AgentResourceMessage").unwrap(),
            Commands::AgentResourceMessage
        );
    }

    #[test]
    fn test_commands_agent_resource_link_message() {
        assert_eq!(
            Commands::try_from("AgentResourceLinkMessage").unwrap(),
            Commands::AgentResourceLinkMessage
        );
    }

    #[test]
    fn test_commands_agent_image_message() {
        assert_eq!(
            Commands::try_from("AgentImageMessage").unwrap(),
            Commands::AgentImageMessage
        );
    }

    #[test]
    fn test_commands_agent_text_message() {
        assert_eq!(
            Commands::try_from("AgentTextMessage").unwrap(),
            Commands::AgentTextMessage
        );
    }

    // Tests for Agent thought commands
    #[test]
    fn test_commands_agent_resource_thought() {
        assert_eq!(
            Commands::try_from("AgentResourceThought").unwrap(),
            Commands::AgentResourceThought
        );
    }

    #[test]
    fn test_commands_agent_resource_link_thought() {
        assert_eq!(
            Commands::try_from("AgentResourceLinkThought").unwrap(),
            Commands::AgentResourceLinkThought
        );
    }

    #[test]
    fn test_commands_agent_image_thought() {
        assert_eq!(
            Commands::try_from("AgentImageThought").unwrap(),
            Commands::AgentImageThought
        );
    }

    #[test]
    fn test_commands_agent_text_thought() {
        assert_eq!(
            Commands::try_from("AgentTextThought").unwrap(),
            Commands::AgentTextThought
        );
    }

    // Test Display trait
    #[test]
    fn test_commands_display() {
        assert_eq!(format!("{}", Commands::TerminalCreate), "TerminalCreate");
        assert_eq!(
            format!("{}", Commands::PermissionRequest),
            "PermissionRequest"
        );
        assert_eq!(format!("{}", Commands::UserTextMessage), "UserTextMessage");
        assert_eq!(
            format!("{}", Commands::AgentTextThought),
            "AgentTextThought"
        );
    }
}
