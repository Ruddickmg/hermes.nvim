use crate::acp::connection::{stdio, Connection};
use crate::nvim::configuration::Permissions;
use crate::PluginState;
use crate::{acp::error::Error, Handler};
use agent_client_protocol::{
    ClientCapabilities, FileSystemCapabilities, Implementation, InitializeRequest, ProtocolVersion,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::thread::JoinHandle;
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, trace, warn};

type ConnectionHandles = Rc<RefCell<HashMap<Assistant, JoinHandle<Result<(), Error>>>>>;

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
    Gemini,
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
            Assistant::Gemini => write!(f, "gemini"),
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
pub struct ConnectionManager {
    handles: ConnectionHandles,
    connection: HashMap<Assistant, Rc<Connection>>,
    state: Arc<Mutex<PluginState>>,
}

impl ConnectionManager {
    #[instrument(level = "trace")]
    pub fn new(state: Arc<Mutex<PluginState>>) -> Self {
        Self {
            handles: Rc::new(RefCell::new(HashMap::new())),
            connection: HashMap::new(),
            state,
        }
    }

    #[instrument(level = "trace", skip(self))]
    fn set_agent(&self, agent: Assistant) {
        let mut config = self.state.blocking_lock();
        config.set_agent(agent);
        drop(config);
    }

    #[instrument(level = "trace", skip(self))]
    fn get_agent(&self) -> Assistant {
        let config = self.state.blocking_lock();
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
    pub fn get_permissions(&self) -> Permissions {
        let config = self.state.blocking_lock();
        let permissions = config.config.permissions.clone();
        drop(config);
        permissions
    }

    #[instrument(level = "trace", skip(self, handler))]
    pub fn connect(
        &mut self,
        handler: Arc<Handler>,
        ConnectionDetails { agent, protocol }: ConnectionDetails,
    ) -> Result<Rc<Connection>, Error> {
        let permissions = self.get_permissions();
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
                let thread_agent = agent.clone();
                let init_config = InitializeRequest::new(ProtocolVersion::LATEST)
                    .client_info(
                        Implementation::new("hermes", env!("CARGO_PKG_VERSION")).title("Hermes"),
                    )
                    .client_capabilities(
                        ClientCapabilities::new()
                            .terminal(permissions.terminal_access)
                            .fs(FileSystemCapabilities::new()
                                .read_text_file(permissions.fs_read_access)
                                .write_text_file(permissions.fs_write_access)),
                    );

                trace!("Starting agent communication in new thread");
                let panic_agent = agent.clone(); // Clone for panic message
                self.handles
                    .try_borrow_mut()
                    .map_err(|e| {
                        Error::Internal(format!("Failed to borrow connection handles: {}", e))
                    })?
                    .insert(
                        agent.clone(),
                        std::thread::spawn(move || {
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                let runtime = tokio::runtime::Builder::new_current_thread()
                                    .enable_all()
                                    .build()
                                    .map_err(|e| Error::Internal(e.to_string()))?;

                                trace!("Starting tokio runtime");
                                runtime.block_on(match protocol {
                                    Protocol::Stdio => {
                                        stdio::connect(handler, thread_agent, receiver)
                                    }
                                    Protocol::Http => unimplemented!(),
                                    Protocol::Socket => unimplemented!(),
                                })
                            }))
                            .map_err(|_| {
                                let err_msg =
                                    format!("Agent '{}' connection thread panicked", panic_agent);
                                error!("{}", err_msg);
                                Error::Internal(err_msg)
                            })?
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
            .try_borrow_mut()
            .map_err(|e| Error::Internal(format!("Failed to borrow connection handles: {}", e)))?
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

impl Drop for ConnectionManager {
    fn drop(&mut self) {
        debug!("ConnectionManager Drop called");
        // Safely join all thread handles to prevent panics from propagating
        match self.handles.try_borrow_mut() {
            Ok(mut handles) => {
                debug!("Joining {} thread handles", handles.len());
                for (agent, handle) in handles.drain() {
                    debug!("Joining thread for agent '{}' during drop", agent);
                    match handle.join() {
                        Ok(result) => {
                            debug!(
                                "Thread for agent '{}' joined successfully: {:?}",
                                agent, result
                            );
                        }
                        Err(_) => {
                            error!("Thread for agent '{}' panicked during drop", agent);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to borrow handles during drop: {}", e);
            }
        }
        debug!("ConnectionManager Drop completed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_assistant_from_str_roundtrip(name in "[a-zA-Z0-9_]*") {
            // Property: converting string to Assistant should never panic
            let _ = Assistant::from(name.as_str());
        }

        #[test]
        fn test_protocol_from_str_roundtrip(name in "[a-zA-Z0-9_]*") {
            // Property: converting string to Protocol should never panic
            let _ = Protocol::from(name.as_str());
        }
    }

    #[test]
    fn test_protocol_display() {
        // Test Display for all Protocol variants using slice comparison
        let protocols: Vec<Protocol> = vec![Protocol::Socket, Protocol::Http, Protocol::Stdio];
        let results: Vec<String> = protocols.iter().map(|p| format!("{}", p)).collect();

        let expected: Vec<String> = vec![
            "socket".to_string(),
            "http".to_string(),
            "stdio".to_string(),
        ];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_protocol_from_str() {
        // Test FromStr for known protocols using slice comparison
        let inputs: Vec<&str> = vec![
            "socket", "http", "stdio", "SOCKET", "HTTP", "STDIO", "unknown",
        ];
        let results: Vec<Protocol> = inputs.iter().map(|&s| Protocol::from(s)).collect();

        let expected: Vec<Protocol> = vec![
            Protocol::Socket, // socket
            Protocol::Http,   // http
            Protocol::Stdio,  // stdio
            Protocol::Socket, // SOCKET (case-insensitive)
            Protocol::Http,   // HTTP (case-insensitive)
            Protocol::Stdio,  // STDIO (case-insensitive)
            Protocol::Stdio,  // unknown defaults to Stdio
        ];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_assistant_display() {
        // Test Display for all Assistant variants using slice comparison
        let assistants: Vec<Assistant> = vec![
            Assistant::Copilot,
            Assistant::Opencode,
            Assistant::Custom {
                name: String::from("my-claude"),
                command: String::from("claude-acp"),
                args: vec![String::from("--socket")],
            },
        ];
        let results: Vec<String> = assistants.iter().map(|a| format!("{}", a)).collect();

        let expected: Vec<String> = vec![
            "copilot".to_string(),
            "opencode".to_string(),
            "my-claude".to_string(),
        ];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_assistant_from_str_copilot_lowercase() {
        assert_eq!(Assistant::from("copilot"), Assistant::Copilot);
    }

    #[test]
    fn test_assistant_from_str_opencode_lowercase() {
        assert_eq!(Assistant::from("opencode"), Assistant::Opencode);
    }

    #[test]
    fn test_assistant_from_str_copilot_uppercase() {
        assert_eq!(Assistant::from("COPILOT"), Assistant::Copilot);
    }

    #[test]
    fn test_assistant_from_str_unknown_creates_custom() {
        let result = Assistant::from("unknown-agent");
        assert!(matches!(result, Assistant::Custom { name, .. } if name == "unknown-agent"));
    }
}
