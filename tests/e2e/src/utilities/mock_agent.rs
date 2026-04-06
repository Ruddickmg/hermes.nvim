#![allow(private_interfaces)]

use agent_client_protocol::{
    Agent, AgentCapabilities, AgentSideConnection, AuthenticateRequest, AuthenticateResponse,
    CancelNotification, Client, ContentBlock, ContentChunk, CreateTerminalRequest,
    CreateTerminalResponse, ExtNotification, ExtRequest, ExtResponse, Implementation,
    InitializeRequest, InitializeResponse, ListSessionsRequest, ListSessionsResponse,
    LoadSessionRequest, LoadSessionResponse, McpCapabilities, NewSessionRequest,
    NewSessionResponse, PromptCapabilities, PromptRequest, PromptResponse, ProtocolVersion,
    ReadTextFileRequest, ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionOutcome, RequestPermissionRequest, SessionCapabilities,
    SessionForkCapabilities, SessionListCapabilities, SessionNotification,
    SessionResumeCapabilities, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse, StopReason,
    TerminalOutputRequest, TerminalOutputResponse, TextContent, WaitForTerminalExitRequest,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{error, info};

use super::mock_agent_handle::MockAgentHandle;
use super::mock_config::{MockConfig, default_ext_response, generate_session_id};

/// Internal error code for mock agent errors (JSON-RPC internal error)
const INTERNAL_ERROR_CODE: i32 = -32603;

/// Create an internal error with a message
fn internal_error(message: impl Into<String>) -> agent_client_protocol::Error {
    agent_client_protocol::Error::new(INTERNAL_ERROR_CODE, message)
}

/// Messages sent from Agent trait methods to the connection handler task.
/// This is an internal detail - callers interact via `MockAgentReceiver`.
pub(crate) enum AgentToConnection {
    /// Send a session notification to Hermes
    SessionNotification(SessionNotification, oneshot::Sender<()>),
    /// Send a permission request to Hermes and return the outcome
    PermissionRequest(
        RequestPermissionRequest,
        oneshot::Sender<RequestPermissionOutcome>,
    ),
    /// Send a terminal creation request to Hermes and return the response
    CreateTerminal(
        CreateTerminalRequest,
        oneshot::Sender<agent_client_protocol::Result<CreateTerminalResponse>>,
    ),
    /// Send a terminal output request to Hermes and return the response
    TerminalOutput(
        TerminalOutputRequest,
        oneshot::Sender<agent_client_protocol::Result<TerminalOutputResponse>>,
    ),
    /// Send a wait for terminal exit request to Hermes and return the response
    WaitForTerminalExit(
        WaitForTerminalExitRequest,
        oneshot::Sender<agent_client_protocol::Result<WaitForTerminalExitResponse>>,
    ),
    /// Send a read text file request to Hermes and return the response
    ReadTextFile(
        ReadTextFileRequest,
        oneshot::Sender<agent_client_protocol::Result<ReadTextFileResponse>>,
    ),
    /// Send a write text file request to Hermes and return the response
    WriteTextFile(
        WriteTextFileRequest,
        oneshot::Sender<agent_client_protocol::Result<WriteTextFileResponse>>,
    ),
    /// Send a release terminal request to Hermes and return the response
    ReleaseTerminal(
        ReleaseTerminalRequest,
        oneshot::Sender<agent_client_protocol::Result<ReleaseTerminalResponse>>,
    ),
}

/// Opaque receiver type passed from `MockAgent::new()` to `MockAgent::start()`.
pub type MockAgentReceiver = mpsc::UnboundedReceiver<AgentToConnection>;

/// Mock agent implementing the ACP Agent trait
pub struct MockAgent {
    config: Arc<Mutex<MockConfig>>,
    /// Channel to send messages to the connection handler task
    conn_tx: mpsc::UnboundedSender<AgentToConnection>,
}

impl MockAgent {
    /// Create a new mock agent with default configuration.
    ///
    /// Returns the agent and the receiver end of the connection channel.
    pub fn new() -> (Self, MockAgentReceiver) {
        let (conn_tx, conn_rx) = mpsc::unbounded_channel();
        let agent = Self {
            config: Arc::new(Mutex::new(MockConfig::default())),
            conn_tx,
        };
        (agent, conn_rx)
    }

    /// Get access to the configuration for customization
    pub fn config(&self) -> &Arc<Mutex<MockConfig>> {
        &self.config
    }

    /// Start the mock agent on a random available port.
    ///
    /// Spawns a thread with a Tokio LocalSet runtime that:
    /// 1. Accepts one TCP connection
    /// 2. Sets up an AgentSideConnection
    /// 3. Spawns a task to handle messages from Agent trait methods
    /// 4. Runs the I/O handler until the connection closes
    pub fn start(
        agent: MockAgent,
        conn_rx: MockAgentReceiver,
    ) -> Result<MockAgentHandle, std::io::Error> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();

        info!("Mock agent starting on port {}", port);

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        let config_clone = agent.config.clone();

        let thread_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime");

            rt.block_on(async {
                let local_set = tokio::task::LocalSet::new();

                local_set
                    .run_until(async {
                        listener
                            .set_nonblocking(true)
                            .expect("Failed to set nonblocking");
                        let tokio_listener = TcpListener::from_std(listener)
                            .expect("Failed to convert to tokio listener");

                        // Wait for either a connection or shutdown signal
                        let stream = tokio::select! {
                            result = tokio_listener.accept() => {
                                match result {
                                    Ok((stream, addr)) => {
                                        info!("Mock agent accepted connection from {}", addr);
                                        stream
                                    }
                                    Err(e) => {
                                        error!("Failed to accept connection: {}", e);
                                        return;
                                    }
                                }
                            }
                            _ = &mut shutdown_rx => {
                                info!("Mock agent shutdown before connection");
                                return;
                            }
                        };

                        let (read_half, write_half) = tokio::io::split(stream);
                        let outgoing = write_half.compat_write();
                        let incoming = read_half.compat();

                        // Create AgentSideConnection (implements both Agent and Client traits)
                        let (conn, handle_io) =
                            AgentSideConnection::new(agent, outgoing, incoming, |fut| {
                                tokio::task::spawn_local(fut);
                            });

                        // Spawn task to handle messages from Agent trait methods.
                        let mut conn_rx = conn_rx;
                        tokio::task::spawn_local(async move {
                            while let Some(msg) = conn_rx.recv().await {
                                match msg {
                                    AgentToConnection::SessionNotification(notification, tx) => {
                                        let result = conn.session_notification(notification).await;
                                        if let Err(e) = result {
                                            error!("Error sending session notification: {}", e);
                                            break;
                                        }
                                        tx.send(()).ok();
                                    }
                                    AgentToConnection::PermissionRequest(request, tx) => {
                                        let result = conn.request_permission(request).await;
                                        match result {
                                            Ok(response) => {
                                                tx.send(response.outcome).ok();
                                            }
                                            Err(e) => {
                                                error!("Error sending permission request: {}", e);
                                                tx.send(RequestPermissionOutcome::Cancelled).ok();
                                            }
                                        }
                                    }
                                    AgentToConnection::CreateTerminal(request, tx) => {
                                        let result = conn.create_terminal(request).await;
                                        tx.send(result).ok();
                                    }
                                    AgentToConnection::TerminalOutput(request, tx) => {
                                        let result = conn.terminal_output(request).await;
                                        tx.send(result).ok();
                                    }
                                    AgentToConnection::WaitForTerminalExit(request, tx) => {
                                        let result = conn.wait_for_terminal_exit(request).await;
                                        tx.send(result).ok();
                                    }
                                    AgentToConnection::ReadTextFile(request, tx) => {
                                        let result = conn.read_text_file(request).await;
                                        tx.send(result).ok();
                                    }
                                    AgentToConnection::WriteTextFile(request, tx) => {
                                        let result = conn.write_text_file(request).await;
                                        tx.send(result).ok();
                                    }
                                    AgentToConnection::ReleaseTerminal(request, tx) => {
                                        let result = conn.release_terminal(request).await;
                                        tx.send(result).ok();
                                    }
                                }
                            }
                        });

                        // Run I/O handler until connection closes OR shutdown signal
                        tokio::select! {
                            result = handle_io => {
                                if let Err(e) = result {
                                    error!("Mock agent I/O error: {}", e);
                                }
                            }
                            _ = shutdown_rx => {
                                info!("Mock agent received shutdown signal");
                            }
                        }
                    })
                    .await;
            });
        });

        Ok(MockAgentHandle::new(
            config_clone,
            port,
            thread_handle,
            shutdown_tx,
        ))
    }
}

#[async_trait(?Send)]
impl Agent for MockAgent {
    async fn initialize(
        &self,
        _request: InitializeRequest,
    ) -> agent_client_protocol::Result<InitializeResponse> {
        let timeout = self.config.lock().unwrap().timeout;
        tokio::time::timeout(timeout, async {
            Ok(InitializeResponse::new(ProtocolVersion::V1)
                .agent_info(Implementation::new("mock-agent", "0.1.0"))
                .agent_capabilities(
                    AgentCapabilities::new()
                        .load_session(true)
                        .prompt_capabilities(
                            PromptCapabilities::new()
                                .image(true)
                                .audio(true)
                                .embedded_context(true),
                        )
                        .mcp_capabilities(McpCapabilities::new().http(true).sse(true))
                        .session_capabilities(
                            SessionCapabilities::new()
                                .list(Some(SessionListCapabilities::new()))
                                .fork(Some(SessionForkCapabilities::new()))
                                .resume(Some(SessionResumeCapabilities::new())),
                        ),
                ))
        })
        .await
        .map_err(|_| internal_error("initialize timed out"))?
    }

    async fn authenticate(
        &self,
        _request: AuthenticateRequest,
    ) -> agent_client_protocol::Result<AuthenticateResponse> {
        let timeout = self.config.lock().unwrap().timeout;
        tokio::time::timeout(timeout, async {
            let config = self.config.lock().unwrap();
            Ok(config.authenticate_response.clone())
        })
        .await
        .map_err(|_| internal_error("authenticate timed out"))?
    }

    async fn new_session(
        &self,
        request: NewSessionRequest,
    ) -> agent_client_protocol::Result<NewSessionResponse> {
        let timeout = self.config.lock().unwrap().timeout;
        tokio::time::timeout(timeout, async {
            let mut config = self.config.lock().unwrap();
            let response = config.new_session_response.clone();

            // Track the session for default load_session/list_sessions behavior
            config.track_session(response.session_id.clone(), request.cwd.clone());

            // Generate a fresh session ID for the next new_session call
            config.new_session_response = NewSessionResponse::new(generate_session_id());

            Ok(response)
        })
        .await
        .map_err(|_| internal_error("new_session timed out"))?
    }

    async fn prompt(
        &self,
        request: PromptRequest,
    ) -> agent_client_protocol::Result<PromptResponse> {
        let timeout = self.config.lock().unwrap().timeout;
        tokio::time::timeout(timeout, async {
            // Check if we should request permission
            let permission_request = {
                let config = self.config.lock().unwrap();
                config.permission_request.clone()
            };

            // If configured, send a permission request to Hermes and wait for the response
            if let Some(perm_req) = permission_request {
                let (tx, rx) = oneshot::channel();
                self.conn_tx
                    .send(AgentToConnection::PermissionRequest(perm_req, tx))
                    .map_err(|_| internal_error("failed to send permission request"))?;

                let inner_timeout = self.config.lock().unwrap().timeout;
                tokio::time::timeout(inner_timeout, rx)
                    .await
                    .map_err(|_| internal_error("permission request timed out"))?
                    .map_err(|_| internal_error("permission request channel closed"))?;
            }

            // Check if terminal workflow is configured
            let (create_terminal, send_terminal_output, send_terminal_exit) = {
                let config = self.config.lock().unwrap();
                (
                    config.create_terminal_request.clone(),
                    config.terminal_output_request.is_some(),
                    config.wait_for_terminal_exit_request.is_some(),
                )
            };

            // If configured, execute terminal workflow
            if let Some(create_req) = create_terminal {
                // Step 1: Create terminal - wait for response with terminal_id
                let (tx, rx) = oneshot::channel();
                self.conn_tx
                    .send(AgentToConnection::CreateTerminal(create_req, tx))
                    .map_err(|_| internal_error("failed to send create_terminal request"))?;

                let create_response = tokio::time::timeout(timeout, rx)
                    .await
                    .map_err(|_| internal_error("create_terminal timed out"))?
                    .map_err(|_| internal_error("create_terminal channel closed"))?
                    .map_err(|e| internal_error(format!("create_terminal failed: {}", e)))?;

                let terminal_id = create_response.terminal_id;

                // Step 2: Get terminal output (if configured)
                if send_terminal_output {
                    let output_req =
                        TerminalOutputRequest::new(request.session_id.clone(), terminal_id.clone());
                    let (tx, rx) = oneshot::channel();
                    self.conn_tx
                        .send(AgentToConnection::TerminalOutput(output_req, tx))
                        .map_err(|_| internal_error("failed to send terminal_output request"))?;

                    tokio::time::timeout(timeout, rx)
                        .await
                        .map_err(|_| internal_error("terminal_output timed out"))?
                        .map_err(|_| internal_error("terminal_output channel closed"))?
                        .map_err(|e| internal_error(format!("terminal_output failed: {}", e)))?;
                }

                // Step 3: Wait for terminal exit (if configured)
                if send_terminal_exit {
                    let exit_req =
                        WaitForTerminalExitRequest::new(request.session_id.clone(), terminal_id);
                    let (tx, rx) = oneshot::channel();
                    self.conn_tx
                        .send(AgentToConnection::WaitForTerminalExit(exit_req, tx))
                        .map_err(|_| {
                            internal_error("failed to send wait_for_terminal_exit request")
                        })?;

                    tokio::time::timeout(timeout, rx)
                        .await
                        .map_err(|_| internal_error("wait_for_terminal_exit timed out"))?
                        .map_err(|_| internal_error("wait_for_terminal_exit channel closed"))?
                        .map_err(|e| {
                            internal_error(format!("wait_for_terminal_exit failed: {}", e))
                        })?;
                }
            }

            // Read text file (if configured)
            let read_file_request = {
                let config = self.config.lock().unwrap();
                config.read_file_request.clone()
            };

            if let Some(read_req) = read_file_request {
                let (tx, rx) = oneshot::channel();
                self.conn_tx
                    .send(AgentToConnection::ReadTextFile(read_req, tx))
                    .map_err(|_| internal_error("failed to send read_text_file request"))?;

                tokio::time::timeout(timeout, rx)
                    .await
                    .map_err(|_| internal_error("read_text_file timed out"))?
                    .map_err(|_| internal_error("read_text_file channel closed"))?
                    .map_err(|e| internal_error(format!("read_text_file failed: {}", e)))?;
            }

            // Write text file (if configured)
            let write_file_request = {
                let config = self.config.lock().unwrap();
                config.write_file_request.clone()
            };

            if let Some(write_req) = write_file_request {
                let (tx, rx) = oneshot::channel();
                self.conn_tx
                    .send(AgentToConnection::WriteTextFile(write_req, tx))
                    .map_err(|_| internal_error("failed to send write_text_file request"))?;

                tokio::time::timeout(timeout, rx)
                    .await
                    .map_err(|_| internal_error("write_text_file timed out"))?
                    .map_err(|_| internal_error("write_text_file channel closed"))?
                    .map_err(|e| internal_error(format!("write_text_file failed: {}", e)))?;
            }

            // Release terminal (if configured)
            let release_terminal_request = {
                let config = self.config.lock().unwrap();
                config.release_terminal_request.clone()
            };

            if let Some(release_req) = release_terminal_request {
                let (tx, rx) = oneshot::channel();
                self.conn_tx
                    .send(AgentToConnection::ReleaseTerminal(release_req, tx))
                    .map_err(|_| internal_error("failed to send release_terminal request"))?;

                tokio::time::timeout(timeout, rx)
                    .await
                    .map_err(|_| internal_error("release_terminal timed out"))?
                    .map_err(|_| internal_error("release_terminal channel closed"))?
                    .map_err(|e| internal_error(format!("release_terminal failed: {}", e)))?;
            }

            // Echo back the prompt content as agent message chunks
            for content in &request.prompt {
                let text = match content {
                    ContentBlock::Text(text_content) => text_content.text.clone(),
                    _ => format!("{:?}", content),
                };

                let notification = SessionNotification::new(
                    request.session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                        TextContent::new(text),
                    ))),
                );

                let (tx, rx) = oneshot::channel();
                if self
                    .conn_tx
                    .send(AgentToConnection::SessionNotification(notification, tx))
                    .is_err()
                {
                    break;
                }
                let _ = rx.await;
            }

            Ok(PromptResponse::new(StopReason::EndTurn))
        })
        .await
        .map_err(|_| internal_error("prompt timed out"))?
    }

    async fn cancel(&self, _notification: CancelNotification) -> agent_client_protocol::Result<()> {
        Ok(())
    }

    async fn load_session(
        &self,
        request: LoadSessionRequest,
    ) -> agent_client_protocol::Result<LoadSessionResponse> {
        let timeout = self.config.lock().unwrap().timeout;
        tokio::time::timeout(timeout, async {
            let config = self.config.lock().unwrap();

            // Return user-configured response if set
            if let Some(ref response) = config.load_session_response {
                return Ok(response.clone());
            }

            // Default: check if session is tracked (session exists)
            if config.sessions.contains_key(&request.session_id) {
                Ok(LoadSessionResponse::new())
            } else {
                Err(internal_error(format!(
                    "session not found: {}",
                    request.session_id
                )))
            }
        })
        .await
        .map_err(|_| internal_error("load_session timed out"))?
    }

    async fn set_session_mode(
        &self,
        _request: SetSessionModeRequest,
    ) -> agent_client_protocol::Result<SetSessionModeResponse> {
        let timeout = self.config.lock().unwrap().timeout;
        tokio::time::timeout(timeout, async {
            let config = self.config.lock().unwrap();
            if let Some(ref response) = config.set_session_mode_response {
                Ok(response.clone())
            } else {
                Ok(SetSessionModeResponse::new())
            }
        })
        .await
        .map_err(|_| internal_error("set_session_mode timed out"))?
    }

    async fn set_session_config_option(
        &self,
        _request: SetSessionConfigOptionRequest,
    ) -> agent_client_protocol::Result<SetSessionConfigOptionResponse> {
        let timeout = self.config.lock().unwrap().timeout;
        tokio::time::timeout(timeout, async {
            let config = self.config.lock().unwrap();
            if let Some(ref response) = config.set_session_config_option_response {
                Ok(response.clone())
            } else {
                Ok(SetSessionConfigOptionResponse::new(vec![]))
            }
        })
        .await
        .map_err(|_| internal_error("set_session_config_option timed out"))?
    }

    async fn list_sessions(
        &self,
        _request: ListSessionsRequest,
    ) -> agent_client_protocol::Result<ListSessionsResponse> {
        let timeout = self.config.lock().unwrap().timeout;
        tokio::time::timeout(timeout, async {
            let config = self.config.lock().unwrap();

            // Return user-configured response if set
            if let Some(ref response) = config.list_sessions_response {
                return Ok(response.clone());
            }

            // Default: return all tracked sessions
            let sessions: Vec<_> = config.sessions.values().cloned().collect();
            Ok(ListSessionsResponse::new(sessions))
        })
        .await
        .map_err(|_| internal_error("list_sessions timed out"))?
    }

    async fn ext_method(&self, _request: ExtRequest) -> agent_client_protocol::Result<ExtResponse> {
        let timeout = self.config.lock().unwrap().timeout;
        tokio::time::timeout(timeout, async {
            let config = self.config.lock().unwrap();
            if let Some(ref response) = config.ext_response {
                Ok(response.clone())
            } else {
                Ok(default_ext_response())
            }
        })
        .await
        .map_err(|_| internal_error("ext_method timed out"))?
    }

    async fn ext_notification(
        &self,
        _request: ExtNotification,
    ) -> agent_client_protocol::Result<()> {
        Ok(())
    }
}
