use crate::{
    Handler,
    apc::{
        connection::{Assistant, UserRequest},
        error::Error,
        handler::message::handle_request,
    },
};
use agent_client_protocol::Client;
use std::sync::mpsc::Receiver;
use std::{ffi::OsStr, process::Stdio, sync::Arc};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub fn stdio_connection<H, I, S>(
    reciever: Receiver<UserRequest>,
    client: Arc<Handler<H>>,
    command: &str,
    args: I,
) -> Result<(), Error>
where
    H: Client + 'static,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::Connection(e.to_string()))?;
    let local_set = tokio::task::LocalSet::new();

    let mut child = tokio::process::Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

    let _: Result<(), Error> = runtime.block_on(local_set.run_until(async {
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
    }));
    Ok(())
}

pub fn connect<H: Client + 'static>(
    client: Arc<Handler<H>>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    match agent.clone() {
        Assistant::Copilot => stdio_connection(
            receiver,
            client,
            "node",
            ["copilot-language-server", "--acp"],
        ),
        Assistant::Opencode => stdio_connection(receiver, client, "opencode", ["acp"]),
    }
}
