//! E2E tests for cancelling a pending incoming (agent -> client) request.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand, mock_agent::MockAgent, mock_config::MockConfig,
        test_helpers::connect_to_mock_agent,
    },
};
use agent_client_protocol::schema::v1::{
    InitializeResponse, NewSessionResponse, ReadTextFileRequest, SessionId,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, Object, conversion::FromObject};
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

fn create_func<A, R>(plugin: Dictionary, name: &str) -> Function<A, R> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .unwrap_or_else(|_| panic!("Failed to create function for {}", name))
}

fn make_err(msg: &str) -> nvim_oxi::Error {
    nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(msg.to_string()))
}

/// Test that cancelling a pending incoming request sends `$/cancel_request` to
/// the agent for the original request id.
///
/// The mock agent sends a `ReadTextFileRequest` during the prompt. Instead of
/// responding to the `ReadTextFile` autocommand, the test cancels it by its
/// Hermes request id. The agent records the request id it generated for the
/// outgoing read request and the request id carried by the `$/cancel_request`
/// notification, so the test can assert they match.
#[nvim_oxi::test]
fn test_cancel_pending_read_request_sends_cancel_notification() -> Result<(), nvim_oxi::Error> {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file_path = temp_file.path().to_path_buf();
    std::fs::write(&file_path, "content").expect("Failed to write temp file");

    let session_placeholder = SessionId::from("placeholder");

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new().set_read_file_request(ReadTextFileRequest::new(
            session_placeholder.clone(),
            file_path.clone(),
        ));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");
    let create_session: Function<CreateSessionArgs, ()> =
        create_func(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs, Option<nvim_oxi::String>>(dict.clone(), "prompt");
    let cancel: Function<String, ()> = create_func(dict.clone(), "cancel");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_read_file =
        autocommand::listen_for_autocommand::<ReadTextFileData>(Commands::ReadTextFile);

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Read the test file");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);
    let _ = prompt.call((session.session_id.to_string(), content))?;

    let read_file = wait_for_read_file(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("ReadTextFile autocommand did not fire"))?;

    // Cancel the pending incoming request by its Hermes request id instead of
    // responding. Hermes should ask the agent to cancel the original request.
    cancel.call(read_file.request_id.clone())?;

    let deadline = Instant::now() + Duration::from_secs(TIMEOUT_IN_SECONDS);
    let mut received = None;
    while Instant::now() < deadline {
        let captured = mock_handle.config.lock().unwrap().cancel_request_id.clone();
        if captured.is_some() {
            received = captured;
            break;
        }
        let _ = nvim_oxi::api::command("sleep 100m");
    }

    let expected = mock_handle
        .config
        .lock()
        .unwrap()
        .last_read_file_request_id
        .clone();

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(
        received, expected,
        "Hermes should send $/cancel_request carrying the read request's id"
    );
    Ok(())
}
