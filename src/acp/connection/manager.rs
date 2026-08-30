use crate::PluginState;
use crate::acp::connection::{Connection, http, socket, stdio, tcp};
use crate::acp::registry::distribution::Distribution;
use crate::acp::registry::entry::AgentEntry;
use crate::acp::registry::resolution::fetch_agent_from_registry;
use crate::nvim::configuration::{DistributionsConfig, Permissions};
use crate::{Handler, acp::error::Error};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    ClientCapabilities, ElicitationCapabilities, ElicitationFormCapabilities,
    ElicitationUrlCapabilities, FileSystemCapabilities, Implementation, InitializeRequest,
};
use async_lock::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(PartialEq, Eq, Clone, Copy, std::hash::Hash, Serialize, Deserialize, Debug, Default)]
pub enum Protocol {
    Tcp,
    Http,
    Https,
    Socket,
    #[default]
    Stdio,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Socket => write!(f, "socket"),
            Protocol::Http => write!(f, "http"),
            Protocol::Https => write!(f, "https"),
            Protocol::Stdio => write!(f, "stdio"),
        }
    }
}

impl From<&str> for Protocol {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "tcp" => Protocol::Tcp,
            "socket" => Protocol::Socket,
            "http" => Protocol::Http,
            "https" => Protocol::Https,
            "stdio" => Protocol::Stdio,
            _ => Protocol::Stdio, // default to Stdio for unknown protocols
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
    Registered {
        agent: AgentEntry,
        distribution: Option<Distribution>,
        configuration: DistributionsConfig,
        command: Option<String>,
        args: Option<Vec<String>>,
        #[serde(skip)]
        registry: Option<crate::acp::registry::Registry>,
    },
    CustomStdio {
        name: String,
        command: String,
        args: Vec<String>,
    },
    CustomUrl {
        name: String,
        host: String,
        port: u16,
        path: Option<String>,
    },
    CustomSocket {
        name: String,
        path: String,
    },
}

impl std::fmt::Display for Assistant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Assistant::Copilot => write!(f, "copilot"),
            Assistant::Opencode => write!(f, "opencode"),
            Assistant::Gemini => write!(f, "gemini"),
            Assistant::CustomStdio { name, .. } => write!(f, "{}", name),
            Assistant::CustomUrl {
                name,
                host,
                port,
                path: Some(path),
            } => write!(f, "{} ({}:{}{})", name, host, port, path),
            Assistant::CustomUrl {
                name,
                host,
                port,
                path: None,
            } => write!(f, "{} ({}:{})", name, host, port),
            Assistant::CustomSocket { name, path } => write!(f, "{} (socket:{})", name, path),
            Assistant::Registered {
                agent,
                distribution,
                ..
            } => {
                if let Some(dist) = distribution {
                    write!(f, "{} ({})", agent.id, dist)
                } else {
                    write!(f, "{}", agent.id)
                }
            }
        }
    }
}

impl Assistant {
    /// Build the `async_process::Command` for this agent without spawning it.
    ///
    /// The caller is responsible for spawning the command on the correct
    /// executor (the one whose reactor will handle the child's IO).
    #[instrument(level = "trace", skip(self))]
    pub async fn command(&self) -> crate::acp::Result<async_process::Command> {
        let owned_command;
        let owned_args;
        let (program, args) = match self {
            Assistant::Copilot => ("copilot", vec!["--acp"]),
            Assistant::Opencode => ("opencode", vec!["acp"]),
            Assistant::Gemini => ("gemini", vec!["--acp"]),
            Assistant::Registered {
                agent,
                distribution,
                configuration,
                command,
                args,
                registry,
            } => {
                let registry = registry.as_ref().ok_or_else(|| {
                    Error::Internal(
                        "Agent registry not available for registered assistant".to_string(),
                    )
                })?;
                let Assistant::CustomStdio {
                    command: registry_command,
                    args: registry_args,
                    ..
                } = fetch_agent_from_registry(agent, *distribution, configuration, registry)
                    .await?
                else {
                    return Err(Error::Internal(
                        "agent registry should only return CustomStdio".to_string(),
                    ));
                };
                owned_command = command.clone().unwrap_or(registry_command);
                owned_args = args.clone().unwrap_or(registry_args);
                (
                    owned_command.as_str(),
                    owned_args.iter().map(|s| s.as_str()).collect(),
                )
            }
            Assistant::CustomStdio { command, args, .. } => {
                (command.as_str(), args.iter().map(|s| s.as_str()).collect())
            }
            Assistant::CustomUrl { .. } => {
                return Err(Error::Connection(
                    "CustomUrl assistants do not use stdio connections".to_string(),
                ));
            }
            Assistant::CustomSocket { .. } => {
                return Err(Error::Connection(
                    "Socket assistants do not use stdio connections".to_string(),
                ));
            }
        };
        let mut cmd = async_process::Command::new(program);
        cmd.args(args);
        Ok(cmd)
    }

    #[instrument(level = "trace", skip(self))]
    pub fn name(&self) -> String {
        match self {
            Assistant::CustomStdio { name, .. } => name.clone(),
            Assistant::CustomUrl { name, .. } => name.clone(),
            Assistant::CustomSocket { name, .. } => name.clone(),
            assistant => assistant.to_string(),
        }
    }
}

impl From<&str> for Assistant {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "copilot" => Assistant::Copilot,
            "opencode" => Assistant::Opencode,
            _ => Assistant::CustomStdio {
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

pub struct ConnectionManager {
    connection: HashMap<String, Connection>,
    state: Arc<Mutex<PluginState>>,
}

impl ConnectionManager {
    #[instrument(level = "trace")]
    pub fn new(state: Arc<Mutex<PluginState>>) -> Self {
        Self {
            connection: HashMap::new(),
            state,
        }
    }

    #[instrument(level = "trace", skip(self))]
    async fn set_agent(&self, agent: Assistant) {
        let mut config = self.state.lock().await;
        config.set_agent(agent);
        drop(config);
    }

    #[instrument(level = "trace", skip(self))]
    async fn get_agent(&self) -> Assistant {
        let config = self.state.lock().await;
        let agent = config.agent_info.current.clone();
        drop(config);
        agent
    }

    #[instrument(level = "trace", skip(self, connection))]
    pub(crate) fn add_connection(&mut self, agent: Assistant, connection: Connection) {
        self.connection.insert(agent.name(), connection);
    }

    #[instrument(level = "trace", skip(self))]
    pub fn get_connection(&self, agent: &Assistant) -> Option<&Connection> {
        self.connection.get(&agent.name())
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn get_current_connection(&self) -> Option<&Connection> {
        self.get_connection(&self.get_agent().await)
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn get_permissions(&self) -> Permissions {
        let config = self.state.lock().await;
        let permissions = config.config.permissions.clone();
        drop(config);
        permissions
    }

    #[instrument(level = "trace", skip(self, handler))]
    pub async fn connect(
        &mut self,
        handler: Arc<Handler>,
        ConnectionDetails { agent, protocol }: ConnectionDetails,
    ) -> crate::acp::Result<&Connection> {
        let permissions = self.get_permissions().await;
        let agent_name = agent.name();

        // Check if connection already exists without borrowing
        let already_connected = self.connection.contains_key(&agent_name);
        if already_connected {
            warn!(
                "A connection already exists for '{}'. Returning existing connection",
                agent
            );
            return self
                .connection
                .get(&agent_name)
                .ok_or_else(|| Error::Internal("Connection not found".to_string()));
        }

        // Now we can safely do mutable operations
        let (sender, receiver) = async_channel::bounded(100);
        let init_config = InitializeRequest::new(ProtocolVersion::LATEST)
            .client_info(Implementation::new("hermes", env!("CARGO_PKG_VERSION")).title("Hermes"))
            .client_capabilities(
                ClientCapabilities::new()
                    .terminal(permissions.terminal_access)
                    .fs(FileSystemCapabilities::new()
                        .read_text_file(permissions.fs_read_access)
                        .write_text_file(permissions.fs_write_access))
                    .elicitation(
                        ElicitationCapabilities::new()
                            .form(
                                permissions
                                    .elicitation
                                    .form
                                    .then(ElicitationFormCapabilities::new),
                            )
                            .url(
                                permissions
                                    .elicitation
                                    .url
                                    .then(ElicitationUrlCapabilities::new),
                            ),
                    ),
            );

        let thread_agent = agent.clone();
        trace!("Starting agent communication in new thread");

        let child = if protocol == Protocol::Stdio {
            Some(Arc::new(stdio::child::Child::new()))
        } else {
            None
        };
        let stdio_child = child.clone();

        let handle = std::thread::spawn(move || {
            let executor = std::rc::Rc::new(smol::LocalExecutor::new());
            let agent_display_name = thread_agent.to_string();

            trace!("Starting smol executor for {}", agent_display_name);

            // Run the connection in the executor.
            // smol::block_on drives the top-level future, while executor.run()
            // continuously polls all tasks spawned onto the LocalExecutor.
            // Each protocol module owns its transport-specific orchestration
            // (stream acquisition, post-disconnect cleanup) and delegates the
            // shared ACP `Client.builder()` plumbing to `connect::run_connection`.
            let run_result = smol::block_on(executor.run(async {
                match protocol {
                    Protocol::Stdio => {
                        stdio::connect(handler, thread_agent, receiver, child.unwrap()).await
                    }
                    Protocol::Tcp => tcp::connect(handler, thread_agent, receiver).await,
                    Protocol::Http | Protocol::Https => {
                        http::connect(protocol, handler, thread_agent, receiver).await
                    }
                    Protocol::Socket => socket::connect(handler, thread_agent, receiver).await,
                }
            }));

            match &run_result {
                Ok(()) => info!("Agent thread for '{}' exited normally", agent_display_name),
                Err(e) => error!(
                    "Agent thread for '{}' exited with error: {:?}",
                    agent_display_name, e
                ),
            }

            run_result
        });

        self.add_connection(agent.clone(), Connection::new(sender, handle, stdio_child));
        self.set_agent(agent.clone()).await;
        let connection = self.get_connection(&agent).unwrap();
        debug!("Stored connection to '{}'", agent);
        connection.initialize(init_config).await?;
        info!("Initialized connection to '{}'", agent);
        Ok(connection)
    }

    #[instrument(level = "trace", skip(self))]
    pub fn connected_agents(&self) -> Vec<Assistant> {
        self.connection
            .keys()
            .map(|key| Assistant::from(key.as_str()))
            .collect()
    }

    #[instrument(level = "trace", skip(self))]
    pub fn close_all(&mut self) -> Result<(), Error> {
        self.disconnect(self.connected_agents())?;
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
        let connection = self.connection.remove(&assistant.name()).ok_or_else(|| {
            Error::Connection(format!("No connection found for assistant {}", assistant))
        })?;
        drop(connection);
        Ok(())
    }
}

impl Drop for ConnectionManager {
    fn drop(&mut self) {
        debug!("ConnectionManager Drop called - initiating cleanup");
        match self.close_all() {
            Ok(_) => debug!("ConnectionManager cleanup completed successfully"),
            Err(e) => error!("ConnectionManager cleanup failed: {:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::registry::{DistributionCommand, PackageDistribution};
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
        let protocols: Vec<Protocol> = vec![
            Protocol::Tcp,
            Protocol::Socket,
            Protocol::Http,
            Protocol::Https,
            Protocol::Stdio,
        ];
        let results: Vec<String> = protocols.iter().map(|p| format!("{}", p)).collect();

        let expected: Vec<String> = vec![
            "tcp".to_string(),
            "socket".to_string(),
            "http".to_string(),
            "https".to_string(),
            "stdio".to_string(),
        ];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_protocol_from_str() {
        // Test FromStr for known protocols using slice comparison
        let inputs: Vec<&str> = vec![
            "tcp", "socket", "http", "https", "stdio", "TCP", "SOCKET", "HTTP", "HTTPS", "STDIO",
            "unknown",
        ];
        let results: Vec<Protocol> = inputs.iter().map(|&s| Protocol::from(s)).collect();

        let expected: Vec<Protocol> = vec![
            Protocol::Tcp,    // tcp
            Protocol::Socket, // socket
            Protocol::Http,   // http
            Protocol::Https,  // https
            Protocol::Stdio,  // stdio
            Protocol::Tcp,    // TCP (case-insensitive)
            Protocol::Socket, // SOCKET (case-insensitive)
            Protocol::Http,   // HTTP (case-insensitive)
            Protocol::Https,  // HTTPS (case-insensitive)
            Protocol::Stdio,  // STDIO (case-insensitive)
            Protocol::Stdio,  // unknown (defaults to Stdio)
        ];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_assistant_display() {
        // Test Display for all Assistant variants using slice comparison
        let assistants: Vec<Assistant> = vec![
            Assistant::Copilot,
            Assistant::Opencode,
            Assistant::CustomStdio {
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
    fn connected_agents_returns_empty_when_no_connections() {
        let manager = ConnectionManager::new(Arc::new(Mutex::new(PluginState::new())));
        let agents = manager.connected_agents();
        assert!(agents.is_empty());
    }

    #[test]
    fn connected_agents_returns_single_agent() {
        let mut manager = ConnectionManager::new(Arc::new(Mutex::new(PluginState::new())));
        let (sender, _) = async_channel::unbounded();
        let handle = std::thread::spawn(|| Ok(()));
        let connection = Connection::new(sender, handle, None);
        manager.add_connection(Assistant::Copilot, connection);
        let agents = manager.connected_agents();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], Assistant::Copilot);
    }

    #[test]
    fn connected_agents_returns_multiple_agents() {
        let mut manager = ConnectionManager::new(Arc::new(Mutex::new(PluginState::new())));
        let (sender1, _) = async_channel::unbounded();
        let handle1 = std::thread::spawn(|| Ok(()));
        let connection1 = Connection::new(sender1, handle1, None);
        manager.add_connection(Assistant::Copilot, connection1);

        let (sender2, _) = async_channel::unbounded();
        let handle2 = std::thread::spawn(|| Ok(()));
        let connection2 = Connection::new(sender2, handle2, None);
        manager.add_connection(Assistant::Opencode, connection2);

        let agents = manager.connected_agents();
        assert_eq!(agents.len(), 2);
        assert!(agents.contains(&Assistant::Copilot));
        assert!(agents.contains(&Assistant::Opencode));
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
    fn test_assistant_custom_url_display() {
        let assistant = Assistant::CustomUrl {
            name: String::from("my-agent"),
            host: String::from("localhost"),
            port: 8080,
            path: None,
        };
        assert_eq!(format!("{}", assistant), "my-agent (localhost:8080)");
    }

    #[test]
    fn test_assistant_custom_url_display_with_path() {
        let assistant = Assistant::CustomUrl {
            name: String::from("my-agent"),
            host: String::from("localhost"),
            port: 8080,
            path: Some(String::from("/acp")),
        };
        assert_eq!(format!("{}", assistant), "my-agent (localhost:8080/acp)");
    }

    #[test]
    fn test_assistant_custom_stdio_display() {
        let assistant = Assistant::CustomStdio {
            name: String::from("custom-cli"),
            command: String::from("my-cmd"),
            args: vec![String::from("--arg1"), String::from("--arg2")],
        };
        assert_eq!(format!("{}", assistant), "custom-cli");
    }

    #[test]
    fn test_assistant_gemini_display() {
        let assistant = Assistant::Gemini;
        assert_eq!(format!("{}", assistant), "gemini");
    }

    #[test]
    fn test_assistant_registered_display_with_distribution() {
        let entry = AgentEntry {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::from([(
                Distribution::Npx,
                DistributionCommand::Package(PackageDistribution {
                    package: "test-agent".to_string(),
                    args: None,
                    env: None,
                }),
            )]),
        };
        let assistant = Assistant::Registered {
            agent: entry,
            configuration: Default::default(),
            distribution: Some(Distribution::Npx),
            command: None,
            args: None,
            registry: None,
        };
        assert_eq!(format!("{}", assistant), "test-agent (npx)");
    }

    #[test]
    fn test_assistant_registered_display_without_distribution() {
        let entry = AgentEntry {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::new(),
        };
        let assistant = Assistant::Registered {
            agent: entry,
            configuration: Default::default(),
            distribution: None,
            command: None,
            args: None,
            registry: None,
        };
        assert_eq!(format!("{}", assistant), "test-agent");
    }

    #[test]
    fn test_assistant_from_str_gemini_lowercase() {
        // Note: Gemini is not currently handled in From<&str>, it becomes CustomStdio
        // This test documents current behavior
        let result = Assistant::from("gemini");
        assert!(matches!(result, Assistant::CustomStdio { .. }));
    }

    #[test]
    fn test_assistant_from_str_custom_name() {
        let result = Assistant::from("my-custom-agent");
        match result {
            Assistant::CustomStdio {
                name,
                command,
                args,
            } => {
                assert_eq!(name, "my-custom-agent");
                assert!(command.is_empty());
                assert!(args.is_empty());
            }
            _ => panic!("Expected CustomStdio variant"),
        }
    }

    #[test]
    fn test_assistant_from_string() {
        let result = Assistant::from(String::from("opencode"));
        assert!(matches!(result, Assistant::Opencode));
    }

    #[test]
    fn test_protocol_default_is_stdio() {
        let protocol: Protocol = Default::default();
        assert!(matches!(protocol, Protocol::Stdio));
    }
}
