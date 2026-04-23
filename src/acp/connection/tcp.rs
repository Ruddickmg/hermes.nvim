use crate::{
    Handler,
    acp::{
        connection::{Assistant, UserRequest},
        error::Error,
        handler::message::handle_requests,
    },
};
use std::rc::Rc;
use std::sync::Arc;
use async_net::TcpStream;
use async_channel::Receiver;
use futures_lite::io::split;
use tracing::{debug, error, info, instrument, trace};

/// Connect to an agent via TCP tcp
///
/// # Arguments
/// * `receiver` - Channel to receive user requests (prompts, cancellations, etc.)
/// * `client` - The Handler that processes agent requests
/// * `agent` - Assistant identifier for logging
/// * `host` - Host address (e.g., "localhost")
/// * `port` - TCP port number
/// * `executor` - The LocalExecutor driving this thread's async tasks
#[instrument(level = "trace", skip(client, receiver, executor))]
pub async fn tcp_connection(
    receiver: Receiver<UserRequest>,
    client: Arc<Handler>,
    agent: &Assistant,
    host: &str,
    port: u16,
    executor: &Rc<smol::LocalExecutor<'static>>,
) -> Result<(), Error> {
    let address = format!("{}:{}", host, port);
    debug!("Connecting to agent at {}", address);

    // Connect to the TCP tcp
    let stream = TcpStream::connect(&address)
        .await
        .map_err(|e| Error::Connection(format!("Failed to connect to {}: {}", address, e)))?;

    info!("Connected to agent '{}' via tcp at {}", agent, address);

    // Split the stream into read and write halves
    let (reader, writer) = split(stream);

    // Clone the executor Rc for the spawn closure (must be 'static)
    let exec_for_spawn = executor.clone();

    trace!("creating ACP client connection");
    let (connection, handle_io) = agent_client_protocol::ClientSideConnection::new(
        client.clone(),
        writer,
        reader,
        move |fut| {
            exec_for_spawn.spawn(fut).detach();
        },
    );

    trace!("starting IO handling task for ACP connection");
    executor.spawn(handle_io).detach();

    handle_requests(connection, receiver, client.clone(), agent).await;

    info!("Disconnected from '{}' via tcp", agent);
    Ok::<(), Error>(())
}

/// Connect to an agent using tcp protocol
///
/// This is the entry point for tcp-based connections, matching the
/// signature of stdio::connect for consistency.
#[instrument(level = "trace", skip(client, receiver, executor))]
pub async fn connect(
    client: Arc<Handler>,
    agent: Assistant,
    receiver: Receiver<UserRequest>,
    executor: &Rc<smol::LocalExecutor<'static>>,
) -> Result<(), Error> {
    match agent.clone() {
        Assistant::CustomUrl { host, port, .. } => {
            trace!("Starting custom agent connection: {}", agent);
            tcp_connection(receiver, client, &agent, &host, port, executor).await
        }
        _ => {
            error!("Unsupported agent type for tcp connection: {}", agent);
            Ok(())
        }
    }
}
