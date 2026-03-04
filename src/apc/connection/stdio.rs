use crate::{
    Handler,
    apc::{
        connection::{Assistant, UserRequest},
        error::Error,
        handler::message::handle_request,
    }, nvim::autocommands::ResponseHandler,
};
use agent_client_protocol::Client;
use std::{ffi::OsStr, process::Stdio, sync::Arc};
use tokio::sync::mpsc::Receiver;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub async fn stdio_connection<H, I, S>(
    reciever: Receiver<UserRequest>,
    client: Arc<Handler<H>>,
    command: &str,
    args: I,
) -> Result<(), Error>
where
    H: Client + ResponseHandler + 'static,
    I: IntoIterator<Item = S>,
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

            handle_request(connection, reciever, client).await
        })
        .await?;
    drop(child);
    Ok::<(), Error>(())
}

pub async fn connect<H: Client + ResponseHandler + 'static>(
    client: Arc<Handler<H>>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    match agent.clone() {
        Assistant::Copilot => {
            stdio_connection(
                receiver,
                client,
                "node",
                ["copilot-language-server", "--acp"],
            )
            .await
        }
        Assistant::Opencode => stdio_connection(receiver, client, "opencode", ["acp"]).await,
    }
}
