use crate::{TIMEOUT_IN_SECONDS, utilities::autocommand};
use agent_client_protocol::{
    InitializeResponse, NewSessionResponse, PermissionOption, PromptResponse, SessionId, ToolCall
};
use hermes::{
    api::{
        ConnectionArgs, CreateSessionArgs, DisconnectArgs, PromptArgs, PromptContent, RespondArgs,
    },
    nvim::{autocommands::Commands, hermes},
};
use nvim_oxi::{Dictionary, Function, conversion::FromObject};
use serde::{Deserialize, Serialize};
use tracing::info;
use std::time::Duration;

fn create_func<A>(plugin: Dictionary, name: &str) -> Function<A, ()> {
    FromObject::from_object(plugin.get(name).unwrap().clone())
        .expect(&format!("Failed to create function for {}", name))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Permissions {
    session_id: SessionId,
    request_id: String,
    tool_call: ToolCall,
    options: Vec<PermissionOption>,
}

// TODO: I can't get opencode to send a permission request, I will need to figure out another way to test this.
#[ignore]
#[nvim_oxi::test]
fn can_chose_a_response_to_a_permission_request() -> Result<(), nvim_oxi::Error> {
    let dict: Dictionary = hermes()?;
    let connect = create_func::<ConnectionArgs>(dict.clone(), "connect");
    let disconnect = create_func::<DisconnectArgs>(dict.clone(), "disconnect");
    let create_session = create_func::<CreateSessionArgs>(dict.clone(), "createSession");
    let prompt = create_func::<PromptArgs>(dict.clone(), "prompt");
    let respond = create_func::<RespondArgs>(dict.clone(), "respond");

    let wait_for_initialization =
        autocommand::listen_for_autocommand::<InitializeResponse>(Commands::ConnectionInitialized);
    let wait_for_session =
        autocommand::listen_for_autocommand::<NewSessionResponse>(Commands::SessionCreated);
    let wait_for_permission_request =
        autocommand::listen_for_autocommand::<Permissions>(Commands::PermissionRequest);
    let wait_for_prompt_finish =
        autocommand::listen_for_autocommand::<PromptResponse>(Commands::Prompted);

    connect.call((nvim_oxi::String::from("opencode"), None))?;

    wait_for_initialization(Duration::from_secs(TIMEOUT_IN_SECONDS))?;

    create_session.call(CreateSessionArgs::Default)?;

    let session = wait_for_session(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    let session_id = session.session_id;
    prompt.call((
        session_id.to_string(),
        PromptContent::Single(hermes::api::ContentBlockType::Text {
            text: "look up the time in france online".to_string(),
        }),
    ))?;

    let mut permission_request =
        wait_for_permission_request(Duration::from_secs(TIMEOUT_IN_SECONDS))?;
    info!("Making permission request: {:?}", permission_request);

    let response = respond.call((
        permission_request.request_id,
        permission_request
            .options
            .pop()
            .unwrap()
            .option_id
            .to_string()
            .into(),
    ));
    info!("Responded to permission request: {:?}", response);
    let finished_prompt = wait_for_prompt_finish(Duration::from_secs(TIMEOUT_IN_SECONDS));
    

    info!("Finished prompt response: {:?}", finished_prompt);
    
    disconnect.call(DisconnectArgs::All)?;

    assert!(
        finished_prompt.is_ok(),
        "Respond autocommand should fire after setMode call"
    );

    Ok(())
}
