use std::path::PathBuf;
use std::time::Duration;

use crate::{
    utilities::{autocommand, mock_agent::MockAgent, mock_config::MockConfig},
    TIMEOUT_IN_SECONDS,
};
use agent_client_protocol::{
    InitializeResponse, NewSessionResponse, PromptResponse, ReadTextFileRequest, SessionId,
    StopReason, WriteTextFileRequest,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{conversion::FromObject, Dictionary, Function, Object};
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

/// Data received from the ReadTextFile autocommand.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadTextFileData {
    pub request_id: String,
    pub session_id: SessionId,
    pub path: PathBuf,
}

/// Data received from the WriteTextFile autocommand.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteTextFileData {
    pub request_id: String,
    pub session_id: SessionId,
    pub path: PathBuf,
    pub content: String,
}

fn create_func<A>(plugin: Dictionary, name: &str) -> Function<A, ()> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .unwrap_or_else(|_| panic!("Failed to create function for {}", name))
}

fn make_err(msg: &str) -> nvim_oxi::Error {
    nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(msg.to_string()))
}

/// Test that the ReadTextFile autocommand fires when the mock agent sends a read request.
///
/// Creates a temp file, configures the mock agent to send a ReadTextFileRequest for it,
/// and verifies that Hermes fires the ReadTextFile autocommand with the correct path.
/// The test responds with the file content so the mock agent can proceed.
#[nvim_oxi::test]
fn test_read_file_fires_with_mock_agent() -> Result<(), nvim_oxi::Error> {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file_path = temp_file.path().to_path_buf();
    let file_content = "Hello from Hermes test file!";
    std::fs::write(&file_path, file_content).expect("Failed to write temp file");

    let session_placeholder = SessionId::from("placeholder");

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new().set_read_file_request(ReadTextFileRequest::new(
            session_placeholder.clone(),
            file_path.clone(),
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
    let wait_for_read_file =
        autocommand::listen_for_autocommand::<ReadTextFileData>(Commands::ReadTextFile);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Read the test file");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session.session_id.to_string(), content))?;

    let read_file = wait_for_read_file(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("ReadTextFile autocommand did not fire"))?;

    // Respond with file content so the mock agent can proceed
    respond.call((read_file.request_id.clone(), Object::from(file_content)))?;

    let _prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete after read file workflow"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(read_file.path, file_path);

    Ok(())
}

/// Test that the WriteTextFile autocommand fires when the mock agent sends a write request.
///
/// Creates a temp file path, configures the mock agent to send a WriteTextFileRequest,
/// and verifies that Hermes fires the WriteTextFile autocommand with the correct path
/// and content.
#[nvim_oxi::test]
fn test_write_file_fires_with_mock_agent() -> Result<(), nvim_oxi::Error> {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file_path = temp_file.path().to_path_buf();
    let write_content = "Content written by mock agent";

    let session_placeholder = SessionId::from("placeholder");

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new().set_write_file_request(WriteTextFileRequest::new(
            session_placeholder.clone(),
            file_path.clone(),
            write_content,
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
    let wait_for_write_file =
        autocommand::listen_for_autocommand::<WriteTextFileData>(Commands::WriteTextFile);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Write to the test file");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session.session_id.to_string(), content))?;

    let write_file = wait_for_write_file(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("WriteTextFile autocommand did not fire"))?;

    // Respond to confirm the write so the mock agent can proceed
    respond.call((write_file.request_id.clone(), Object::from("")))?;

    let _prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete after write file workflow"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(write_file.content, write_content);

    Ok(())
}

/// Test that the file is actually written to disk when using the default handler.
///
/// Configures the mock agent to send a WriteTextFileRequest WITHOUT listening
/// for the autocommand, so Hermes uses its default handler which writes the file.
/// After the prompt completes, verifies the file content on disk.
#[nvim_oxi::test]
fn test_write_file_default_handler_writes_to_disk() -> Result<(), nvim_oxi::Error> {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file_path = temp_file.path().to_path_buf();
    let write_content = "Content written via default handler";

    let session_placeholder = SessionId::from("placeholder");

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new().set_write_file_request(WriteTextFileRequest::new(
            session_placeholder.clone(),
            file_path.clone(),
            write_content,
        ));
    }
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect = create_func::<ConnectionArgs>(dict.clone(), "connect");
    let disconnect = create_func::<DisconnectArgs>(dict.clone(), "disconnect");
    let create_session = create_func::<CreateSessionArgs>(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs>(dict.clone(), "prompt");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    // NOTE: intentionally NOT listening for WriteTextFile autocommand
    // so Hermes uses the default handler which writes the file to disk
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Write to the test file");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session.session_id.to_string(), content))?;

    let _prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete after write file workflow"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();
    let actual_content = std::fs::read_to_string(&file_path).expect("Failed to read written file");
    // Neovim buffers always end with a trailing newline when saved to disk
    assert_eq!(actual_content.trim_end(), write_content);

    Ok(())
}

/// Test that the default read handler returns the correct file content.
///
/// Creates a temp file with known content, configures the mock agent to send
/// a ReadTextFileRequest WITHOUT listening for the autocommand, so Hermes uses
/// its default handler which reads the file from disk. Verifies the prompt completes
/// (which means the read succeeded and the mock agent received the content).
#[nvim_oxi::test]
fn test_read_file_default_handler_reads_from_disk() -> Result<(), nvim_oxi::Error> {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file_path = temp_file.path().to_path_buf();
    let file_content = "Content to be read by default handler";
    std::fs::write(&file_path, file_content).expect("Failed to write temp file");

    let session_placeholder = SessionId::from("placeholder");

    let (agent, conn_rx) = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new().set_read_file_request(ReadTextFileRequest::new(
            session_placeholder.clone(),
            file_path.clone(),
        ));
    }
    let mock_handle = MockAgent::start(agent, conn_rx).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect = create_func::<ConnectionArgs>(dict.clone(), "connect");
    let disconnect = create_func::<DisconnectArgs>(dict.clone(), "disconnect");
    let create_session = create_func::<CreateSessionArgs>(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs>(dict.clone(), "prompt");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    // NOTE: intentionally NOT listening for ReadTextFile autocommand
    // so Hermes uses the default handler which reads the file from disk
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    let mut options = Dictionary::new();
    options.insert("protocol", "tcp");
    options.insert("host", "localhost");
    options.insert("port", mock_handle.port() as i64);

    connect.call((nvim_oxi::String::from("mock-agent"), Some(options)))?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Read the test file");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session.session_id.to_string(), content))?;

    // If the default handler fails to read, the mock agent will error and the prompt
    // will not complete, causing this to time out
    let prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete - default read handler may have failed"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(prompt_response.stop_reason, StopReason::EndTurn);

    Ok(())
}
