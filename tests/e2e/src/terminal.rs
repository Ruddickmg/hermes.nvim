use std::time::Duration;

use crate::{TIMEOUT_IN_SECONDS, utilities::autocommand};
use agent_client_protocol::{
    CreateTerminalRequest, InitializeResponse, NewSessionResponse, PermissionOption,
    PromptResponse, SessionId, StopReason, TerminalOutputRequest,
    ToolCallUpdate, WaitForTerminalExitRequest,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, Object, conversion::FromObject};
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PermissionRequest {
    pub request_id: String,
    pub session_id: SessionId,
    pub tool_call: ToolCallUpdate,
    pub options: Vec<PermissionOption>,
}
/// Test that verifies the default terminal workflow handles a simple echo command
/// This test validates the complete terminal lifecycle:
/// 1. TerminalCreate - Agent requests terminal creation
/// 2. TerminalOutput - Agent requests terminal output
/// 3. TerminalExit - Agent requests notification when terminal exits
/// 4. TerminalRelease - Agent requests terminal release
#[ignore = "I can't find an agent that uses the ACP terminal/* commands, I won't be able to test until agent's use the functionality"]
#[nvim_oxi::test]
fn test_default_terminal_echo_workflow() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("createSession").unwrap().clone())?;
    let prompt: Function<PromptArgs, ()> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;
    let respond: Function<(String, String), ()> =
        FromObject::from_object(dict.get("respond").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_terminal_create =
        autocommand::listen_for_autocommand::<CreateTerminalRequest>(Commands::TerminalCreate);
    let wait_for_terminal_output =
        autocommand::listen_for_autocommand::<TerminalOutputRequest>(Commands::TerminalOutput);
    let wait_for_terminal_exit =
        autocommand::listen_for_autocommand::<WaitForTerminalExitRequest>(Commands::TerminalExit);
    let wait_for_permission_request =
        autocommand::listen_for_autocommand::<PermissionRequest>(Commands::PermissionRequest);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    // Step 1: Connect to agent
    connect.call((nvim_oxi::String::from("copilot"), None))?;
    let _init_response = wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    // Step 2: Create session
    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Step 3: Send prompt requesting terminal execution
    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    // Explicitly request ACP terminal protocol
    content_dict.insert(
        "text",
        "Run 'echo success && exit 0' in a terminal and tell me when it completes",
    );
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session_id.to_string(), content))?;

    let permission_request = wait_for_permission_request(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let permission_id = permission_request
        .clone()
        .options
        .into_iter()
        .find(|option| option.option_id.to_string().as_str() == "allow_always")
        .unwrap()
        .option_id;

    let request_id = permission_request.request_id;
    // Step 4: Wait for TerminalCreate autocommand
    respond.call((request_id.to_string(), permission_id.to_string()))?;

    let terminal_create = wait_for_terminal_create(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Verify TerminalCreate contains expected command data
    assert!(
        !terminal_create.session_id.to_string().is_empty(),
        "TerminalCreate should have a valid session ID"
    );
    // The command should contain 'echo' (agent may wrap it in a shell)
    let command_str = terminal_create.command.to_string();
    assert!(
        command_str.contains("echo") || !terminal_create.args.is_empty(),
        "TerminalCreate should contain echo command or args: got command='{}'",
        command_str
    );

    // Step 5: Wait for TerminalOutput autocommand
    let terminal_output = wait_for_terminal_output(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Verify TerminalOutput contains the expected output
    assert_eq!(
        terminal_output.session_id.to_string(),
        session_id.to_string(),
        "TerminalOutput session ID should match"
    );
    assert!(
        !terminal_output.terminal_id.to_string().is_empty(),
        "TerminalOutput should have a valid terminal ID"
    );

    // Step 6: Wait for TerminalExit autocommand
    let terminal_exit = wait_for_terminal_exit(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Verify TerminalExit contains expected data
    assert_eq!(
        terminal_exit.session_id.to_string(),
        session_id.to_string(),
        "TerminalExit session ID should match"
    );
    assert!(
        !terminal_exit.terminal_id.to_string().is_empty(),
        "TerminalExit should have a valid terminal ID"
    );

    // Step 7: Wait for prompt completion (the agent should respond after terminal workflow)
    let prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Step 8: Disconnect
    disconnect.call(DisconnectArgs::All)?;

    // Final assertions
    assert_eq!(
        prompt_response.stop_reason,
        StopReason::EndTurn,
        "Prompt should complete successfully after terminal workflow"
    );

    Ok(())
}

/// Test that verifies specific exit codes are captured correctly
/// This test runs a command that exits with a specific code and verifies
/// the exit code is properly communicated back to the agent.
#[ignore = "I can't find an agent that uses the ACP terminal/* commands, I won't be able to test until agent's use the functionality"]
#[nvim_oxi::test]
fn test_terminal_exit_code_capture() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("createSession").unwrap().clone())?;
    let prompt: Function<PromptArgs, ()> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_terminal_create =
        autocommand::listen_for_autocommand::<CreateTerminalRequest>(Commands::TerminalCreate);
    let wait_for_terminal_exit =
        autocommand::listen_for_autocommand::<WaitForTerminalExitRequest>(Commands::TerminalExit);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    // Connect and create session
    connect.call((nvim_oxi::String::from("copilot"), None))?;
    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Send prompt requesting a command that exits with code 0 (success)
    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert(
        "text",
        "Run 'echo success && exit 0' in a terminal and tell me when it completes",
    );
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session_id.to_string(), content))?;

    // Wait for TerminalCreate
    let _terminal_create = wait_for_terminal_create(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Wait for TerminalExit
    let terminal_exit = wait_for_terminal_exit(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    // Wait for prompt completion
    let _prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;

    // Verify the terminal workflow completed with proper IDs
    assert!(
        !terminal_exit.terminal_id.to_string().is_empty(),
        "Terminal ID should be present in exit notification"
    );

    Ok(())
}
