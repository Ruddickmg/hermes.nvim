//! Configuration for MockAgent

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::schema::v1::{
    AgentCapabilities, AuthenticateResponse, CloseSessionResponse, CompleteElicitationNotification,
    CreateElicitationRequest, CreateTerminalRequest, DeleteSessionResponse, ElicitationId,
    ExtResponse, Implementation, InitializeResponse, ListSessionsResponse, LoadSessionResponse,
    McpCapabilities, NewSessionResponse, PermissionOption, PermissionOptionId,
    PermissionOptionKind, PromptCapabilities, ReadTextFileRequest, ReleaseTerminalRequest,
    RequestPermissionRequest, ResumeSessionResponse, SessionAdditionalDirectoriesCapabilities,
    SessionCapabilities, SessionCloseCapabilities, SessionDeleteCapabilities,
    SessionForkCapabilities, SessionId, SessionInfo, SessionListCapabilities,
    SessionResumeCapabilities, SetSessionConfigOptionResponse, SetSessionModeResponse,
    TerminalOutputRequest, ToolCallId, ToolCallUpdate, ToolCallUpdateFields,
    WaitForTerminalExitRequest, WriteTextFileRequest,
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
    /// Optional override for resume_session response
    pub resume_session_response: Option<ResumeSessionResponse>,
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
    /// Read text file request to send during prompt (None = skip)
    pub read_file_request: Option<ReadTextFileRequest>,
    /// Write text file request to send during prompt (None = skip)
    pub write_file_request: Option<WriteTextFileRequest>,
    /// Release terminal request to send during prompt (None = skip)
    pub release_terminal_request: Option<ReleaseTerminalRequest>,
    /// Elicitation request to send during prompt (None = skip)
    pub elicitation_request: Option<CreateElicitationRequest>,
    /// Elicitation complete notification to send during prompt (None = skip)
    pub elicitation_complete_notification: Option<CompleteElicitationNotification>,
    /// Close session response to return when a CloseSessionRequest is received
    pub close_session_response: Option<CloseSessionResponse>,
    /// Delete session response to return when a DeleteSessionRequest is received
    pub delete_session_response: Option<DeleteSessionResponse>,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            initialize_response: InitializeResponse::new(ProtocolVersion::LATEST)
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
                                .resume(Some(SessionResumeCapabilities::new()))
                                .close(Some(SessionCloseCapabilities::new()))
                                .delete(Some(SessionDeleteCapabilities::new()))
                                .additional_directories(Some(
                                    SessionAdditionalDirectoriesCapabilities::new(),
                                )),
                        ),
                ),
            authenticate_response: AuthenticateResponse::default(),
            new_session_response: NewSessionResponse::new(generate_session_id()),
            permission_request: None,
            load_session_response: None,
            resume_session_response: None,
            list_sessions_response: None,
            set_session_mode_response: None,
            set_session_config_option_response: None,
            ext_response: None,
            sessions: HashMap::new(),
            timeout: Duration::from_secs(30),
            create_terminal_request: None,
            terminal_output_request: None,
            wait_for_terminal_exit_request: None,
            read_file_request: None,
            write_file_request: None,
            release_terminal_request: None,
            elicitation_request: None,
            elicitation_complete_notification: None,
            close_session_response: None,
            delete_session_response: None,
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

    /// Set a custom resume_session response
    pub fn set_resume_session_response(mut self, response: ResumeSessionResponse) -> Self {
        self.resume_session_response = Some(response);
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

    /// Set a read text file request to send during prompt
    pub fn set_read_file_request(mut self, request: ReadTextFileRequest) -> Self {
        self.read_file_request = Some(request);
        self
    }

    /// Set a write text file request to send during prompt
    pub fn set_write_file_request(mut self, request: WriteTextFileRequest) -> Self {
        self.write_file_request = Some(request);
        self
    }

    /// Set a release terminal request to send during prompt
    pub fn set_release_terminal_request(mut self, request: ReleaseTerminalRequest) -> Self {
        self.release_terminal_request = Some(request);
        self
    }

    /// Set an elicitation request to send during prompt
    pub fn set_elicitation_request(mut self, request: CreateElicitationRequest) -> Self {
        self.elicitation_request = Some(request);
        self
    }

    /// Set an elicitation complete notification to send during prompt
    pub fn set_elicitation_complete_notification(
        mut self,
        notification: CompleteElicitationNotification,
    ) -> Self {
        self.elicitation_complete_notification = Some(notification);
        self
    }

    /// Set a close session response to return on CloseSessionRequest
    pub fn set_close_session_response(mut self, response: CloseSessionResponse) -> Self {
        self.close_session_response = Some(response);
        self
    }

    /// Set a delete session response to return on DeleteSessionRequest
    pub fn set_delete_session_response(mut self, response: DeleteSessionResponse) -> Self {
        self.delete_session_response = Some(response);
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
                PermissionOptionId::new("reject-once"),
                "Reject Once",
                PermissionOptionKind::RejectOnce,
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
    terminal_id: impl Into<agent_client_protocol::schema::v1::TerminalId>,
) -> TerminalOutputRequest {
    TerminalOutputRequest::new(session_id, terminal_id)
}

/// Create a default wait for terminal exit request for testing
pub fn create_test_wait_for_terminal_exit_request(
    session_id: impl Into<SessionId>,
    terminal_id: impl Into<agent_client_protocol::schema::v1::TerminalId>,
) -> WaitForTerminalExitRequest {
    WaitForTerminalExitRequest::new(session_id, terminal_id)
}

/// Create an elicitation complete notification for testing
pub fn create_test_elicitation_complete_notification(
    elicitation_id: impl Into<ElicitationId>,
) -> CompleteElicitationNotification {
    CompleteElicitationNotification::new(elicitation_id)
}
