use std::time::Duration;

use crate::{utilities::autocommand, TIMEOUT_IN_SECONDS};
use agent_client_protocol::{InitializeResponse, NewSessionResponse, PromptResponse, StopReason};
use hermes::{
    acp::connection::{Assistant, Protocol},
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{conversion::FromObject, Array, Dictionary, Function, Object};

#[nvim_oxi::test]
fn test_setup_returns_prompt_function() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;

    assert!(
        dict.get("prompt").is_some(),
        "prompt function should be registered"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_prompt_single_content() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<Option<ConnectionArgs>, ()> =
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
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::CreatedSession);
    let wait_for_prompt =
        autocommand::listen_for_autocommand::<PromptResponse>(Commands::AgentPrompted);

    connect.call(Some(ConnectionArgs {
        agent: Some(Assistant::Opencode),
        protocol: Some(Protocol::Stdio),
    }))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Create single text content
    let mut content_dict = Dictionary::new();
    content_dict.insert("type", "text");
    content_dict.insert("text", "Hello, what time is it?");
    let content = PromptContent::Single(FromObject::from_object(Object::from(content_dict))?);

    prompt.call((session_id.to_string(), content))?;

    // Wait longer for agent to process prompt and respond
    let response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;

    assert_eq!(response.stop_reason, StopReason::EndTurn);

    Ok(())
}

#[nvim_oxi::test]
fn test_prompt_multiple_content() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<Option<ConnectionArgs>, ()> =
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
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::CreatedSession);
    let wait_for_prompt =
        autocommand::listen_for_autocommand::<PromptResponse>(Commands::AgentPrompted);

    connect.call(Some(ConnectionArgs {
        agent: Some(Assistant::Opencode),
        protocol: Some(Protocol::Stdio),
    }))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;

    // Create array of all content types
    let mut text_dict = Dictionary::new();
    text_dict.insert("type", "text");
    text_dict.insert("text", "What time is it?");

    let mut link_dict = Dictionary::new();
    link_dict.insert("type", "link");
    link_dict.insert("name", "Example file");
    link_dict.insert("uri", "/path/to/example.txt");

    let mut embedded_text_dict = Dictionary::new();
    embedded_text_dict.insert("type", "embedded");
    let mut text_resource_dict = Dictionary::new();
    text_resource_dict.insert("uri", "file:///home/user/script.py");
    text_resource_dict.insert("mimeType", "text/x-python");
    text_resource_dict.insert("text", "def hello():\n    print('Hello, world!')");
    embedded_text_dict.insert("resource", text_resource_dict);

    let mut embedded_blob_dict = Dictionary::new();
    embedded_blob_dict.insert("type", "embedded");
    let mut blob_resource_dict = Dictionary::new();
    blob_resource_dict.insert("uri", "file:///home/user/document.pdf");
    blob_resource_dict.insert("mimeType", "application/pdf");
    blob_resource_dict.insert("blob", "Base64-encoded-binary-data");
    embedded_blob_dict.insert("resource", blob_resource_dict);

    let mut image_dict = Dictionary::new();
    image_dict.insert("type", "image");
    image_dict.insert("data", "base64-encoded-image-data");
    image_dict.insert("mimeType", "image/png");

    let mut audio_dict = Dictionary::new();
    audio_dict.insert("type", "audio");
    audio_dict.insert("data", "base64-encoded-audio-data");
    audio_dict.insert("mimeType", "audio/wav");

    let content_array = Array::from_iter(vec![
        Object::from(text_dict),
        Object::from(link_dict),
        Object::from(embedded_text_dict),
        Object::from(embedded_blob_dict),
        Object::from(image_dict),
        Object::from(audio_dict),
    ]);

    let content = PromptContent::Multiple(
        content_array
            .into_iter()
            .map(|obj| FromObject::from_object(obj))
            .collect::<Result<Vec<_>, _>>()?,
    );

    prompt.call((session_id.to_string(), content))?;

    // Wait longer for agent to process multiple content blocks and respond
    let response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;

    assert_eq!(response.stop_reason, StopReason::EndTurn);

    Ok(())
}
