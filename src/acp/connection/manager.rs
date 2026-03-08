use crate::acp::connection::{stdio, Connection};
use crate::nvim::autocommands::ResponseHandler;
use crate::{acp::error::Error, Handler};
use agent_client_protocol::{Client, Implementation, InitializeRequest, ProtocolVersion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument, trace, warn};

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

    #[instrument(level = "trace", skip(self))]
    fn set_agent(&self, agent: Assistant) {
        let mut config = self.handler.state.blocking_lock();
        config.set_agent(agent);
        drop(config);
    }

    #[instrument(level = "trace", skip(self))]
    fn get_agent(&self) -> Assistant {
        let config = self.handler.state.blocking_lock();
        let agent = config.agent.clone();
        drop(config);
        agent
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
        self.get_connection(&self.get_agent())
    }

    #[instrument(level = "trace", skip(self))]
    pub fn connect(
        &mut self,
        ConnectionDetails { agent, protocol }: ConnectionDetails,
    ) -> Result<Rc<Connection>, Error> {
        Ok(match self.get_connection(&agent) {
            Some(connection) => {
                warn!(
                    "A connection already exists for '{}'. Returning existing connection",
                    agent
                );
                connection
            }
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
                self.set_agent(agent.clone());
                connection
            }
        })
    }

    #[instrument(level = "trace", skip(self))]
    pub fn close_all(&mut self) -> Result<(), Error> {
        self.disconnect(self.connection.keys().cloned().collect())?;
        info!("Successfully disconnected from all agents");
        Ok(())
    }

    #[instrument(level = "trace", skip(self))]
    pub fn disconnect(&mut self, assistants: Vec<Assistant>) -> Result<(), Error> {
        let erroneous = assistants
            .clone()
            .into_iter()
            .filter(|assistant| self.disconnect_assistant(assistant).is_err())
            .map(|assistant| assistant.to_string())
            .collect::<Vec<String>>();
        if erroneous.is_empty() {
            debug!("Disconnected from agent(s), {:#?}", assistants);
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
        debug!("Disconnecting from agent {}", assistant);
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
        debug!("Successfully disconnected from agent {}", assistant);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_display_socket() {
        assert_eq!(format!("{}", Protocol::Socket), "socket");
    }

    #[test]
    fn test_protocol_display_http() {
        assert_eq!(format!("{}", Protocol::Http), "http");
    }

    #[test]
    fn test_protocol_display_stdio() {
        assert_eq!(format!("{}", Protocol::Stdio), "stdio");
    }

    #[test]
    fn test_protocol_from_str_socket() {
        assert_eq!(Protocol::from("socket"), Protocol::Socket);
    }

    #[test]
    fn test_protocol_from_str_socket_case_insensitive() {
        assert_eq!(Protocol::from("SOCKET"), Protocol::Socket);
    }

    #[test]
    fn test_protocol_from_str_http() {
        assert_eq!(Protocol::from("http"), Protocol::Http);
    }

    #[test]
    fn test_protocol_from_str_stdio() {
        assert_eq!(Protocol::from("stdio"), Protocol::Stdio);
    }

    #[test]
    fn test_protocol_from_str_unknown() {
        assert_eq!(Protocol::from("unknown"), Protocol::Stdio);
    }

    #[test]
    fn test_assistant_display_copilot() {
        assert_eq!(format!("{}", Assistant::Copilot), "copilot");
    }

    #[test]
    fn test_assistant_display_opencode() {
        assert_eq!(format!("{}", Assistant::Opencode), "opencode");
    }

    #[test]
    fn test_assistant_from_str_copilot() {
        assert_eq!(Assistant::from("copilot"), Assistant::Copilot);
    }

    #[test]
    fn test_assistant_from_str_copilot_case_insensitive() {
        assert_eq!(Assistant::from("COPILOT"), Assistant::Copilot);
    }

    #[test]
    fn test_assistant_from_str_opencode() {
        assert_eq!(Assistant::from("opencode"), Assistant::Opencode);
    }

    #[test]
    fn test_assistant_from_str_unknown() {
        assert_eq!(Assistant::from("unknown"), Assistant::Copilot);
    }
}
