use std::time::Duration;

use crate::{
    utilities::{
        autocommand,
        mock_agent::MockAgent,
        mock_config::{
            create_test_create_terminal_request, create_test_terminal_output_request,
            create_test_wait_for_terminal_exit_request, MockConfig,
        },
    },
    TIMEOUT_IN_SECONDS,
};
use agent_client_protocol::{
    InitializeResponse, NewSessionResponse, PromptResponse, SessionId, StopReason, TerminalId,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{conversion::FromObject, Dictionary, Function, Object};
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};

/// Data received from the TerminalCreate autocommand.
/// Includes the requestId injected by Hermes for responding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminalCreateData {
    pub request_id: String,
    pub session_id: SessionId,
    pub command: String,
    pub args: Vec<String>,
}

/// Data received from the TerminalOutput autocommand.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminalOutputData {
    pub request_id: String,
    pub session_id: SessionId,
    pub terminal_id: TerminalId,
}

/// Data received from the TerminalExit (WaitForTerminalExit) autocommand.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminalExitData {
    pub request_id: String,
    pub session_id: SessionId,
    pub terminal_id: TerminalId,
}

fn create_func<A>(plugin: Dictionary, name: &str) -> Function<A, ()> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .unwrap_or_else(|_| panic!("Failed to create function for {}", name))
}

fn make_err(msg: &str) -> nvim_oxi::Error {
    nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(msg.to_string()))
}

/// Test that the TerminalCreate autocommand fires with the correct command.
///
/// Configures the mock agent to send a CreateTerminalRequest during prompt.
/// The test responds with a terminal ID so the mock agent can proceed.
#[nvim_oxi::test]
fn test_terminal_create_fires_with_mock_agent() -> Result<(), nvim_oxi::Error> {
    let session_placeholder = SessionId::from("placeholder");

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config =
            MockConfig::new().set_create_terminal_request(create_test_create_terminal_request(
                session_placeholder.clone(),
                "echo",
                vec!["success".to_string()],
            ));
    }
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect = create_func::<ConnectionArgs>(dict.clone(), "connect");
    let disconnect = create_func::<DisconnectArgs>(dict.clone(), "disconnect");
    let create_session = create_func::<CreateSessionArgs>(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs>(dict.clone(), "prompt");
    let respond: Function<(String, Object), ()> = create_func(dict.clone(), "respond");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_terminal_create =
        autocommand::listen_for_autocommand::<TerminalCreateData>(Commands::TerminalCreate);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    let mut options = Dictionary::new();
    options.insert("protocol", "socket");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Run echo in a terminal");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session.session_id.to_string(), content))?;

    let terminal_create = wait_for_terminal_create(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("TerminalCreate autocommand did not fire"))?;

    // Respond with a terminal ID so the mock agent can proceed
    respond.call((
        terminal_create.request_id.clone(),
        Object::from("test-term-1"),
    ))?;

    let _prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete after terminal workflow"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(terminal_create.command, "echo");

    Ok(())
}

/// Test that TerminalOutput autocommand fires after terminal creation.
///
/// Configures the mock agent with both create_terminal and terminal_output requests.
/// The mock agent uses the terminal_id from the CreateTerminalResponse to send
/// the TerminalOutputRequest.
#[nvim_oxi::test]
fn test_terminal_output_fires_with_mock_agent() -> Result<(), nvim_oxi::Error> {
    let session_placeholder = SessionId::from("placeholder");
    let terminal_placeholder = TerminalId::from("placeholder");

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new()
            .set_create_terminal_request(create_test_create_terminal_request(
                session_placeholder.clone(),
                "echo",
                vec!["success".to_string()],
            ))
            .set_terminal_output_request(create_test_terminal_output_request(
                session_placeholder.clone(),
                terminal_placeholder.clone(),
            ));
    }
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect = create_func::<ConnectionArgs>(dict.clone(), "connect");
    let disconnect = create_func::<DisconnectArgs>(dict.clone(), "disconnect");
    let create_session = create_func::<CreateSessionArgs>(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs>(dict.clone(), "prompt");
    let respond: Function<(String, Object), ()> = create_func(dict.clone(), "respond");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_terminal_create =
        autocommand::listen_for_autocommand::<TerminalCreateData>(Commands::TerminalCreate);
    let wait_for_terminal_output =
        autocommand::listen_for_autocommand::<TerminalOutputData>(Commands::TerminalOutput);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    let mut options = Dictionary::new();
    options.insert("protocol", "socket");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Run echo in a terminal");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session.session_id.to_string(), content))?;

    // Respond to TerminalCreate with a terminal ID
    let terminal_create = wait_for_terminal_create(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("TerminalCreate autocommand did not fire"))?;
    respond.call((
        terminal_create.request_id.clone(),
        Object::from("test-term-1"),
    ))?;

    // Wait for and respond to TerminalOutput
    let terminal_output = wait_for_terminal_output(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("TerminalOutput autocommand did not fire"))?;
    respond.call((
        terminal_output.request_id.clone(),
        Object::from("command output"),
    ))?;

    let _prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete after terminal workflow"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        terminal_output.session_id.to_string(),
        session.session_id.to_string(),
    );

    Ok(())
}

/// Test that WaitForTerminalExit autocommand fires after terminal creation.
///
/// Configures the mock agent with create_terminal and wait_for_terminal_exit requests.
/// The mock agent uses the terminal_id from the CreateTerminalResponse to send
/// the WaitForTerminalExitRequest.
#[nvim_oxi::test]
fn test_terminal_exit_fires_with_mock_agent() -> Result<(), nvim_oxi::Error> {
    let session_placeholder = SessionId::from("placeholder");
    let terminal_placeholder = TerminalId::from("placeholder");

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new()
            .set_create_terminal_request(create_test_create_terminal_request(
                session_placeholder.clone(),
                "echo",
                vec!["success".to_string()],
            ))
            .set_wait_for_terminal_exit_request(create_test_wait_for_terminal_exit_request(
                session_placeholder.clone(),
                terminal_placeholder.clone(),
            ));
    }
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect = create_func::<ConnectionArgs>(dict.clone(), "connect");
    let disconnect = create_func::<DisconnectArgs>(dict.clone(), "disconnect");
    let create_session = create_func::<CreateSessionArgs>(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs>(dict.clone(), "prompt");
    let respond: Function<(String, Object), ()> = create_func(dict.clone(), "respond");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_terminal_create =
        autocommand::listen_for_autocommand::<TerminalCreateData>(Commands::TerminalCreate);
    let wait_for_terminal_exit =
        autocommand::listen_for_autocommand::<TerminalExitData>(Commands::TerminalExit);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    let mut options = Dictionary::new();
    options.insert("protocol", "socket");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Run echo in a terminal");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session.session_id.to_string(), content))?;

    // Respond to TerminalCreate with a terminal ID
    let terminal_create = wait_for_terminal_create(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("TerminalCreate autocommand did not fire"))?;
    respond.call((
        terminal_create.request_id.clone(),
        Object::from("test-term-1"),
    ))?;

    // Wait for and respond to TerminalExit with exit code 0
    let terminal_exit = wait_for_terminal_exit(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("TerminalExit autocommand did not fire"))?;
    respond.call((terminal_exit.request_id.clone(), Object::from(0i64)))?;

    let _prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete after terminal workflow"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        terminal_exit.session_id.to_string(),
        session.session_id.to_string(),
    );

    Ok(())
}

/// Test that the full terminal workflow completes: Create, Output, Exit.
///
/// This is the comprehensive test that verifies all three terminal operations
/// fire in sequence and the prompt completes after the workflow.
#[nvim_oxi::test]
fn test_terminal_full_workflow_with_mock_agent() -> Result<(), nvim_oxi::Error> {
    let session_placeholder = SessionId::from("placeholder");
    let terminal_placeholder = TerminalId::from("placeholder");

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new()
            .set_create_terminal_request(create_test_create_terminal_request(
                session_placeholder.clone(),
                "echo",
                vec!["success".to_string()],
            ))
            .set_terminal_output_request(create_test_terminal_output_request(
                session_placeholder.clone(),
                terminal_placeholder.clone(),
            ))
            .set_wait_for_terminal_exit_request(create_test_wait_for_terminal_exit_request(
                session_placeholder.clone(),
                terminal_placeholder.clone(),
            ));
    }
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect = create_func::<ConnectionArgs>(dict.clone(), "connect");
    let disconnect = create_func::<DisconnectArgs>(dict.clone(), "disconnect");
    let create_session = create_func::<CreateSessionArgs>(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs>(dict.clone(), "prompt");
    let respond: Function<(String, Object), ()> = create_func(dict.clone(), "respond");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_terminal_create =
        autocommand::listen_for_autocommand::<TerminalCreateData>(Commands::TerminalCreate);
    let wait_for_terminal_output =
        autocommand::listen_for_autocommand::<TerminalOutputData>(Commands::TerminalOutput);
    let wait_for_terminal_exit =
        autocommand::listen_for_autocommand::<TerminalExitData>(Commands::TerminalExit);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    let mut options = Dictionary::new();
    options.insert("protocol", "socket");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Run echo in a terminal");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session.session_id.to_string(), content))?;

    // Step 1: Respond to TerminalCreate with a terminal ID
    let terminal_create = wait_for_terminal_create(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("TerminalCreate autocommand did not fire"))?;
    respond.call((
        terminal_create.request_id.clone(),
        Object::from("test-term-1"),
    ))?;

    // Step 2: Respond to TerminalOutput
    let _terminal_output = wait_for_terminal_output(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("TerminalOutput autocommand did not fire"))?;
    respond.call((
        _terminal_output.request_id.clone(),
        Object::from("success\n"),
    ))?;

    // Step 3: Respond to TerminalExit with exit code 0
    let _terminal_exit = wait_for_terminal_exit(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("TerminalExit autocommand did not fire"))?;
    respond.call((_terminal_exit.request_id.clone(), Object::from(0i64)))?;

    // Step 4: Wait for prompt to complete
    let prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete after terminal workflow"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(prompt_response.stop_reason, StopReason::EndTurn);

    Ok(())
}
