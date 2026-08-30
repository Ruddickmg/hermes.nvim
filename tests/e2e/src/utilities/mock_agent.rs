#![allow(private_interfaces)]

// MockAgent for e2e tests, rewritten for agent-client-protocol 0.11.
//
// In 0.10.x this was an `impl Agent for MockAgent` block driven by
// `AgentSideConnection::new(...)`. In 0.11 there is no `Agent` trait; handlers
// are closures registered on `Agent.builder()`. The prompt handler runs inside
// `cx.spawn(...)` and calls `cx.send_request(...).block_task().await` directly
// on the cloned `ConnectionTo<Client>`, eliminating the AgentToConnection channel
// indirection that was carried over from 0.10.x.

use agent_client_protocol::{
    self as acp, Agent, ByteStreams, ConnectionTo, Lines, Responder, on_receive_notification,
    on_receive_request,
    schema::v1::{
        AuthenticateRequest, AuthenticateResponse, CancelNotification, CloseSessionRequest,
        CloseSessionResponse, ContentBlock, ContentChunk, DeleteSessionRequest,
        DeleteSessionResponse, InitializeRequest, InitializeResponse, ListSessionsRequest,
        ListSessionsResponse, LoadSessionRequest, LoadSessionResponse, LogoutRequest,
        LogoutResponse, NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse,
        ResumeSessionRequest, ResumeSessionResponse, SessionNotification, SessionUpdate,
        SetSessionConfigOptionRequest, SetSessionConfigOptionResponse, SetSessionModeRequest,
        SetSessionModeResponse, StopReason, TerminalOutputRequest, TextContent,
        WaitForTerminalExitRequest,
    },
};
use async_channel::bounded;
use async_io::{Async, Timer};
use futures::StreamExt;
use futures::future::{Either, select};
use futures::io::AsyncReadExt;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

use super::mock_agent_handle::MockAgentHandle;
use super::mock_config::{MockConfig, generate_session_id};

/// Internal error code for mock agent errors (JSON-RPC internal error)
const INTERNAL_ERROR_CODE: i32 = -32603;

/// Create an internal error with a message
fn internal_error(message: impl Into<String>) -> acp::Error {
    acp::Error::new(INTERNAL_ERROR_CODE, message)
}

/// Mock agent state shared with the builder closures.
pub struct MockAgent {
    config: Arc<Mutex<MockConfig>>,
}

impl MockAgent {
    /// Create a new mock agent with default configuration.
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(MockConfig::default())),
        }
    }

    /// Get access to the configuration for customization
    pub fn config(&self) -> &Arc<Mutex<MockConfig>> {
        &self.config
    }

    /// Start the mock agent on a random available port.
    ///
    /// Spawns a thread with a smol LocalExecutor that:
    /// 1. Accepts one TCP connection
    /// 2. Builds an `Agent.builder()` with handlers that delegate to MockConfig
    /// 3. Drives the connection until the transport closes or shutdown is signaled
    pub fn start(agent: MockAgent) -> Result<MockAgentHandle, std::io::Error> {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = std_listener.local_addr()?.port();

        info!("Mock agent starting on port {}", port);

        let (shutdown_tx, shutdown_rx) = bounded::<()>(1);
        let config_clone = agent.config.clone();
        let MockAgent { config } = agent;

        let thread_handle = std::thread::spawn(move || {
            let executor = Rc::new(smol::LocalExecutor::new());

            let listener = match Async::new(std_listener) {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to create async listener: {}", e);
                    return;
                }
            };

            smol::block_on(executor.clone().run(async move {
                let accept_fut = async {
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            info!("Mock agent accepted connection from {}", addr);
                            Some(stream)
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                            None
                        }
                    }
                };

                let shutdown_fut = async {
                    let _ = shutdown_rx.recv().await;
                    info!("Mock agent received shutdown signal before connection");
                };

                match select(Box::pin(accept_fut), Box::pin(shutdown_fut)).await {
                    Either::Left((Some(stream), _)) => {
                        let (read_half, write_half) = stream.split();

                        let builder = build_mock_agent_builder(config.clone());

                        let serve_fut = async move {
                            let result = builder
                                .connect_to(ByteStreams::new(write_half, read_half))
                                .await;
                            if let Err(e) = result {
                                error!("Mock agent connection error: {}", e);
                            }
                            info!("Mock agent connection completed");
                        };

                        let shutdown_wait = async {
                            let _ = shutdown_rx.recv().await;
                            info!("Shutdown received while serving connection");
                        };

                        let _ = select(Box::pin(serve_fut), Box::pin(shutdown_wait)).await;
                    }
                    Either::Left((None, _)) => {
                        info!("Mock agent accept failed, exiting");
                    }
                    Either::Right((_, _)) => {
                        info!("Mock agent shutting down before connection established");
                    }
                }

                info!("Mock agent main task completed");
            }));

            info!("Mock agent thread exiting");
        });

        Ok(MockAgentHandle::new(
            config_clone,
            port,
            thread_handle,
            shutdown_tx,
        ))
    }

    /// Start the mock agent on a random available port, accepting WebSocket connections.
    ///
    /// Identical to `start` but performs a WebSocket upgrade on the accepted TCP
    /// connection and drives the ACP protocol over WebSocket text frames.
    pub fn start_websocket(agent: MockAgent) -> Result<MockAgentHandle, std::io::Error> {
        let std_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = std_listener.local_addr()?.port();

        info!("Mock agent (websocket) starting on port {}", port);

        let (shutdown_tx, shutdown_rx) = bounded::<()>(1);
        let config_clone = agent.config.clone();
        let MockAgent { config } = agent;

        let thread_handle = std::thread::spawn(move || {
            let executor = Rc::new(smol::LocalExecutor::new());

            let listener = match Async::new(std_listener) {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to create async listener: {}", e);
                    return;
                }
            };

            smol::block_on(executor.clone().run(async move {
                let accept_fut = async {
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            info!("Mock agent accepted connection from {}", addr);
                            Some(stream)
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                            None
                        }
                    }
                };

                let shutdown_fut = async {
                    let _ = shutdown_rx.recv().await;
                    info!("Mock agent received shutdown signal before connection");
                };

                match select(Box::pin(accept_fut), Box::pin(shutdown_fut)).await {
                    Either::Left((Some(stream), _)) => {
                        let ws_stream = match async_tungstenite::accept_async(stream).await {
                            Ok(ws) => ws,
                            Err(e) => {
                                error!("WebSocket accept failed: {}", e);
                                return;
                            }
                        };

                        let (ws_sender, ws_receiver) = ws_stream.split();

                        let outgoing_sink = futures::sink::unfold(
                            ws_sender,
                            |mut sender, text: String| async move {
                                sender
                                    .send(async_tungstenite::tungstenite::Message::Text(
                                        text.into(),
                                    ))
                                    .await
                                    .map_err(|e| {
                                        std::io::Error::new(std::io::ErrorKind::Other, e)
                                    })?;
                                Ok::<_, std::io::Error>(sender)
                            },
                        );

                        let incoming_stream = ws_receiver.map(|msg| match msg {
                            Ok(async_tungstenite::tungstenite::Message::Text(text)) => {
                                Ok(text.to_string())
                            }
                            Ok(async_tungstenite::tungstenite::Message::Close(_)) => {
                                Err(std::io::Error::new(
                                    std::io::ErrorKind::ConnectionAborted,
                                    "WebSocket closed",
                                ))
                            }
                            Ok(other) => {
                                tracing::debug!("Received non-text WebSocket message: {:?}", other);
                                Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("Unexpected WebSocket message: {:?}", other),
                                ))
                            }
                            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
                        });

                        let lines = Lines::new(outgoing_sink, incoming_stream);

                        let builder = build_mock_agent_builder(config.clone());

                        let serve_fut = async move {
                            let result = builder.connect_to(lines).await;
                            if let Err(e) = result {
                                error!("Mock agent connection error: {}", e);
                            }
                            info!("Mock agent connection completed");
                        };

                        let shutdown_wait = async {
                            let _ = shutdown_rx.recv().await;
                            info!("Shutdown received while serving connection");
                        };

                        let _ = select(Box::pin(serve_fut), Box::pin(shutdown_wait)).await;
                    }
                    Either::Left((None, _)) => {
                        info!("Mock agent accept failed, exiting");
                    }
                    Either::Right((_, _)) => {
                        info!("Mock agent shutting down before connection established");
                    }
                }

                info!("Mock agent main task completed");
            }));

            info!("Mock agent thread exiting");
        });

        Ok(MockAgentHandle::new(
            config_clone,
            port,
            thread_handle,
            shutdown_tx,
        ))
    }

    /// Start the mock agent on a Unix domain socket.
    ///
    /// Identical to `start` but listens on a Unix domain socket instead of TCP.
    #[cfg(unix)]
    pub fn start_unix_socket(agent: MockAgent) -> Result<MockAgentHandle, std::io::Error> {
        let unique = format!(
            "hermes-test-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let socket_path = std::env::temp_dir().join(unique);
        let _ = std::fs::remove_file(&socket_path);
        let std_listener = std::os::unix::net::UnixListener::bind(&socket_path)?;
        let path = socket_path.to_string_lossy().to_string();

        info!("Mock agent (unix socket) starting at {}", path);

        let (shutdown_tx, shutdown_rx) = bounded::<()>(1);
        let config_clone = agent.config.clone();
        let MockAgent { config } = agent;

        let thread_handle = std::thread::spawn(move || {
            let executor = Rc::new(smol::LocalExecutor::new());

            let listener = match Async::new(std_listener) {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to create async listener: {}", e);
                    return;
                }
            };

            smol::block_on(executor.clone().run(async move {
                let accept_fut = async {
                    match listener.accept().await {
                        Ok((stream, _)) => {
                            info!("Mock agent accepted unix socket connection");
                            Some(stream)
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                            None
                        }
                    }
                };

                let shutdown_fut = async {
                    let _ = shutdown_rx.recv().await;
                    info!("Mock agent received shutdown signal before connection");
                };

                match select(Box::pin(accept_fut), Box::pin(shutdown_fut)).await {
                    Either::Left((Some(stream), _)) => {
                        let (read_half, write_half) = stream.split();

                        let builder = build_mock_agent_builder(config.clone());

                        let serve_fut = async move {
                            let result = builder
                                .connect_to(ByteStreams::new(write_half, read_half))
                                .await;
                            if let Err(e) = result {
                                error!("Mock agent connection error: {}", e);
                            }
                            info!("Mock agent connection completed");
                        };

                        let shutdown_wait = async {
                            let _ = shutdown_rx.recv().await;
                            info!("Shutdown received while serving connection");
                        };

                        let _ = select(Box::pin(serve_fut), Box::pin(shutdown_wait)).await;
                    }
                    Either::Left((None, _)) => {
                        info!("Mock agent accept failed, exiting");
                    }
                    Either::Right((_, _)) => {
                        info!("Mock agent shutting down before connection established");
                    }
                }

                info!("Mock agent main task completed");
            }));

            info!("Mock agent thread exiting");
        });

        Ok(MockAgentHandle::new_unix_socket(
            config_clone,
            path,
            thread_handle,
            shutdown_tx,
        ))
    }
}

/// Helper: race a future against a timeout, matching tokio::time::timeout semantics.
async fn timeout<T>(
    duration: std::time::Duration,
    future: impl std::future::Future<Output = T>,
) -> Result<T, acp::Error> {
    match select(Box::pin(future), Box::pin(Timer::after(duration))).await {
        Either::Left((result, _)) => Ok(result),
        Either::Right((_, _)) => Err(internal_error("operation timed out")),
    }
}

/// Build an `Agent.builder()` with every inbound handler registered.
///
/// Each closure clones the shared `config` and reads from `MockConfig` to
/// determine the response.
fn build_mock_agent_builder(
    config: Arc<Mutex<MockConfig>>,
) -> acp::Builder<
    Agent,
    impl acp::HandleDispatchFrom<acp::Client>,
    impl acp::RunWithConnectionTo<acp::Client>,
> {
    Agent
        .builder()
        .name("mock-agent")
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: InitializeRequest,
                      responder: Responder<InitializeResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            Ok::<_, acp::Error>(config.lock().unwrap().initialize_response.clone())
                        })
                        .await
                        .map_err(|_| internal_error("initialize timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: AuthenticateRequest,
                      responder: Responder<AuthenticateResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            Ok::<_, acp::Error>(
                                config.lock().unwrap().authenticate_response.clone(),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("authenticate timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |request: NewSessionRequest,
                      responder: Responder<NewSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let mut config = config.lock().unwrap();
                            let response = config.new_session_response.clone();
                            config.track_session(response.session_id.clone(), request.cwd.clone());
                            config.new_session_response =
                                NewSessionResponse::new(generate_session_id());
                            Ok::<_, acp::Error>(response)
                        })
                        .await
                        .map_err(|_| internal_error("new_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                // Prompt handling drives a multi-step workflow that calls
                // `cx.send_request(...).block_task().await` on the spawned-task path.
                // If we awaited that workflow inside this handler, the dispatch loop
                // would deadlock (it cannot route the response while the handler is
                // still running). Per the 0.11 migration guide we spawn the work and
                // let it call `responder.respond(...)` when it completes.
                move |request: PromptRequest,
                      responder: Responder<PromptResponse>,
                      cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    let cx_for_prompt = cx.clone();
                    async move {
                        cx.spawn(async move {
                            let result = handle_prompt(config, cx_for_prompt, request).await;
                            let _ = responder.respond_with_result(result);
                            Ok(())
                        })
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_notification(
            move |_notification: CancelNotification, _cx: ConnectionTo<acp::Client>| async move {
                Ok(())
            },
            on_receive_notification!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |request: LoadSessionRequest,
                      responder: Responder<LoadSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            if let Some(ref response) = config.load_session_response {
                                return Ok(response.clone());
                            }
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
                        .map_err(|_| internal_error("load_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |request: ResumeSessionRequest,
                      responder: Responder<ResumeSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            if let Some(ref response) = config.resume_session_response {
                                return Ok(response.clone());
                            }
                            if config.sessions.contains_key(&request.session_id) {
                                Ok(ResumeSessionResponse::new())
                            } else {
                                Err(internal_error(format!(
                                    "session not found: {}",
                                    request.session_id
                                )))
                            }
                        })
                        .await
                        .map_err(|_| internal_error("resume_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: SetSessionModeRequest,
                      responder: Responder<SetSessionModeResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            Ok::<_, acp::Error>(
                                config.set_session_mode_response.clone().unwrap_or_default(),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("set_session_mode timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: SetSessionConfigOptionRequest,
                      responder: Responder<SetSessionConfigOptionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            Ok::<_, acp::Error>(
                                config
                                    .set_session_config_option_response
                                    .clone()
                                    .unwrap_or_else(|| SetSessionConfigOptionResponse::new(vec![])),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("set_session_config_option timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: ListSessionsRequest,
                      responder: Responder<ListSessionsResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            if let Some(ref response) = config.list_sessions_response {
                                return Ok(response.clone());
                            }
                            let sessions: Vec<_> = config.sessions.values().cloned().collect();
                            Ok(ListSessionsResponse::new(sessions))
                        })
                        .await
                        .map_err(|_| internal_error("list_sessions timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: CloseSessionRequest,
                      responder: Responder<CloseSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            Ok::<_, acp::Error>(
                                config.close_session_response.clone().unwrap_or_default(),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("close_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: DeleteSessionRequest,
                      responder: Responder<DeleteSessionResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result = timeout(dur, async {
                            let config = config.lock().unwrap();
                            Ok::<_, acp::Error>(
                                config.delete_session_response.clone().unwrap_or_default(),
                            )
                        })
                        .await
                        .map_err(|_| internal_error("delete_session timed out"))
                        .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
        .on_receive_request(
            {
                let config = config.clone();
                move |_req: LogoutRequest,
                      responder: Responder<LogoutResponse>,
                      _cx: ConnectionTo<acp::Client>| {
                    let config = config.clone();
                    async move {
                        let dur = config.lock().unwrap().timeout;
                        let result =
                            timeout(dur, async { Ok::<_, acp::Error>(LogoutResponse::new()) })
                                .await
                                .map_err(|_| internal_error("logout timed out"))
                                .and_then(|r| r);
                        responder.respond_with_result(result)
                    }
                }
            },
            on_receive_request!(),
        )
    // NOTE: `ExtRequest`/`ExtNotification` are not `JsonRpcRequest`/`JsonRpcNotification`
    // in agent-client-protocol 0.11. Handling them would require defining a custom
    // typed wrapper or registering a catch-all `on_receive_dispatch`. The previous
    // MockAgent implementation only returned a default response for them, which the
    // existing test suite never exercises meaningfully. They are intentionally omitted
    // here. If a test needs ext support, register a typed wrapper at that point.
}

/// Handle a prompt request. Drives the configured agent → client message flow
/// (permission requests, terminal lifecycle, file ops) via `cx.send_request(...)`
/// and `cx.send_notification(...)`, then echoes the prompt content back as agent
/// message chunks before returning EndTurn.
async fn handle_prompt(
    config: Arc<Mutex<MockConfig>>,
    cx: ConnectionTo<acp::Client>,
    request: PromptRequest,
) -> Result<PromptResponse, acp::Error> {
    let dur = config.lock().unwrap().timeout;
    timeout(dur, async {
        // Check if we should request permission
        let permission_request = {
            let config = config.lock().unwrap();
            config.permission_request.clone()
        };

        if let Some(perm_req) = permission_request {
            cx.send_request(perm_req)
                .block_task()
                .await
                .map_err(|e| internal_error(format!("permission request failed: {}", e)))?;
        }

        // Check if terminal workflow is configured
        let (create_terminal, send_terminal_output, send_terminal_exit) = {
            let config = config.lock().unwrap();
            (
                config.create_terminal_request.clone(),
                config.terminal_output_request.is_some(),
                config.wait_for_terminal_exit_request.is_some(),
            )
        };

        if let Some(create_req) = create_terminal {
            let create_response = cx
                .send_request(create_req)
                .block_task()
                .await
                .map_err(|e| internal_error(format!("create_terminal failed: {}", e)))?;

            let terminal_id = create_response.terminal_id;

            if send_terminal_output {
                let output_req =
                    TerminalOutputRequest::new(request.session_id.clone(), terminal_id.clone());
                cx.send_request(output_req)
                    .block_task()
                    .await
                    .map_err(|e| internal_error(format!("terminal_output failed: {}", e)))?;
            }

            if send_terminal_exit {
                let exit_req =
                    WaitForTerminalExitRequest::new(request.session_id.clone(), terminal_id);
                cx.send_request(exit_req)
                    .block_task()
                    .await
                    .map_err(|e| internal_error(format!("wait_for_terminal_exit failed: {}", e)))?;
            }
        }

        // Read text file (if configured)
        let read_file_request = {
            let config = config.lock().unwrap();
            config.read_file_request.clone()
        };

        if let Some(read_req) = read_file_request {
            cx.send_request(read_req)
                .block_task()
                .await
                .map_err(|e| internal_error(format!("read_text_file failed: {}", e)))?;
        }

        // Write text file (if configured)
        let write_file_request = {
            let config = config.lock().unwrap();
            config.write_file_request.clone()
        };

        if let Some(write_req) = write_file_request {
            cx.send_request(write_req)
                .block_task()
                .await
                .map_err(|e| internal_error(format!("write_text_file failed: {}", e)))?;
        }

        // Release terminal (if configured)
        let release_terminal_request = {
            let config = config.lock().unwrap();
            config.release_terminal_request.clone()
        };

        if let Some(release_req) = release_terminal_request {
            cx.send_request(release_req)
                .block_task()
                .await
                .map_err(|e| internal_error(format!("release_terminal failed: {}", e)))?;
        }

        // Send elicitation request (if configured)
        let elicitation_request = {
            let config = config.lock().unwrap();
            config.elicitation_request.clone()
        };

        if let Some(elic_req) = elicitation_request {
            cx.send_request(elic_req)
                .block_task()
                .await
                .map_err(|e| internal_error(format!("elicitation failed: {}", e)))?;
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

            if cx.send_notification(notification).is_err() {
                break;
            }
        }

        Ok(PromptResponse::new(StopReason::EndTurn))
    })
    .await
    .map_err(|_| internal_error("prompt timed out"))?
}
