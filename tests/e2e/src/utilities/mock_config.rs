//! Configuration for MockAgent

use agent_client_protocol::{
    AuthenticateResponse, CreateTerminalRequest, ExtResponse, InitializeResponse,
    ListSessionsResponse, LoadSessionResponse, NewSessionResponse, PermissionOption,
    PermissionOptionId, PermissionOptionKind, ProtocolVersion, RequestPermissionRequest, SessionId,
    SessionInfo, SetSessionConfigOptionResponse, SetSessionModeResponse, TerminalOutputRequest,
    ToolCallId, ToolCallUpdate, ToolCallUpdateFields, WaitForTerminalExitRequest,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Default responses for mock agent methods
#[derive(Clone)]
pub struct MockConfig {
    pub initialize_response: InitializeResponse,
    pub authenticate_response: AuthenticateResponse,
    pub new_session_response: NewSessionResponse,
    /// Permission request to send during prompt (None = don't request permission)
    pub permission_request: Option<RequestPermissionRequest>,
    /// Optional override for load_session response
    pub load_session_response: Option<LoadSessionResponse>,
    /// Optional override for list_sessions response
    pub list_sessions_response: Option<ListSessionsResponse>,
    /// Optional override for set_session_mode response
    pub set_session_mode_response: Option<SetSessionModeResponse>,
    /// Optional override for set_session_config_option response
    pub set_session_config_option_response: Option<SetSessionConfigOptionResponse>,
    /// Optional override for ext_method response
    pub ext_response: Option<ExtResponse>,
    /// Session tracking (used for default behavior of load_session and list_sessions)
    pub sessions: HashMap<SessionId, SessionInfo>,
    /// Global timeout for all agent methods
    pub timeout: Duration,
    /// Terminal creation request to send during prompt (None = skip)
    pub create_terminal_request: Option<CreateTerminalRequest>,
    /// Terminal output request to send during prompt (None = skip)
    pub terminal_output_request: Option<TerminalOutputRequest>,
    /// Wait for terminal exit request to send during prompt (None = skip)
    pub wait_for_terminal_exit_request: Option<WaitForTerminalExitRequest>,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            initialize_response: InitializeResponse::new(ProtocolVersion::LATEST),
            authenticate_response: AuthenticateResponse::default(),
            new_session_response: NewSessionResponse::new(generate_session_id()),
            permission_request: None,
            load_session_response: None,
            list_sessions_response: None,
            set_session_mode_response: None,
            set_session_config_option_response: None,
            ext_response: None,
            sessions: HashMap::new(),
            timeout: Duration::from_secs(30),
            create_terminal_request: None,
            terminal_output_request: None,
            wait_for_terminal_exit_request: None,
        }
    }
}

impl MockConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initialize response
    pub fn set_initialize_response(mut self, response: InitializeResponse) -> Self {
        self.initialize_response = response;
        self
    }

    /// Set the authenticate response
    pub fn set_authenticate_response(mut self, response: AuthenticateResponse) -> Self {
        self.authenticate_response = response;
        self
    }

    /// Set the new_session response
    pub fn set_new_session_response(mut self, response: NewSessionResponse) -> Self {
        self.new_session_response = response;
        self
    }

    /// Set a permission request to send during prompt
    pub fn set_permission_request(mut self, request: RequestPermissionRequest) -> Self {
        self.permission_request = Some(request);
        self
    }

    /// Clear the permission request (don't request permission)
    pub fn clear_permission_request(mut self) -> Self {
        self.permission_request = None;
        self
    }

    /// Set a custom load_session response (overrides default session tracking)
    pub fn set_load_session_response(mut self, response: LoadSessionResponse) -> Self {
        self.load_session_response = Some(response);
        self
    }

    /// Set a custom list_sessions response (overrides default session tracking)
    pub fn set_list_sessions_response(mut self, response: ListSessionsResponse) -> Self {
        self.list_sessions_response = Some(response);
        self
    }

    /// Set a custom set_session_mode response
    pub fn set_set_session_mode_response(mut self, response: SetSessionModeResponse) -> Self {
        self.set_session_mode_response = Some(response);
        self
    }

    /// Set a custom set_session_config_option response
    pub fn set_set_session_config_option_response(
        mut self,
        response: SetSessionConfigOptionResponse,
    ) -> Self {
        self.set_session_config_option_response = Some(response);
        self
    }

    /// Set a custom ext_method response
    pub fn set_ext_response(mut self, response: ExtResponse) -> Self {
        self.ext_response = Some(response);
        self
    }

    /// Set the global timeout for all agent methods
    pub fn set_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set a terminal creation request to send during prompt
    pub fn set_create_terminal_request(mut self, request: CreateTerminalRequest) -> Self {
        self.create_terminal_request = Some(request);
        self
    }

    /// Set a terminal output request to send during prompt
    pub fn set_terminal_output_request(mut self, request: TerminalOutputRequest) -> Self {
        self.terminal_output_request = Some(request);
        self
    }

    /// Set a wait for terminal exit request to send during prompt
    pub fn set_wait_for_terminal_exit_request(
        mut self,
        request: WaitForTerminalExitRequest,
    ) -> Self {
        self.wait_for_terminal_exit_request = Some(request);
        self
    }

    /// Track a newly created session
    pub fn track_session(&mut self, session_id: SessionId, cwd: PathBuf) {
        let session_info = SessionInfo::new(session_id.clone(), cwd);
        self.sessions.insert(session_id, session_info);
    }
}

/// Create a simple permission request for testing
pub fn create_test_permission_request(session_id: &str) -> RequestPermissionRequest {
    RequestPermissionRequest::new(
        SessionId::from(session_id.to_string()),
        ToolCallUpdate::new(
            ToolCallId::from("mock-tool-call-id"),
            ToolCallUpdateFields::default(),
        ),
        vec![
            PermissionOption::new(
                PermissionOptionId::new("allow-once"),
                "Allow Once",
                PermissionOptionKind::AllowOnce,
            ),
            PermissionOption::new(
                PermissionOptionId::new("deny"),
                "Deny",
                PermissionOptionKind::AllowOnce,
            ),
        ],
    )
}

/// Create a default ext_response with empty JSON object {}
pub fn default_ext_response() -> ExtResponse {
    let raw_value: Box<serde_json::value::RawValue> =
        serde_json::value::RawValue::from_string("{}".to_string()).unwrap();
    ExtResponse::new(std::sync::Arc::from(raw_value))
}

/// Generate a new unique session ID using UUID
pub fn generate_session_id() -> SessionId {
    SessionId::from(uuid::Uuid::new_v4().to_string())
}

/// Create a default terminal creation request for testing
pub fn create_test_create_terminal_request(
    session_id: impl Into<SessionId>,
    command: impl Into<String>,
    args: Vec<String>,
) -> CreateTerminalRequest {
    CreateTerminalRequest::new(session_id, command).args(args)
}

/// Create a default terminal output request for testing
pub fn create_test_terminal_output_request(
    session_id: impl Into<SessionId>,
    terminal_id: impl Into<agent_client_protocol::TerminalId>,
) -> TerminalOutputRequest {
    TerminalOutputRequest::new(session_id, terminal_id)
}

/// Create a default wait for terminal exit request for testing
pub fn create_test_wait_for_terminal_exit_request(
    session_id: impl Into<SessionId>,
    terminal_id: impl Into<agent_client_protocol::TerminalId>,
) -> WaitForTerminalExitRequest {
    WaitForTerminalExitRequest::new(session_id, terminal_id)
}
