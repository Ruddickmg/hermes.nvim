use crate::acp::connection::{Connection, stdio};
use crate::nvim::autocommands::ResponseHandler;
use crate::{Handler, acp::error::Error};
use agent_client_protocol::{Client, Implementation, InitializeRequest, ProtocolVersion};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, trace, warn};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::Mutex;

#[derive(PartialEq, Eq, Clone, std::hash::Hash, Serialize, Deserialize, Debug, Default)]
pub enum Protocol {
    Socket,
    Http,
    #[default]
    Stdio,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Socket => write!(f, "socket"),
            Protocol::Http => write!(f, "http"),
            Protocol::Stdio => write!(f, "stdio"),
        }
    }
}

impl From<&str> for Protocol {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "socket" => Protocol::Socket,
            "http" => Protocol::Http,
            "stdio" => Protocol::Stdio,
            _ => Protocol::default(), // Default to Stdio if unrecognized
        }
    }
}

impl From<String> for Protocol {
    fn from(s: String) -> Self {
        Protocol::from(s.as_str())
    }
}

#[derive(PartialEq, Eq, Clone, std::hash::Hash, Serialize, Deserialize, Debug, Default)]
pub enum Assistant {
    #[default]
    Copilot,
    Opencode,
}

impl std::fmt::Display for Assistant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Assistant::Copilot => write!(f, "copilot"),
            Assistant::Opencode => write!(f, "opencode"),
        }
    }
}

impl From<&str> for Assistant {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "copilot" => Assistant::Copilot,
            "opencode" => Assistant::Opencode,
            _ => Assistant::default(),
        }
    }
}

impl From<String> for Assistant {
    fn from(s: String) -> Self {
        Assistant::from(s.as_str())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionDetails {
    pub agent: Assistant,
    pub protocol: Protocol,
}

#[derive(Clone)]
pub struct ConnectionManager<H: Client> {
    handles: Arc<Mutex<HashMap<Assistant, JoinHandle<Result<(), Error>>>>>,
    connection: HashMap<Assistant, Rc<Connection>>,
    handler: Arc<Handler<H>>,
    agent: Assistant,
}

impl<H: Client + ResponseHandler + Sync + Send + 'static> ConnectionManager<H> {
    #[instrument(level = "trace", skip(client))]
    pub fn new(client: Arc<Handler<H>>) -> Self {
        Self {
            agent: Assistant::default(),
            handler: client,
            handles: Arc::new(Mutex::new(HashMap::new())),
            connection: HashMap::new(),
        }
    }

    #[instrument(level = "trace", skip(self, connection))]
    fn add_connection(&mut self, agent: Assistant, connection: Rc<Connection>) {
        self.connection.insert(agent, connection);
    }

    #[instrument(level = "trace", skip(self))]
    pub fn get_connection(&self, agent: &Assistant) -> Option<Rc<Connection>> {
        self.connection.get(agent).cloned()
    }

    #[instrument(level = "trace", skip(self))]
    pub fn get_current_connection(&self) -> Option<Rc<Connection>> {
        self.get_connection(&self.agent)
    }

    #[instrument(level = "trace", skip(self))]
    pub fn connect(
        &mut self,
        ConnectionDetails { agent, protocol }: ConnectionDetails,
    ) -> Result<Rc<Connection>, Error> {
        Ok(match self.get_connection(&agent) {
            Some(connection) => {
                warn!("Returning existing connection");
                connection
            },
            None => {
                let (sender, receiver) = tokio::sync::mpsc::channel(100);
                let connection = Rc::new(Connection::new(sender));
                let handler = self.handler.clone();
                let thread_agent = agent.clone();
                let init_config = InitializeRequest::new(ProtocolVersion::LATEST).client_info(
                    Implementation::new("hermes", env!("CARGO_PKG_VERSION")).title("Hermes"),
                );

                trace!("Starting agent communication in new thread");
                self.handles.blocking_lock().insert(
                    agent.clone(),
                    std::thread::spawn(move || {
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .map_err(|e| Error::Internal(e.to_string()))?;

                        trace!("Starting tokio runtime");
                        runtime.block_on(match protocol {
                            Protocol::Stdio => stdio::connect(handler, thread_agent, receiver),
                            Protocol::Http => unimplemented!(),
                            Protocol::Socket => unimplemented!(),
                        })
                    }),
                );
                self.add_connection(agent.clone(), connection.clone());
                debug!("Stored connection to '{}'", agent);
                connection.initialize(init_config)?;
                info!("Initialized connection to '{}'", agent);
                connection
            }
        })
    }

    #[instrument(level = "trace", skip(self))]
    pub fn close_all(&mut self) -> Result<(), Error> {
        self.disconnect(self.connection.keys().cloned().collect())
    }

    #[instrument(level = "trace", skip(self))]
    pub fn disconnect(&mut self, assistants: Vec<Assistant>) -> Result<(), Error> {
        let erroneous = assistants
            .into_iter()
            .filter(|assistant| self.disconnect_assistant(assistant).is_err())
            .map(|assistant| assistant.to_string())
            .collect::<Vec<String>>();
        if erroneous.is_empty() {
            debug!("Successfully disconnected from all agents");
            Ok(())
        } else {
            Err(Error::Connection(format!(
                "A problem occurred while trying to disconnect from agent(s): {}",
                erroneous.join(", ")
            )))
        }
    }

    #[instrument(level = "trace", skip(self))]
    fn disconnect_assistant(&mut self, assistant: &Assistant) -> Result<(), Error> {
        let sender = self.connection.remove(assistant).ok_or_else(|| {
            Error::Connection(format!("No connection found for assistant {}", assistant))
        })?;
        let handle = self
            .handles
            .blocking_lock()
            .remove(assistant)
            .ok_or_else(|| {
                Error::Connection(format!("No handle found for assistant {}", assistant))
            })?;
        debug!("Disconnecting from assistant {}", assistant);
        drop(sender);
        handle
            .join()
            .map_err(|e| {
                Error::Internal(format!(
                    "Failed to join thread for assistant {}: {:#?}",
                    assistant, e
                ))
            })?
            .map_err(|e| {
                Error::Connection(format!(
                    "Error in connection thread for assistant {}: {:#?}",
                    assistant, e
                ))
            })?;
        Ok(())
    }
}
