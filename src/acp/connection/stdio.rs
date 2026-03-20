use crate::{
    Handler,
    acp::{
        connection::{Assistant, UserRequest},
        error::Error,
        handler::message::handle_request,
    },
};
use std::fmt::Debug;
use std::{ffi::OsStr, process::Stdio, sync::Arc};
use tokio::sync::mpsc::Receiver;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{info, instrument, trace};

#[instrument(level = "trace", skip(client, receiver))]
pub async fn stdio_connection<I, S>(
    receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: &Assistant,
    command: &str,
    args: I,
) -> Result<(), Error>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr>,
{
    let local_set = tokio::task::LocalSet::new();
    let mut child = tokio::process::Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Error::Connection(e.to_string()))?;

    let outgoing = child
        .stdin
        .take()
        .ok_or_else(|| Error::Connection("Failed to take stdin".to_string()))?
        .compat_write();

    let incoming = child
        .stdout
        .take()
        .ok_or_else(|| Error::Connection("Failed to take stdout".to_string()))?
        .compat();

    trace!("Starting async runtime for ACP communication");
    local_set
        .run_until(async {
            let (connection, handle_io) = agent_client_protocol::ClientSideConnection::new(
                client.clone(),
                outgoing,
                incoming,
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            tokio::task::spawn_local(handle_io);

            handle_request(connection, receiver, client.clone(), agent).await
        })
        .await?;

    drop(child);
    info!("Disconnected from '{}'", agent);
    Ok::<(), Error>(())
}

#[instrument(level = "trace", skip(client, receiver))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    match agent.clone() {
        Assistant::Copilot => {
            trace!("Starting copilot connection");
            stdio_connection(receiver, client, &agent, "copilot", ["--acp"]).await
        }
        Assistant::Opencode => {
            trace!("Starting opencode connection");
            stdio_connection(receiver, client, &agent, "opencode", ["acp"]).await
        }
        Assistant::Gemini => {
            trace!("Starting gemini connection");
            stdio_connection(receiver, client, &agent, "gemini", ["--acp"]).await
        }
        Assistant::Custom { command, args, .. } => {
            trace!("Starting custom agent connection: {}", agent);
            stdio_connection(receiver, client, &agent, &command, args).await
        }
    }
}
