use crate::{
    Handler,
    acp::{
        Result,
        connection::{Assistant, UserRequest},
        error::Error,
        handler::build_client,
    },
};
use agent_client_protocol::{self as acp, ConnectionTo};
use async_channel::Receiver;
use std::sync::Arc;
use tracing::{debug, error, instrument, trace};

#[instrument(level = "trace", skip(cx, client))]
async fn dispatch(
    cx: &ConnectionTo<acp::Agent>,
    client: &Arc<Handler>,
    agent: &Assistant,
    msg: UserRequest,
) -> Result<()> {
    match msg {
        UserRequest::Initialize(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.initialized(agent, response).await?;
        }
        UserRequest::Cancel(notification) => {
            cx.send_notification(notification)?;
        }
        UserRequest::Prompt(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.prompted(response).await?;
        }
        UserRequest::Authenticate(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.authenticated(response).await?;
        }
        UserRequest::TerminalAuthentication(agent, request) => {
            client
                .terminal_authentication(agent, client.clone(), request)
                .await?;
        }
        UserRequest::SetConfigOption(request) => {
            let session_id = request.session_id.to_string();
            let response = cx.send_request(request).block_task().await?;
            client.config_option_set(&session_id, response).await?;
        }
        UserRequest::SetMode(request) => {
            let session_id = request.session_id.to_string();
            let mode = request.mode_id.to_string();
            let response = cx.send_request(request).block_task().await?;
            client
                .session_mode_set(&session_id, &mode, response)
                .await?;
        }
        UserRequest::CreateSession(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.session_created(response).await?;
        }
        UserRequest::LoadSession(request) => {
            let session_id = request.session_id.clone();
            let response = cx.send_request(request).block_task().await?;
            client
                .session_loaded(session_id.to_string(), response)
                .await?;
        }
        UserRequest::ListSessions(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.sessions_listed(response).await?;
        }
        UserRequest::ForkSession(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.session_forked(response).await?;
        }
        UserRequest::ResumeSession(request) => {
            let session_id = request.session_id.clone();
            let response = cx.send_request(request).block_task().await?;
            client
                .session_resumed(session_id.to_string(), response)
                .await?;
        }
        UserRequest::CloseSession(request) => {
            let session_id = request.session_id.to_string();
            let response = cx.send_request(request).block_task().await?;
            client.session_closed(session_id, response).await?;
        }
        UserRequest::DeleteSession(request) => {
            let session_id = request.session_id.to_string();
            let response = cx.send_request(request).block_task().await?;
            client.session_deleted(session_id, response).await?;
        }
        UserRequest::Logout(request) => {
            let response = cx.send_request(request).block_task().await?;
            client.logged_out(response).await?;
        }
        UserRequest::Close => return Err(Error::InvalidInput(format!("{:?}", msg))),
    }
    Ok(())
}

#[instrument(level = "trace", skip(client, receiver, transport))]
pub async fn handle_connection<T>(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
    transport: T,
) -> Result<()>
where
    T: acp::ConnectTo<acp::Client> + Send + 'static,
{
    trace!("Starting ACP client connection for '{}'", agent);

    let agent_for_main = agent.clone();
    let client_for_main = client.clone();

    build_client(client)
        .connect_with(transport, async move |cx| {
            while let Ok(msg) = receiver.recv().await {
                debug!("Received request from '{}': {:#?}", agent, msg);
                if matches!(msg, UserRequest::Close) {
                    debug!("Close requested for '{}'", agent);
                    break;
                }
                if let Err(e) = dispatch(&cx, &client_for_main, &agent_for_main, msg).await {
                    error!("Error dispatching user request for '{}': {:?}", agent, e);
                } else {
                    debug!("Completed request for '{}'", agent);
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| Error::Connection(e.to_string()))
}
