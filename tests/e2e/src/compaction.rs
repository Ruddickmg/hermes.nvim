//! E2E tests for ACP v1 session compaction updates.

use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand, mock_agent::MockAgent, mock_config::MockConfig,
        test_helpers::connect_to_mock_agent,
    },
};
use agent_client_protocol::schema::v1::{
    CompactionCapabilities, CompactionStatus, CompactionSummaryChunk, CompactionUpdate,
    ContentBlock, InitializeResponse, NewSessionResponse, SessionUpdate, TextContent,
};
use hermes::{
    acp::handler::client::HermesNotification,
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, Object, conversion::FromObject};
use pretty_assertions::assert_eq;

fn create_func<A, R>(plugin: Dictionary, name: &str) -> Function<A, R> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .unwrap_or_else(|_| panic!("Failed to create function for {}", name))
}

fn make_err(msg: &str) -> nvim_oxi::Error {
    nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(msg.to_string()))
}

/// Connect to the mock agent, run a prompt, and return the notification
/// payload delivered on the given compaction autocommand.
fn run_compaction_prompt<F>(
    command: Commands,
    configure: F,
) -> Result<HermesNotification, nvim_oxi::Error>
where
    F: FnOnce(MockConfig) -> MockConfig,
{
    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = configure(MockConfig::new());
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
    let wait_for_compaction = autocommand::listen_for_autocommand::<HermesNotification>(command);

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "compact the context");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);
    let _ = prompt.call((session.session_id.to_string(), content))?;

    let notification = wait_for_compaction(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("compaction autocommand did not fire"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    Ok(notification)
}

/// Advertise `session.compaction` when a compaction autocommand listener exists.
#[nvim_oxi::test]
fn test_compaction_capability_advertised_with_listener() -> Result<(), nvim_oxi::Error> {
    let agent = MockAgent::new();
    let config_handle = agent.config().clone();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    // Attach the listener BEFORE connecting so it gates the advertisement.
    let _wait_for_compaction =
        autocommand::listen_for_autocommand::<HermesNotification>(Commands::CompactionUpdate);

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    let compaction = config_handle
        .lock()
        .unwrap()
        .initialize_request
        .as_ref()
        .expect("initialize request should be captured")
        .client_capabilities
        .session
        .clone()
        .expect("session capabilities should be advertised")
        .compaction;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(compaction, Some(CompactionCapabilities::new()));

    Ok(())
}

/// Omit `session.compaction` when no compaction autocommand listener exists.
#[nvim_oxi::test]
fn test_compaction_capability_not_advertised_without_listener() -> Result<(), nvim_oxi::Error> {
    let agent = MockAgent::new();
    let config_handle = agent.config().clone();
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    // NOTE: intentionally NOT listening for the compaction autocommands.

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    let compaction = config_handle
        .lock()
        .unwrap()
        .initialize_request
        .as_ref()
        .expect("initialize request should be captured")
        .client_capabilities
        .session
        .clone()
        .expect("session capabilities should be advertised")
        .compaction;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(compaction, None);

    Ok(())
}

/// A `compaction_update` session notification reaches the `CompactionUpdate`
/// autocommand with its payload intact.
#[nvim_oxi::test]
fn test_compaction_update_fires_autocommand() -> Result<(), nvim_oxi::Error> {
    let expected = CompactionUpdate::new("cmp_001", CompactionStatus::InProgress);

    let notification = run_compaction_prompt(Commands::CompactionUpdate, |config| {
        config.set_compaction_update(expected.clone())
    })?;

    assert_eq!(
        notification.update,
        SessionUpdate::CompactionUpdate(expected)
    );

    Ok(())
}

/// A `compaction_summary_chunk` session notification reaches the
/// `CompactionSummaryChunk` autocommand with its payload intact.
#[nvim_oxi::test]
fn test_compaction_summary_chunk_fires_autocommand() -> Result<(), nvim_oxi::Error> {
    let expected = CompactionSummaryChunk::new(
        "cmp_001",
        ContentBlock::Text(TextContent::new("Retained context")),
    );

    let notification = run_compaction_prompt(Commands::CompactionSummaryChunk, |config| {
        config.set_compaction_summary_chunk(expected.clone())
    })?;

    assert_eq!(
        notification.update,
        SessionUpdate::CompactionSummaryChunk(expected)
    );

    Ok(())
}
