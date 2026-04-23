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
use std::rc::Rc;
use std::sync::Arc;
use async_channel::Receiver;
use tracing::{error, info, instrument, trace};

#[instrument(level = "trace", skip(client, receiver, stdio, executor))]
pub async fn stdio_connection(
    receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: &Assistant,
    stdio: Arc<Child>,
    executor: &Rc<smol::LocalExecutor<'static>>,
) -> Result<(), Error> {
    stdio.initialize(&mut agent.command()?).await?;

    let stdin = stdio
        .take_stdin()
        .await
        .ok_or_else(|| Error::Connection("Failed to take stdin".to_string()))?;

    let stdout = stdio
        .take_stdout()
        .await
        .ok_or_else(|| Error::Connection("Failed to take stdout".to_string()))?;

    // async_process types already implement futures::AsyncRead/AsyncWrite
    let outgoing = stdin;
    let incoming = stdout;

    trace!("Starting async runtime for ACP communication");

    // Clone the executor Rc for the spawn closure (must be 'static)
    let exec_for_spawn = executor.clone();

    trace!("creating ACP client connection");
    let (connection, handle_io) = agent_client_protocol::ClientSideConnection::new(
        client.clone(),
        outgoing,
        incoming,
        move |fut| {
            // Spawn onto the same LocalExecutor that drives this entire thread.
            // The outer smol::block_on(executor.run(...)) in manager.rs will
            // poll these tasks, matching the Tokio LocalSet::spawn_local pattern.
            exec_for_spawn.spawn(fut).detach();
        },
    );

    trace!("starting IO handling task for ACP connection");
    // Spawn the IO driver onto the executor so it runs concurrently
    // with handle_requests. This is the critical piece that was missing.
    executor.spawn(handle_io).detach();

    handle_requests(connection, receiver, client.clone(), agent).await;

    // Wait for the child to exit (it may have already exited when the ACP
    // connection closed, or we may need to wait briefly)
    let status = stdio.wait().await?;
    info!("Disconnected from '{}' with exit status: {}", agent, status);
    Ok::<(), Error>(())
}

#[instrument(level = "trace", skip(client, receiver, stdio, executor))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
    stdio: Arc<Child>,
    executor: &Rc<smol::LocalExecutor<'static>>,
) -> Result<(), Error> {
    match agent.clone() {
        Assistant::Copilot
        | Assistant::Opencode
        | Assistant::Gemini
        | Assistant::CustomStdio { .. } => {
            trace!("Starting stdio connection for '{}'", agent);
            stdio_connection(receiver, client, &agent, stdio, executor).await
        }
        _ => {
            error!("Unsupported agent type for stdio connection: {}", agent);
            Ok(())
        }
    }
}
