use std::{sync::Arc};

use tokio::sync::mpsc::Receiver;

use agent_client_protocol::{Agent, ClientSideConnection};
use tracing::{debug, error, instrument};

use crate::{
    Handler,
    acp::{
        Result,
        connection::{Assistant, UserRequest},
        error::Error,
    },
};

async fn handle_request(
    connection: &ClientSideConnection,
    client: &Arc<Handler>,
    agent: &Assistant,
    msg: UserRequest,
) -> Result<()> {
    match msg {
        UserRequest::Initialize(request) => {
            let response = connection.initialize(request).await?;
            client.initialized(agent, response).await?;
        }
        UserRequest::Cancel(config) => {
            connection.cancel(config).await?;
        }
        UserRequest::Prompt(request) => {
            let response = connection.prompt(request).await?;
            client.prompted(response).await?;
        }
        UserRequest::Authenticate(request) => {
            let response = connection.authenticate(request).await?;
            client.authenticated(response).await?;
        }
        UserRequest::SetConfigOption(request) => {
            let response = connection.set_session_config_option(request).await?;
            client.config_option_set(response).await?;
        }
        UserRequest::SetMode(request) => {
            let response = connection.set_session_mode(request).await?;
            client.mode_set(response).await?;
        }
        UserRequest::CreateSession(config) => {
            let response = connection.new_session(config).await?;
            client.session_created(response).await?;
        }
        UserRequest::LoadSession(request) => {
            let response = connection.load_session(request).await?;
            client.session_loaded(response).await?;
        }
        UserRequest::ListSessions(request) => {
            let response = connection.list_sessions(request).await?;
            client.sessions_listed(response).await?;
        }
        UserRequest::ForkSession(request) => {
            let response = connection.fork_session(request).await?;
            client.session_forked(response).await?;
        }
        UserRequest::ResumeSession(request) => {
            let response = connection.resume_session(request).await?;
            client.session_resumed(response).await?;
        }
        UserRequest::SetSessionModel(request) => {
            let response = connection.set_session_model(request).await?;
            client.session_model_set(response).await?;
        }
        _ => return Err(Error::InvalidInput(format!("{:?}", msg))),
    }
    Ok(())
}

#[instrument(level = "trace", skip_all)]
pub async fn handle_requests(
    connection: ClientSideConnection,
    mut receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: &Assistant,
) {
    while let Some(msg) = receiver.recv().await {
        debug!("Received request from '{}': {:#?}", agent, msg);
        if msg == UserRequest::Close {
           break; 
        } else if let Err(e) = handle_request(&connection, &client, agent, msg).await {
            error!("{:?}", e);
        } else {
            debug!("Completed request for '{}'", agent);
        }
    }
}
