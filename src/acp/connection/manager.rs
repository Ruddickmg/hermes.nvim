use crate::acp::connection::{Connection, stdio};
use crate::nvim::autocommands::ResponseHandler;
use crate::{Handler, acp::error::Error};
use agent_client_protocol::{Client, Implementation, InitializeRequest, ProtocolVersion};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::{sync::Mutex, task};
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
    Custom {
        name: String,
        command: String,
        args: Vec<String>,
    },
}

impl std::fmt::Display for Assistant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Assistant::Copilot => write!(f, "copilot"),
            Assistant::Opencode => write!(f, "opencode"),
            Assistant::Custom { name, .. } => write!(f, "{}", name),
        }
    }
}

impl From<&str> for Assistant {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "copilot" => Assistant::Copilot,
            "opencode" => Assistant::Opencode,
            _ => Assistant::Custom {
                name: s.to_string(),
                command: String::new(),
                args: Vec::new(),
            },
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
pub struct ConnectionManager<H: Client + ResponseHandler> {
    handles: Arc<Mutex<HashMap<Assistant, JoinHandle<Result<(), Error>>>>>,
    connection: HashMap<Assistant, Rc<Connection>>,
    handler: Arc<Handler<H>>,
}

impl<H: Client + ResponseHandler + Sync + Send + 'static> ConnectionManager<H> {
    #[instrument(level = "trace", skip(client))]
    pub fn new(client: Arc<Handler<H>>) -> Self {
        Self {
            handler: client,
            handles: Arc::new(Mutex::new(HashMap::new())),
            connection: HashMap::new(),
        }
    }

    #[instrument(level = "trace", skip(self))]
    async fn set_agent(&self, agent: Assistant) {
        let mut config = self.handler.state.lock().await;
        config.set_agent(agent);
        drop(config);
    }

    #[instrument(level = "trace", skip(self))]
    async fn get_agent(&self) -> Assistant {
        let config = self.handler.state.lock().await;
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
    pub async fn get_current_connection(&self) -> Option<Rc<Connection>> {
        self.get_connection(&self.get_agent().await)
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn connect(
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
                let mut handles = self.handles.lock().await;
                handles.insert(
                    agent.clone(),
                    task::spawn(async move {
                        let runtime = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .map_err(|e| Error::Internal(e.to_string()))?;

                        runtime.block_on(match protocol {
                            Protocol::Stdio => stdio::connect(handler, thread_agent, receiver),
                            Protocol::Http => unimplemented!(),
                            Protocol::Socket => unimplemented!(),
                        })
                    }),
                );
                drop(handles);
                self.add_connection(agent.clone(), connection.clone());
                debug!("Stored connection to '{}'", agent);
                connection.initialize(init_config).await?;
                info!("Initialized connection to '{}'", agent);
                self.set_agent(agent.clone());
                connection
            }
        })
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn close_all(&mut self) -> Result<(), Error> {
        self.disconnect(self.connection.keys().cloned().collect()).await?;
        info!("Successfully disconnected from all agents");
        Ok(())
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn disconnect(&mut self, assistants: Vec<Assistant>) -> Result<(), Error> {
        let senders = self.connection.extract_if(|k, _| assistants.contains(k));
        let mut handles = self.handles.lock().await;
        let removed = handles.extract_if(|k, _| assistants.contains(k));
        senders.for_each(|(assistant, sender)| {
            debug!("Disconnecting from agent {}", assistant);
            drop(sender);
        });
        join_all(removed.into_iter().map(|(_, j)| j))
            .await;
        drop(handles);
        debug!("Disconnected from agent(s), {:#?}", assistants);
        Ok(())
    }

    #[instrument(level = "trace", skip(self))]
    async fn disconnect_assistant(&mut self, assistant: &Assistant) -> Result<(), Error> {
        self.disconnect(vec![assistant.clone()]).await
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
        let assistant = Assistant::from("unknown");
        assert!(matches!(assistant, Assistant::Custom { name, .. } if name == "unknown"));
    }

    #[test]
    fn test_assistant_display_custom() {
        let assistant = Assistant::Custom {
            name: String::from("my-claude"),
            command: String::from("claude-acp"),
            args: vec![String::from("--socket")],
        };
        assert_eq!(format!("{}", assistant), "my-claude");
    }

    #[test]
    fn test_assistant_from_str_creates_custom() {
        let assistant = Assistant::from("my-custom-agent");
        match assistant {
            Assistant::Custom {
                name,
                command,
                args,
            } => {
                assert_eq!(name, "my-custom-agent");
                assert_eq!(command, "");
                assert!(args.is_empty());
            }
            _ => panic!("Expected Custom variant"),
        }
    }
}
