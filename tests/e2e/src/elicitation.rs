//! E2E tests for the elicitation flow.
//!
//! These tests configure the mock agent to send an `elicitation/create` request during a
//! prompt and verify that Hermes fires the `FormElicitation` autocommand with the request
//! data, and that responding via `hermes.respond(...)` flows back to the agent.

use std::time::Duration;

use crate::{
    TIMEOUT_IN_SECONDS,
    utilities::{
        autocommand, mock_agent::MockAgent, mock_config::MockConfig,
        test_helpers::connect_to_mock_agent,
    },
};
use agent_client_protocol::schema::v1::{
    CreateElicitationRequest, ElicitationFormMode, ElicitationSchema, ElicitationScope,
    ElicitationSessionScope, InitializeResponse, NewSessionResponse, PromptResponse, SessionId,
};
use hermes::{
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, Object, conversion::FromObject};
use pretty_assertions::assert_eq;
use serde::Deserialize;

/// Data received from the FormElicitation autocommand.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormElicitationData {
    pub request_id: String,
    pub mode: String,
    pub message: String,
}

fn create_func<A, R>(plugin: Dictionary, name: &str) -> Function<A, R> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .unwrap_or_else(|_| panic!("Failed to create function for {}", name))
}

fn make_err(msg: &str) -> nvim_oxi::Error {
    nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(msg.to_string()))
}

fn create_form_elicitation_request(session_id: SessionId) -> CreateElicitationRequest {
    let schema = ElicitationSchema::new().string("name", true);
    let scope = ElicitationScope::Session(ElicitationSessionScope::new(session_id));
    let mode = ElicitationFormMode::new(scope, schema);
    CreateElicitationRequest::new(mode, "Please enter your name")
}

/// Test that the FormElicitation autocommand fires when the mock agent sends an
/// `elicitation/create` request, and that responding delivers the response back to the agent.
#[nvim_oxi::test]
fn test_form_elicitation_fires_and_responds_with_mock_agent() -> Result<(), nvim_oxi::Error> {
    let session_placeholder = SessionId::from("placeholder");

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new()
            .set_elicitation_request(create_form_elicitation_request(session_placeholder.clone()));
    }
    let mock_handle = MockAgent::start(agent).expect("Failed to start mock agent");

    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> = create_func(dict.clone(), "connect");
    let disconnect: Function<DisconnectArgs, ()> = create_func(dict.clone(), "disconnect");
    let create_session: Function<CreateSessionArgs, ()> =
        create_func(dict.clone(), "create_session");
    let prompt = create_func::<PromptArgs, Option<nvim_oxi::String>>(dict.clone(), "prompt");
    let respond: Function<(String, Object), ()> = create_func(dict.clone(), "respond");

    let wait_for_init =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_elicitation =
        autocommand::listen_for_autocommand::<FormElicitationData>(Commands::FormElicitation);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Ask me for my name");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    let _ = prompt.call((session.session_id.to_string(), content))?;

    let elicitation = wait_for_elicitation(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("FormElicitation autocommand did not fire"))?;

    let mut response_dict = Dictionary::new();
    response_dict.insert("action", Object::from("accept"));
    let mut content = Dictionary::new();
    content.insert("name", Object::from("Alice"));
    response_dict.insert("content", Object::from(content));
    respond.call((elicitation.request_id.clone(), Object::from(response_dict)))?;

    let _prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete after elicitation workflow"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    assert_eq!(elicitation.mode, "form");
    assert_eq!(elicitation.message, "Please enter your name");

    Ok(())
}

/// Test that when no listener is attached to the FormElicitation autocommand, Hermes uses
/// the default (placeholder) response so the agent does not hang.
#[nvim_oxi::test]
fn test_form_elicitation_default_response_with_mock_agent() -> Result<(), nvim_oxi::Error> {
    let session_placeholder = SessionId::from("placeholder");

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        *config = MockConfig::new()
            .set_elicitation_request(create_form_elicitation_request(session_placeholder.clone()));
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
    // NOTE: intentionally NOT listening for the FormElicitation autocommand
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    connect_to_mock_agent(&connect, &mock_handle)?;
    wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS)).map_err(|_| make_err("init timeout"))?;

    create_session.call(CreateSessionArgs::Default)?;
    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("session timeout"))?;

    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Ask me for my name");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    let _ = prompt.call((session.session_id.to_string(), content))?;

    // The default response should let the prompt flow complete even without a listener.
    let _prompt_response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))
        .map_err(|_| make_err("Prompt did not complete after default elicitation response"))?;

    disconnect.call(DisconnectArgs::All)?;
    mock_handle.close();

    Ok(())
}
