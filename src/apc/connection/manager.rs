use crate::apc::connection::{Connection, stdio};
use crate::{ApcClient, apc::error::Error};
use agent_client_protocol::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(PartialEq, Eq, Clone, std::hash::Hash, Serialize, Deserialize, Debug)]
pub enum Protocol {
    Socket,
    Http,
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

impl Default for Protocol {
    fn default() -> Self {
        Protocol::Stdio
    }
}

impl From<String> for Protocol {
    fn from(s: String) -> Self {
        Protocol::from(s.as_str())
    }
}

#[derive(PartialEq, Eq, Clone, std::hash::Hash, Serialize, Deserialize, Debug)]
pub enum Assistant {
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

impl Default for Assistant {
    fn default() -> Self {
        Assistant::Copilot
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
    handles: Arc<Mutex<Vec<JoinHandle<Result<(), Error>>>>>,
    connection: HashMap<Assistant, Connection>,
    handler: Arc<ApcClient<H>>,
}

impl<H: Client + Sync + Send + 'static> ConnectionManager<H> {
    pub fn new(client: Arc<ApcClient<H>>) -> Result<Self, Error> {
        Ok(Self {
            handler: client,
            handles: Arc::new(Mutex::new(Vec::new())),
            connection: HashMap::new(),
        })
    }

    fn add_connection(&mut self, agent: Assistant, connection: Connection) {
        self.connection.insert(agent, connection);
    }

    pub fn get_connection(&self, agent: &Assistant) -> Option<Connection> {
        self.connection.get(agent).cloned()
    }

    pub fn connect(
        &mut self,
        ConnectionDetails { agent, protocol }: ConnectionDetails,
    ) -> Result<Connection, Error> {
        Ok(match self.get_connection(&agent) {
            Some(connection) => connection,
            None => {
                let (sender, receiver) = std::sync::mpsc::channel();
                let connection = Connection::new(sender);
                let handler = self.handler.clone();
                let thread_agent = agent.clone();
                self.handles
                    .lock()
                    .map_err(|e| Error::Internal(e.to_string()))?
                    .push(std::thread::spawn(move || match protocol {
                        Protocol::Stdio => stdio::connect(handler, thread_agent, receiver),
                        Protocol::Http => unimplemented!(),
                        Protocol::Socket => unimplemented!(),
                    }));
                self.add_connection(agent.clone(), connection.clone());
                connection
            }
        })
    }

    pub fn disconnect(&mut self, assistant: &Assistant) -> Result<(), Error> {
        let sender = self.connection.remove(assistant).ok_or_else(|| {
            Error::Connection(format!("No connection found for assistant {}", assistant))
        })?;
        drop(sender);
        Ok(())
    }
}
