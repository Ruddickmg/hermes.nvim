use std::time::Duration;

use agent_client_protocol::{InitializeResponse, NewSessionResponse, PromptResponse, StopReason};
use hermes::{
    acp::connection::Assistant,
    api::{ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent},
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{conversion::FromObject, Dictionary, Function, Object};

use crate::{utilities::autocommand, TIMEOUT_IN_SECONDS};

pub fn test_agent_prompt(agent: Assistant) -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;
    let prompt: Function<PromptArgs, ()> =
        FromObject::from_object(dict.get("prompt").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_prompt = autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    connect.call((agent.to_string().into(), None))?;

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

    // Wait for agent response (mock agent is fast)
    let response = wait_for_prompt(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    disconnect.call(DisconnectArgs::All)?;

    assert_eq!(response.stop_reason, StopReason::EndTurn);

    Ok(())
}

pub fn test_session_creation(agent: Assistant) -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect: Function<ConnectionArgs, ()> =
        FromObject::from_object(dict.get("connect").unwrap().clone())?;
    let disconnect: Function<DisconnectArgs, ()> =
        FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
    let create_session: Function<CreateSessionArgs, ()> =
        FromObject::from_object(dict.get("create_session").unwrap().clone())?;

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);

    connect.call((agent.to_string().into(), None))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS));

    disconnect.call(DisconnectArgs::All)?;

    assert!(session.is_ok());

    Ok(())
}

// fn test_authenticate() -> Result<(), nvim_oxi::Error> {
//     let agent = Assistant::Opencode;
//     let dict: Dictionary = hermes()?;
//     let connect: Function<ConnectionArgs, ()> =
//         FromObject::from_object(dict.get("connect").unwrap().clone())?;
//     let authenticate: Function<String, ()> =
//         FromObject::from_object(dict.get("authenticate").unwrap().clone())?;
//     let disconnect: Function<DisconnectArgs, ()> =
//         FromObject::from_object(dict.get("disconnect").unwrap().clone())?;
//     let wait_for_init =
//         autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
//     let wait_for_authentication =
//         autocommand::listen_for_autocommand::<AuthenticateResponse>(Commands::Authenticated);
//
//     connect.call((agent.to_string().into(), None))?;
//
//     let mut init_response = wait_for_init(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
//     let auth_method = init_response.auth_methods.pop().unwrap();
//     authenticate.call(auth_method.id().to_string())?;
//     let auth_response = wait_for_authentication(Duration::from_secs(TIMEOUT_IN_SECONDS));
//
//     assert!(auth_response.is_ok(), "Authentication should succeed");
//
//     disconnect.call(DisconnectArgs::All)?;
//
//     Ok(())
// }
