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
use async_channel::Receiver;
use tracing::{error, info, instrument, trace};

#[instrument(level = "trace", skip(client, receiver, stdio))]
pub async fn stdio_connection(
    receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: &Assistant,
    stdio: Arc<Child>,
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

    // async_process types already implement AsyncRead/AsyncWrite
    let outgoing = stdin;
    let incoming = stdout;

    trace!("Starting async runtime for ACP communication");
    
    // Create a local executor for managing local tasks
    let executor = Arc::new(smol::LocalExecutor::new());

    // Channel to send spawn requests from the closure to the main loop
    let (spawn_tx, spawn_rx) = async_channel::unbounded::<std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>>();

    // Clone for the closure
    let spawn_tx_clone = spawn_tx.clone();

    // The connection will be created and run within the executor
    let result = executor.run(async {
        trace!("creating ACP client connection");
        
        // Create the connection with a spawn closure that sends to the channel
        let (connection, handle_io) = agent_client_protocol::ClientSideConnection::new(
            client.clone(),
            outgoing,
            incoming,
            move |fut| {
                // Send the future to the channel
                let _ = spawn_tx_clone.try_send(fut);
            },
        );

        trace!("starting IO handling task for ACP connection");
        // We can't spawn handle_io because it's !Send
        // For now, we'll handle this by processing it after requests
        // TODO: Find a better solution for !Send futures

        // Handle requests
        let req_result = handle_requests(connection, receiver, client.clone(), agent).await;

        // Process any pending spawn requests
        while let Ok(fut) = spawn_rx.try_recv() {
            fut.await;
        }
        
        // Close the spawn channel
        drop(spawn_tx);
        
        req_result;
        Ok::<(), Error>(())
    }).await;

    // Wait for the child to exit (it may have already exited when the ACP
    // connection closed, or we may need to wait briefly)
    let status = stdio.wait().await?;
    info!("Disconnected from '{}' with exit status: {}", agent, status);
    result
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
