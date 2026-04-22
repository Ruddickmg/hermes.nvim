pub mod child;

use crate::{
    Handler,
    acp::{
        connection::{Assistant, UserRequest},
        error::Error,
        handler::message::handle_requests,
    },
};
use child::Child;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{error, info, instrument, trace};

#[instrument(level = "trace", skip(client, receiver, stdio))]
pub async fn stdio_connection(
    receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: &Assistant,
    stdio: Arc<Child>,
) -> Result<(), Error> {
    let local_set = tokio::task::LocalSet::new();

    stdio.initialize(&mut agent.command()?).await?;

    let outgoing = stdio
        .take_stdin()
        .await
        .ok_or_else(|| Error::Connection("Failed to take stdin".to_string()))?
        .compat_write();

    let incoming = stdio
        .take_stdout()
        .await
        .ok_or_else(|| Error::Connection("Failed to take stdout".to_string()))?
        .compat();

    trace!("Starting async runtime for ACP communication");
    local_set
        .run_until(async {
            trace!("creating ACP client connection");
            let (connection, handle_io) = agent_client_protocol::ClientSideConnection::new(
                client.clone(),
                outgoing,
                incoming,
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            trace!("starting IO handling task for ACP connection");
            tokio::task::spawn_local(handle_io);

            handle_requests(connection, receiver, client.clone(), agent).await
        })
        .await;

    // Wait for the child to exit (it may have already exited when the ACP
    // connection closed, or we may need to wait briefly)
    let status = stdio.wait().await?;
    info!("Disconnected from '{}' with exit status: {}", agent, status);
    Ok::<(), Error>(())
}

#[instrument(level = "trace", skip(client, receiver, stdio))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
    stdio: Arc<Child>,
) -> Result<(), Error> {
    match agent.clone() {
        Assistant::Copilot
        | Assistant::Opencode
        | Assistant::Gemini
        | Assistant::CustomStdio { .. } => {
            trace!("Starting stdio connection for '{}'", agent);
            stdio_connection(receiver, client, &agent, stdio).await
        }
        _ => {
            error!("Unsupported agent type for stdio connection: {}", agent);
            Ok(())
        }
    }
}
