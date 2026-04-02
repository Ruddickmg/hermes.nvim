use crate::{
    Handler,
    acp::{
        connection::{Assistant, UserRequest},
        error::Error,
        handler::message::handle_request,
    },
};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Receiver;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{debug, error, info, instrument, trace};

/// Connect to an agent via TCP tcp
///
/// # Arguments
/// * `receiver` - Channel to receive user requests (prompts, cancellations, etc.)
/// * `client` - The Handler that processes agent requests
/// * `agent` - Assistant identifier for logging
/// * `host` - Host address (e.g., "localhost")
/// * `port` - TCP port number
#[instrument(level = "trace", skip(client, receiver))]
pub async fn tcp_connection(
    receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: &Assistant,
    host: &str,
    port: u16,
) -> Result<(), Error> {
    let address = format!("{}:{}", host, port);
    debug!("Connecting to agent at {}", address);

    // Connect to the TCP tcp
    let stream = TcpStream::connect(&address)
        .await
        .map_err(|e| Error::Connection(format!("Failed to connect to {}: {}", address, e)))?;

    info!("Connected to agent '{}' via tcp at {}", agent, address);

    // Split the stream into read and write halves
    let (reader, writer) = tokio::io::split(stream);

    // Convert to compat streams for ACP protocol
    let outgoing = writer.compat_write();
    let incoming = reader.compat();

    // Run the ACP protocol handler in a LocalSet
    let local_set = tokio::task::LocalSet::new();
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

    info!("Disconnected from '{}' via tcp", agent);
    Ok::<(), Error>(())
}

/// Connect to an agent using tcp protocol
///
/// This is the entry point for tcp-based connections, matching the
/// signature of stdio::connect for consistency.
#[instrument(level = "trace", skip(client, receiver))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
) -> Result<(), Error> {
    match agent.clone() {
        Assistant::CustomUrl { host, port, .. } => {
            trace!("Starting custom agent connection: {}", agent);
            tcp_connection(receiver, client, &agent, &host, port).await
        }
        _ => {
            error!("Unsupported agent type for tcp connection: {}", agent);
            Ok(())
        }
    }
}
