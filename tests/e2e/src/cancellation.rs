//! E2E tests for cancelling a pending incoming (agent -> client) request,
//! whether the cancellation is initiated by the client or by the agent.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand, mock_agent::MockAgent, mock_config::MockConfig,
        test_helpers::connect_to_mock_agent,
    },
};
use agent_client_protocol::schema::v1::{
    InitializeResponse, NewSessionResponse, ReadTextFileRequest, RequestId, SessionId,
};
use hermes::nvim::requests::RequestDetails;
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

/// Result of running the client-initiated cancellation flow.
struct ClientInitiatedCancel {
    read_file: ReadTextFileData,
    cancelled: RequestDetails,
    received: Option<RequestId>,
    expected: Option<RequestId>,
}

/// Drive the full client-initiated cancellation flow.
///
/// The mock agent sends a `ReadTextFileRequest` during the prompt. Instead of
/// responding to the `ReadTextFile` autocommand, the test cancels it by its
/// Hermes request id. Hermes should send `$/cancel_request` for the original
/// request id and fire the `RequestCancelled` autocommand locally.
fn run_client_initiated_cancel() -> Result<ClientInitiatedCancel, nvim_oxi::Error> {
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
    let wait_for_cancelled =
        autocommand::listen_for_autocommand::<RequestDetails>(Commands::RequestCancelled);

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
    // responding. Hermes should ask the agent to cancel the original request
    // and fire RequestCancelled locally.
    cancel.call(read_file.request_id.clone())?;

    let cancelled = wait_for_cancelled(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("RequestCancelled autocommand did not fire"))?;

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

    Ok(ClientInitiatedCancel {
        read_file,
        cancelled,
        received,
        expected,
    })
}

/// Cancelling a pending incoming request sends `$/cancel_request` to the agent
/// carrying the original request id.
#[nvim_oxi::test]
fn test_cancel_pending_read_request_sends_cancel_notification() -> Result<(), nvim_oxi::Error> {
    let result = run_client_initiated_cancel()?;

    assert_eq!(
        result.received, result.expected,
        "Hermes should send $/cancel_request carrying the read request's id"
    );
    Ok(())
}

/// Cancelling a pending incoming request fires `RequestCancelled` carrying the
/// cancelled request's id.
#[nvim_oxi::test]
fn test_client_cancelled_read_request_fires_request_cancelled() -> Result<(), nvim_oxi::Error> {
    let result = run_client_initiated_cancel()?;

    assert_eq!(
        result.cancelled.request_id, result.read_file.request_id,
        "RequestCancelled should carry the same request id as the original request"
    );
    Ok(())
}

/// Result of running the agent-initiated cancellation flow.
struct AgentInitiatedCancel {
    read_file: ReadTextFileData,
    cancelled: RequestDetails,
    error_code: Option<i32>,
}

/// Drive the full agent-initiated cancellation flow.
///
/// The mock agent sends a `ReadTextFileRequest` during the prompt, waits until
/// the test signals that Hermes has registered it (observed via the
/// `ReadTextFile` autocommand), then sends `$/cancel_request` for it. The test
/// collects the `RequestCancelled` payload and the error code the agent
/// received for the original request.
fn run_agent_initiated_cancel() -> Result<AgentInitiatedCancel, nvim_oxi::Error> {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let file_path = temp_file.path().to_path_buf();
    std::fs::write(&file_path, "content").expect("Failed to write temp file");

    let session_placeholder = SessionId::from("placeholder");

    let (cancel_tx, cancel_rx) = async_channel::bounded::<()>(1);
    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new()
            .set_read_file_request(ReadTextFileRequest::new(
                session_placeholder.clone(),
                file_path.clone(),
            ))
            .set_cancel_read_file_request(Arc::new(cancel_rx));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");
    let create_session: Function<CreateSessionArgs, ()> =
        create_func(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs, Option<nvim_oxi::String>>(dict.clone(), "prompt");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_read_file =
        autocommand::listen_for_autocommand::<ReadTextFileData>(Commands::ReadTextFile);
    let wait_for_cancelled =
        autocommand::listen_for_autocommand::<RequestDetails>(Commands::RequestCancelled);

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

    // Wait until Hermes has registered the read request, then let the agent
    // cancel it. Waiting for the ReadTextFile autocommand guarantees the
    // pending request exists before the cancellation notification arrives.
    let read_file = wait_for_read_file(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("ReadTextFile autocommand did not fire"))?;
    cancel_tx
        .send_blocking(())
        .map_err(|_| make_err("Failed to signal cancellation"))?;

    let cancelled = wait_for_cancelled(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("RequestCancelled autocommand did not fire"))?;

    let deadline = Instant::now() + Duration::from_secs(TIMEOUT_IN_SECONDS);
    let mut error_code = None;
    while Instant::now() < deadline {
        let captured = mock_handle
            .config
            .lock()
            .unwrap()
            .last_read_file_response_error_code;
        if captured.is_some() {
            error_code = captured;
            break;
        }
        let _ = nvim_oxi::api::command("sleep 100m");
    }

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    Ok(AgentInitiatedCancel {
        read_file,
        cancelled,
        error_code,
    })
}

/// When the agent cancels a pending request, Hermes answers the original
/// request with the standard `$/cancel_request` error code.
#[nvim_oxi::test]
fn test_agent_cancelled_read_request_responds_request_cancelled() -> Result<(), nvim_oxi::Error> {
    let result = run_agent_initiated_cancel()?;

    assert_eq!(
        result.error_code,
        Some(i32::from(
            agent_client_protocol::Error::request_cancelled().code
        )),
        "Hermes should answer a cancelled request with the -32800 error"
    );
    Ok(())
}

/// When the agent cancels a pending request, Hermes drops it and fires the
/// `RequestCancelled` autocommand carrying the cancelled request's id.
#[nvim_oxi::test]
fn test_agent_cancelled_read_request_fires_request_cancelled() -> Result<(), nvim_oxi::Error> {
    let result = run_agent_initiated_cancel()?;

    assert_eq!(
        result.cancelled.request_id, result.read_file.request_id,
        "RequestCancelled should carry the same request id as the original request"
    );
    Ok(())
}
