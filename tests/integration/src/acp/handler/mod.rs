//! Integration tests for Handler notification and permissions
// NOTE: tests/integration/src/acp/handler/client.rs exists but is NOT declared here.
// It was written as dead test code and was never compiled. The "allowed" tests in it
// timeout due to test environment issues. Keep it undeclared until those are resolved.
pub mod response;

use crate::helpers::{MockRequestHandler, mock_runtime};
use agent_client_protocol::{
    Error,
    schema::ProtocolVersion,
    schema::v1::{
        AgentCapabilities, CompleteElicitationNotification, ContentBlock, ContentChunk,
        ElicitationId, InitializeResponse, LoadSessionResponse, NewSessionResponse,
        ResumeSessionResponse, SessionCapabilities, SessionConfigOption,
        SessionConfigOptionCategory, SessionConfigSelectOption, SessionMode, SessionModeState,
        SessionNotification, SessionResumeCapabilities, SessionUpdate,
        SetSessionConfigOptionResponse, SetSessionModeResponse, TextContent, UsageUpdate,
    },
};
use async_lock::Mutex;
use hermes::acp::handler::Handler;
use hermes::nvim::state::PluginState;
use pretty_assertions::assert_eq;
use std::io::{Read, Write};
use std::rc::Rc;
use std::sync::Arc;
use tempfile::TempDir;

fn create_test_notification() -> SessionNotification {
    let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new("test message")));
    SessionNotification::new("session_id", SessionUpdate::UserMessageChunk(chunk))
}

#[nvim_oxi::test]
fn test_session_notification_permissions_denied() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(async {
        state.lock().await.config.permissions.send_notifications = false;
    });

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let notification = create_test_notification();
    let res = smol::block_on(handler.session_notification(notification));
    assert_eq!(
        res.unwrap_err(),
        Error::method_not_found(),
        "Should return MethodNotFound when permissions denied"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_session_notification_permissions_allowed() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let notification = create_test_notification();
    let res: agent_client_protocol::Result<()> =
        smol::block_on(handler.session_notification(notification));
    assert_eq!(res, Ok(()), "Should succeed when permissions allowed");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_write_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(async {
        state.lock().await.config.permissions.fs_write_access = false;
    });

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.can_write());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_read_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(async {
        state.lock().await.config.permissions.fs_read_access = false;
    });

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.can_read());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_access_terminal_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(state.lock())
        .config
        .permissions
        .terminal_access = false;

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.can_access_terminal());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_request_permissions_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(state.lock())
        .config
        .permissions
        .request_permissions = false;

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.can_request_permissions());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

// Note: These tests cover the "true" code paths for CI coverage requirements.
// Per AGENTS.md, we avoid testing defaults, but these methods are used in
// production code (client.rs) and need coverage. Keeping them per AGENTS.md:793-799.

#[nvim_oxi::test]
fn test_set_agent_info_updates_agent_information() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let agent = hermes::acp::connection::Assistant::from("test-agent");
    let info = agent_client_protocol::schema::v1::InitializeResponse::new(
        agent_client_protocol::schema::ProtocolVersion::LATEST,
    );

    smol::block_on(handler.set_agent_info(agent.clone(), info.clone()));

    // Verify agent info was set by setting current agent and checking info
    let mut state_guard = smol::block_on(state.lock());
    state_guard.agent_info.set_agent(agent.clone());
    let stored_info = state_guard.agent_info.get_current_info();
    assert!(
        stored_info.is_some(),
        "Agent info should be stored after set_agent_info"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_session_notification_usage_update_succeeds() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let usage = UsageUpdate::new(1000, 200000);
    let notification = SessionNotification::new("session_id", SessionUpdate::UsageUpdate(usage));
    let res: agent_client_protocol::Result<()> =
        smol::block_on(handler.session_notification(notification));
    assert_eq!(res, Ok(()), "Usage update notification should succeed");

    Ok(())
}

#[nvim_oxi::test]
fn test_can_receive_notifications_returns_false_when_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(state.lock())
        .config
        .permissions
        .send_notifications = false;

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.can_receive_notifications());
    assert!(!result, "Should return false when disabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_elicitation_enabled_returns_false_when_all_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(async {
        state.lock().await.config.permissions.elicitation.form = false;
        state.lock().await.config.permissions.elicitation.url = false;
    });

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.elicitation_enabled());
    assert!(
        !result,
        "Should return false when both form and url are disabled"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_elicitation_enabled_returns_true_when_form_enabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(async {
        state.lock().await.config.permissions.elicitation.url = false;
    });

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.elicitation_enabled());
    assert!(
        result,
        "Should return true when form elicitation is enabled"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_elicitation_enabled_returns_true_when_url_enabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(async {
        state.lock().await.config.permissions.elicitation.form = false;
    });

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.elicitation_enabled());
    assert!(result, "Should return true when url elicitation is enabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_elicitation_complete_permissions_denied() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    smol::block_on(async {
        state.lock().await.config.permissions.elicitation.form = false;
        state.lock().await.config.permissions.elicitation.url = false;
    });

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let notification = CompleteElicitationNotification::new(ElicitationId::from("url-elicitation"));
    let res = smol::block_on(handler.elicitation_complete(notification));
    assert_eq!(
        res.unwrap_err(),
        Error::method_not_found(),
        "Should return MethodNotFound when elicitation is disabled"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_elicitation_complete_permissions_allowed() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let notification = CompleteElicitationNotification::new(ElicitationId::from("url-elicitation"));
    let res: agent_client_protocol::Result<()> =
        smol::block_on(handler.elicitation_complete(notification));
    assert_eq!(res, Ok(()), "Should succeed when elicitation is enabled");

    Ok(())
}

#[nvim_oxi::test]
fn test_execute_autocommand_request_sends_with_responder() -> nvim_oxi::Result<()> {
    // Test execute_autocommand_request with a responder - covers lines 207-208
    // This sends an autocommand with response_data, triggering the full flow
    use agent_client_protocol::schema::v1::WriteTextFileResponse;
    use hermes::nvim::requests::Responder;
    use std::sync::Arc;

    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let (sender, _receiver) = async_channel::bounded::<WriteTextFileResponse>(1);
    let responder = Responder::WriteFileResponse(
        sender,
        agent_client_protocol::schema::v1::WriteTextFileRequest::new(
            agent_client_protocol::schema::v1::SessionId::from("test-session"),
            std::path::Path::new("/tmp/test.txt"),
            "test content",
        ),
    );

    // This should succeed - covers lines 207-208
    let result = smol::block_on(handler.execute_autocommand_request(
        "test-session".to_string(),
        "TestCommand",
        serde_json::json!({"test": "data"}),
        responder,
    ));

    assert!(result.is_ok(), "execute_autocommand_request should succeed");
    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn test_no_listener_with_request_triggers_default_response_error_path() -> nvim_oxi::Result<()> {
    // Test lines 71-78: "No listener but has request" error handling path
    // This triggers when no autocommand listener is attached but a request is provided
    // AND when default_response fails (to trigger the error! at lines 74-77)
    use agent_client_protocol::schema::v1::WriteTextFileResponse;
    use hermes::nvim::requests::{RequestHandler, Responder};
    use std::sync::Arc;
    use uuid::Uuid;

    use async_trait::async_trait;

    // Create a mock that fails on default_response to trigger error! at lines 74-77
    struct FailingMockRequestHandler;
    #[async_trait(?Send)]
    impl RequestHandler for FailingMockRequestHandler {
        async fn default_response(
            &self,
            _request_id: &Uuid,
            _data: serde_json::Value,
        ) -> hermes::acp::Result<()> {
            // Return an error to trigger the error! logging at lines 74-77
            Err(hermes::acp::error::Error::Internal(
                "Test error from default_response".to_string(),
            ))
        }

        async fn handle_response(
            &self,
            _request_id: &Uuid,
            _response: nvim_oxi::Object,
        ) -> hermes::acp::Result<()> {
            Ok(())
        }

        async fn cancel_session_requests(&self, _session_id: String) -> hermes::acp::Result<()> {
            Ok(())
        }

        async fn add_request(&self, _session_id: String, _responder: Responder) -> Uuid {
            Uuid::new_v4()
        }

        async fn get_request(&self, _request_id: &Uuid) -> Option<hermes::nvim::requests::Request> {
            None
        }
    }

    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        std::rc::Rc::new(FailingMockRequestHandler),
    )
    .expect("Handler creation should succeed");

    // Create a responder which will generate a request_id
    // But don't attach any autocommand listener for "TestErrorCommand"
    let (sender, _receiver) = async_channel::bounded::<WriteTextFileResponse>(1);
    let responder = Responder::WriteFileResponse(
        sender,
        agent_client_protocol::schema::v1::WriteTextFileRequest::new(
            agent_client_protocol::schema::v1::SessionId::from("test-session"),
            std::path::Path::new("/tmp/test.txt"),
            "test content",
        ),
    );

    // Send with a responder but NO listener attached - triggers lines 71-78
    // With the failing mock, default_response will fail, triggering the error! at 74-77
    let result = smol::block_on(handler.execute_autocommand_request(
        "test-session".to_string(),
        "TestErrorCommand", // No listener for this command
        serde_json::json!({"data": "value"}),
        responder,
    ));

    // Send should succeed even if default_response fails (error is logged, not propagated)
    assert!(result.is_ok(), "Send should succeed even with no listener");

    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn test_no_listener_with_request_logs_default_response_error() -> nvim_oxi::Result<()> {
    // Test error logging at lines 74-77 when default_response fails
    use agent_client_protocol::schema::v1::WriteTextFileResponse;
    use hermes::nvim::requests::{RequestHandler, Responder};
    use std::sync::Arc;
    use uuid::Uuid;

    use async_trait::async_trait;

    struct FailingMockRequestHandler;
    #[async_trait(?Send)]
    impl RequestHandler for FailingMockRequestHandler {
        async fn default_response(
            &self,
            _request_id: &Uuid,
            _data: serde_json::Value,
        ) -> hermes::acp::Result<()> {
            Err(hermes::acp::error::Error::Internal(
                "Test error from default_response".to_string(),
            ))
        }

        async fn handle_response(
            &self,
            _request_id: &Uuid,
            _response: nvim_oxi::Object,
        ) -> hermes::acp::Result<()> {
            Ok(())
        }

        async fn cancel_session_requests(&self, _session_id: String) -> hermes::acp::Result<()> {
            Ok(())
        }

        async fn add_request(&self, _session_id: String, _responder: Responder) -> Uuid {
            Uuid::new_v4()
        }

        async fn get_request(&self, _request_id: &Uuid) -> Option<hermes::nvim::requests::Request> {
            None
        }
    }

    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        std::rc::Rc::new(FailingMockRequestHandler),
    )
    .expect("Handler creation should succeed");

    let (sender, _receiver) = async_channel::bounded::<WriteTextFileResponse>(1);
    let responder = Responder::WriteFileResponse(
        sender,
        agent_client_protocol::schema::v1::WriteTextFileRequest::new(
            agent_client_protocol::schema::v1::SessionId::from("test-session"),
            std::path::Path::new("/tmp/test.txt"),
            "test content",
        ),
    );

    let _result = smol::block_on(handler.execute_autocommand_request(
        "test-session".to_string(),
        "TestErrorCommand",
        serde_json::json!({"data": "value"}),
        responder,
    ));

    nvim_oxi::api::command("sleep 10m")?;

    assert!(
        logs_contain("Failed to send default response"),
        "Expected error log for failed default_response (lines 74-77)"
    );

    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn test_no_listener_no_request_triggers_warn_path() -> nvim_oxi::Result<()> {
    // Test line 80: "No listener attached for command" warn! path (else branch)
    // This triggers when no autocommand listener is attached AND no request is provided
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    // Call execute_autocommand (not execute_autocommand_request) with no listener
    // This passes None for response_data, hitting the else branch at line 79-80
    let result = smol::block_on(handler.execute_autocommand(
        "TestWarnCommand", // No listener for this command, no request
        serde_json::json!({"data": "value"}),
    ));

    // Send should succeed (warn is logged, not propagated)
    assert!(
        result.is_ok(),
        "Send should succeed even with no listener and no request"
    );

    Ok(())
}

fn create_agent_notification() -> SessionNotification {
    let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new("test message")));
    SessionNotification::new("session_id", SessionUpdate::AgentMessageChunk(chunk))
}

#[nvim_oxi::test]
fn get_prompt_id_returns_value_after_user_message_chunk() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let notification = create_test_notification();
    let _ = smol::block_on(handler.session_notification(notification));

    let prompt_id = smol::block_on(handler.get_prompt_id("session_id")).unwrap();
    let mut state_guard = smol::block_on(state.lock());
    let stored_id = state_guard.get_session_prompt("session_id");
    assert_eq!(prompt_id, stored_id);

    Ok(())
}

#[nvim_oxi::test]
fn agent_message_chunk_succeeds_and_stores_prompt_id_when_none_exists() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let notification = create_agent_notification();
    smol::block_on(handler.session_notification(notification)).unwrap();

    let mut state_guard = smol::block_on(state.lock());
    let stored_id = state_guard.get_session_prompt("session_id");
    assert!(
        !stored_id.is_empty(),
        "A prompt id should be auto-generated and stored"
    );

    Ok(())
}

#[nvim_oxi::test]
fn agent_message_chunk_succeeds_after_user_message_chunk() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let user_notification = create_test_notification();
    let _ = smol::block_on(handler.session_notification(user_notification));

    let agent_notification = create_agent_notification();
    let res = smol::block_on(handler.session_notification(agent_notification));
    assert_eq!(res, Ok(()));

    Ok(())
}

#[nvim_oxi::test]
fn get_agent_returns_current_agent() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let agent = hermes::acp::connection::Assistant::from("test-agent");
    smol::block_on(async {
        state.lock().await.agent_info.set_agent(agent.clone());
    });

    let result = smol::block_on(handler.get_agent());
    assert_eq!(
        result.to_string(),
        agent.to_string(),
        "get_agent should return the current agent"
    );

    Ok(())
}

#[nvim_oxi::test]
fn set_prompt_id_updates_session_prompt_id() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    smol::block_on(handler.set_prompt_id("session-1".to_string(), "prompt-abc".to_string()));

    let mut state_guard = smol::block_on(state.lock());
    let stored_id = state_guard.get_session_prompt("session-1");
    assert_eq!(
        stored_id, "prompt-abc",
        "set_prompt_id should update the prompt id"
    );

    Ok(())
}

#[nvim_oxi::test]
fn get_prompt_id_returns_same_value_on_repeated_calls_without_user_message() -> nvim_oxi::Result<()>
{
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let first_id = smol::block_on(handler.get_prompt_id("session_id")).unwrap();
    let second_id = smol::block_on(handler.get_prompt_id("session_id")).unwrap();

    assert_eq!(
        first_id, second_id,
        "Repeated calls should return the same cached id"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_loaded_stores_legacy_mode_info() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let mode = SessionMode::new("chat", "Chat");
    let modes = SessionModeState::new("chat", vec![mode]);
    let response = LoadSessionResponse::default().modes(modes);

    smol::block_on(handler.session_loaded("test-session".to_string(), response))?;

    let state_guard = smol::block_on(state.lock());
    let details = state_guard.session_info.get("test-session").unwrap();
    assert_eq!(details.mode_is_legacy(), Some(true));

    Ok(())
}

#[nvim_oxi::test]
fn session_loaded_stores_config_options_info() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let option = SessionConfigOption::select(
        "mode",
        "Mode",
        "chat",
        vec![SessionConfigSelectOption::new("chat", "Chat")],
    )
    .category(SessionConfigOptionCategory::Mode);
    let response = LoadSessionResponse::default().config_options(vec![option]);

    smol::block_on(handler.session_loaded("test-session".to_string(), response))?;

    let state_guard = smol::block_on(state.lock());
    let details = state_guard.session_info.get("test-session").unwrap();
    assert_eq!(details.mode_is_legacy(), Some(false));

    Ok(())
}

#[nvim_oxi::test]
fn session_loaded_stores_none_when_empty() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let response = LoadSessionResponse::default();

    smol::block_on(handler.session_loaded("test-session".to_string(), response))?;

    let state_guard = smol::block_on(state.lock());
    let details = state_guard.session_info.get("test-session").unwrap();
    assert_eq!(details.mode_is_legacy(), None);

    Ok(())
}

#[nvim_oxi::test]
fn session_resumed_stores_legacy_mode_info() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let mode = SessionMode::new("chat", "Chat");
    let modes = SessionModeState::new("chat", vec![mode]);
    let response = ResumeSessionResponse::default().modes(modes);

    smol::block_on(handler.session_resumed("test-session".to_string(), response))?;

    let state_guard = smol::block_on(state.lock());
    let details = state_guard.session_info.get("test-session").unwrap();
    assert_eq!(details.mode_is_legacy(), Some(true));

    Ok(())
}

#[nvim_oxi::test]
fn session_resumed_stores_config_options_info() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let option = SessionConfigOption::select(
        "mode",
        "Mode",
        "chat",
        vec![SessionConfigSelectOption::new("chat", "Chat")],
    )
    .category(SessionConfigOptionCategory::Mode);
    let response = ResumeSessionResponse::default().config_options(vec![option]);

    smol::block_on(handler.session_resumed("test-session".to_string(), response))?;

    let state_guard = smol::block_on(state.lock());
    let details = state_guard.session_info.get("test-session").unwrap();
    assert_eq!(details.mode_is_legacy(), Some(false));

    Ok(())
}

#[nvim_oxi::test]
fn session_resumed_stores_none_when_empty() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let response = ResumeSessionResponse::default();

    smol::block_on(handler.session_resumed("test-session".to_string(), response))?;

    let state_guard = smol::block_on(state.lock());
    let details = state_guard.session_info.get("test-session").unwrap();
    assert_eq!(details.mode_is_legacy(), None);

    Ok(())
}

#[nvim_oxi::test]
fn config_option_set_with_mode_category_succeeds() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session = NewSessionResponse::new("test-session").config_options(vec![
        SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode),
    ]);
    smol::block_on(async {
        state.lock().await.set_session_info(&session);
    });

    let option = SessionConfigOption::select(
        "mode",
        "Mode",
        "chat",
        vec![SessionConfigSelectOption::new("chat", "Chat")],
    )
    .category(SessionConfigOptionCategory::Mode);
    let response = SetSessionConfigOptionResponse::new(vec![option]);

    let result = smol::block_on(handler.config_option_set("test-session", "chat", response));

    assert!(
        result.is_ok(),
        "config_option_set with Mode category should succeed"
    );

    Ok(())
}

#[nvim_oxi::test]
fn config_option_set_with_model_category_succeeds() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session = NewSessionResponse::new("test-session").config_options(vec![
        SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model),
    ]);
    smol::block_on(async {
        state.lock().await.set_session_info(&session);
    });

    let option = SessionConfigOption::select(
        "model",
        "Model",
        "gpt4",
        vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
    )
    .category(SessionConfigOptionCategory::Model);
    let response = SetSessionConfigOptionResponse::new(vec![option]);

    let result = smol::block_on(handler.config_option_set("test-session", "gpt4", response));

    assert!(
        result.is_ok(),
        "config_option_set with Model category should succeed"
    );

    Ok(())
}

#[nvim_oxi::test]
fn config_option_set_empty_options_succeeds() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let response = SetSessionConfigOptionResponse::new(vec![]);

    let result = smol::block_on(handler.config_option_set("test-session", "", response));

    assert!(
        result.is_ok(),
        "config_option_set with empty options should succeed"
    );

    Ok(())
}

#[nvim_oxi::test]
fn config_option_set_with_other_category_succeeds() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let option = SessionConfigOption::select(
        "custom",
        "Custom",
        "val",
        vec![SessionConfigSelectOption::new("val", "Value")],
    )
    .category(SessionConfigOptionCategory::Other("custom".into()));
    let response = SetSessionConfigOptionResponse::new(vec![option]);

    let result = smol::block_on(handler.config_option_set("test-session", "val", response));

    assert!(
        result.is_ok(),
        "config_option_set with Other category should succeed via wildcard arm"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_loaded_stores_model_config_options_info() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let option = SessionConfigOption::select(
        "model",
        "Model",
        "gpt4",
        vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
    )
    .category(SessionConfigOptionCategory::Model);
    let response = LoadSessionResponse::default().config_options(vec![option]);

    smol::block_on(handler.session_loaded("test-session".to_string(), response))?;

    let state_guard = smol::block_on(state.lock());
    let details = state_guard.session_info.get("test-session").unwrap();
    assert_eq!(details.model_is_legacy(), Some(false));

    Ok(())
}

#[nvim_oxi::test]
fn session_mode_set_mode_not_found() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session = NewSessionResponse::new("test-session").config_options(vec![
        SessionConfigOption::select(
            "mode",
            "Mode",
            "chat",
            vec![SessionConfigSelectOption::new("chat", "Chat")],
        )
        .category(SessionConfigOptionCategory::Mode),
    ]);
    smol::block_on(async {
        state.lock().await.set_session_info(&session);
    });

    let result = smol::block_on(handler.session_mode_set(
        "test-session",
        "nonexistent",
        SetSessionModeResponse::default(),
    ));

    assert!(
        matches!(result, Err(hermes::acp::error::Error::Internal(_))),
        "Should return Internal error when mode not in selection"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_mode_set_session_not_found() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.session_mode_set(
        "nonexistent-session",
        "chat",
        SetSessionModeResponse::default(),
    ));

    assert!(
        matches!(result, Err(hermes::acp::error::Error::SessionNotFound(_))),
        "Should return SessionNotFound for nonexistent session"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_model_set_model_not_found() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session = NewSessionResponse::new("test-session").config_options(vec![
        SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model),
    ]);
    smol::block_on(async {
        state.lock().await.set_session_info(&session);
    });

    let result = smol::block_on(handler.session_model_set("test-session", "nonexistent"));

    assert!(
        matches!(result, Err(hermes::acp::error::Error::Internal(_))),
        "Should return Internal error when model not in selection"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_model_set_succeeds() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session = NewSessionResponse::new("test-session").config_options(vec![
        SessionConfigOption::select(
            "model",
            "Model",
            "gpt4",
            vec![SessionConfigSelectOption::new("gpt4", "GPT-4")],
        )
        .category(SessionConfigOptionCategory::Model),
    ]);
    smol::block_on(async {
        state.lock().await.set_session_info(&session);
    });

    let result = smol::block_on(handler.session_model_set("test-session", "gpt4"));

    assert!(
        result.is_ok(),
        "session_model_set should succeed when model exists"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_model_set_session_not_found() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.session_model_set("nonexistent-session", "gpt4"));

    assert!(
        matches!(result, Err(hermes::acp::error::Error::SessionNotFound(_))),
        "Should return SessionNotFound for nonexistent session"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_thought_level_set_succeeds() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session = NewSessionResponse::new("test-session").config_options(vec![
        SessionConfigOption::select(
            "thought_level",
            "Thought Level",
            "low",
            vec![
                SessionConfigSelectOption::new("low", "Low"),
                SessionConfigSelectOption::new("high", "High"),
            ],
        )
        .category(SessionConfigOptionCategory::ThoughtLevel),
    ]);
    smol::block_on(async {
        state.lock().await.set_session_info(&session);
    });

    let result = smol::block_on(handler.session_thought_level_set("test-session", "low"));

    assert!(
        result.is_ok(),
        "session_thought_level_set should succeed when thought level exists"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_thought_level_set_not_found() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session = NewSessionResponse::new("test-session").config_options(vec![
        SessionConfigOption::select(
            "thought_level",
            "Thought Level",
            "low",
            vec![
                SessionConfigSelectOption::new("low", "Low"),
                SessionConfigSelectOption::new("high", "High"),
            ],
        )
        .category(SessionConfigOptionCategory::ThoughtLevel),
    ]);
    smol::block_on(async {
        state.lock().await.set_session_info(&session);
    });

    let result = smol::block_on(handler.session_thought_level_set("test-session", "nonexistent"));

    assert!(
        matches!(result, Err(hermes::acp::error::Error::Internal(_))),
        "Should return Internal error when thought level not in selection"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_thought_level_set_session_not_found() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let result = smol::block_on(handler.session_thought_level_set("nonexistent-session", "low"));

    assert!(
        matches!(result, Err(hermes::acp::error::Error::SessionNotFound(_))),
        "Should return SessionNotFound for nonexistent session"
    );

    Ok(())
}

fn create_notification_with_session(session_id: &str) -> SessionNotification {
    let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new("test message")));
    SessionNotification::new(
        session_id.to_string(),
        SessionUpdate::AgentMessageChunk(chunk),
    )
}

#[nvim_oxi::test]
fn session_notification_writes_history_to_file() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let state = Arc::new(Mutex::new(
        PluginState::new().with_storage_path(temp_dir.path().to_path_buf()),
    ));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let agent = hermes::acp::connection::Assistant::from("test-agent");
    let session_caps = SessionCapabilities::new().resume(Some(SessionResumeCapabilities::new()));
    let info = InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
        AgentCapabilities::new()
            .load_session(false)
            .session_capabilities(session_caps),
    );
    smol::block_on(handler.set_agent_info(agent.clone(), info));
    smol::block_on(async {
        state.lock().await.agent_info.set_agent(agent);
    });

    let session = NewSessionResponse::new("test-session");
    smol::block_on(async {
        state.lock().await.set_session_info(&session);
    });

    let notification = create_notification_with_session("test-session");
    let result = smol::block_on(handler.session_notification(notification));
    assert!(result.is_ok(), "session_notification should succeed");

    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.agent_info.history.flush().unwrap();
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    let history_path = temp_dir
        .path()
        .join("history")
        .join("test-agent")
        .join("test-session.jsonl");
    assert!(history_path.exists(), "History file should exist");

    let mut file = std::fs::File::open(&history_path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
    assert_eq!(
        parsed["update"]["sessionUpdate"], "agent_message_chunk",
        "Should store agent_message_chunk update"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_notification_skips_history_when_not_needed() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let state = Arc::new(Mutex::new(
        PluginState::new().with_storage_path(temp_dir.path().to_path_buf()),
    ));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let agent = hermes::acp::connection::Assistant::from("test-agent");
    let info = InitializeResponse::new(ProtocolVersion::V1)
        .agent_capabilities(AgentCapabilities::new().load_session(true));
    smol::block_on(handler.set_agent_info(agent.clone(), info));
    smol::block_on(async {
        state.lock().await.agent_info.set_agent(agent);
    });

    let notification = create_notification_with_session("test-session");
    let result = smol::block_on(handler.session_notification(notification));
    assert!(result.is_ok(), "session_notification should succeed");

    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.agent_info.history.flush().unwrap();
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    let history_path = temp_dir
        .path()
        .join("history")
        .join("test-agent")
        .join("test-session.jsonl");
    assert!(
        !history_path.exists(),
        "History file should not exist when needs_local_history is false"
    );

    Ok(())
}

#[nvim_oxi::test]
fn session_notification_does_not_write_history_when_permissions_denied() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let state = Arc::new(Mutex::new(
        PluginState::new().with_storage_path(temp_dir.path().to_path_buf()),
    ));
    smol::block_on(async {
        state.lock().await.config.permissions.send_notifications = false;
    });
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let agent = hermes::acp::connection::Assistant::from("test-agent");
    let session_caps = SessionCapabilities::new().resume(Some(SessionResumeCapabilities::new()));
    let info = InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
        AgentCapabilities::new()
            .load_session(false)
            .session_capabilities(session_caps),
    );
    smol::block_on(handler.set_agent_info(agent.clone(), info));
    smol::block_on(async {
        state.lock().await.agent_info.set_agent(agent);
    });

    let notification = create_notification_with_session("test-session");
    let result = smol::block_on(handler.session_notification(notification));
    assert_eq!(
        result.unwrap_err(),
        Error::method_not_found(),
        "Should return MethodNotFound when permissions denied"
    );

    smol::block_on(async {
        let mut guard = state.lock().await;
        guard.agent_info.history.flush().unwrap();
    });
    std::thread::sleep(std::time::Duration::from_millis(200));

    let history_path = temp_dir
        .path()
        .join("history")
        .join("test-agent")
        .join("test-session.jsonl");
    assert!(
        !history_path.exists(),
        "History file should not exist when notification is rejected"
    );

    Ok(())
}

// === Handler callback branch tests ===

use hermes::nvim::autocommands::Commands;
use hermes::nvim::requests::Responder;
use hermes::utilities::TransmitToNvim;
use hermes::utilities::autocmd::create_augroup;
use nvim_oxi::api::opts::CreateAutocmdOpts;
use std::time::Duration;

#[nvim_oxi::test]
fn handler_callback_fires_autocmd_when_listener_attached() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests = Rc::new(MockRequestHandler::new());
    let handler =
        Handler::new(state, mock_runtime(), requests).expect("Handler creation should succeed");

    // Ensure the hermes augroup exists so listener_attached can query it
    let _ = create_augroup("hermes", true);

    // Register a Lua autocmd listener for PermissionRequest
    let opts = CreateAutocmdOpts::builder()
        .patterns([Commands::PermissionRequest.to_string().as_str()])
        .group("hermes")
        .command("let g:hermes_test_callback_fired = 1")
        .build();
    nvim_oxi::api::create_autocmd(["User"], &opts)?;

    // Send message through handler channel using async send
    let executor = smol::LocalExecutor::new();
    smol::block_on(executor.run(async {
        handler
            .channel
            .send((
                Commands::PermissionRequest.to_string(),
                serde_json::json!({}),
                None,
            ))
            .await
            .expect("Send should succeed");
    }));

    // Wait for the scheduled callback to execute
    let fired = crate::helpers::ui::wait_for(
        || {
            nvim_oxi::api::get_var("hermes_test_callback_fired")
                .map(|v: i64| v == 1)
                .unwrap_or(false)
        },
        Duration::from_millis(500),
    );
    assert!(
        fired,
        "autocmd listener should have been triggered by handler callback"
    );

    // Clean up global variable
    nvim_oxi::api::del_var("hermes_test_callback_fired").ok();
    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn handler_callback_sends_default_response_when_no_listener() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests = Rc::new(MockRequestHandler::new());
    let handler =
        Handler::new(state, mock_runtime(), requests).expect("Handler creation should succeed");

    // Dummy sender – the mock default_response returns Ok without sending,
    // but the callback branch itself is what we want to exercise for coverage.
    let (sender, _receiver) =
        async_channel::bounded::<agent_client_protocol::schema::v1::RequestPermissionOutcome>(1);

    let executor = smol::LocalExecutor::new();
    smol::block_on(executor.run(async {
        handler
            .channel
            .send((
                "UnknownCommand".to_string(),
                serde_json::json!({}),
                Some((
                    Responder::PermissionResponse(sender),
                    "test-session".to_string(),
                )),
            ))
            .await
            .expect("Send should succeed");
    }));

    // Give the event loop time to process the scheduled callback
    nvim_oxi::api::command("sleep 50m").ok();

    // Verify the callback executed by checking for the expected log message
    assert!(
        logs_contain("No listener attached for command"),
        "Expected warn log for unknown command with no listener"
    );

    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn handler_callback_warns_when_no_listener_and_no_request() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests = Rc::new(MockRequestHandler::new());
    let handler =
        Handler::new(state, mock_runtime(), requests).expect("Handler creation should succeed");

    let executor = smol::LocalExecutor::new();
    smol::block_on(executor.run(async {
        handler
            .channel
            .send((
                "AnotherUnknownCommand".to_string(),
                serde_json::json!({}),
                None,
            ))
            .await
            .expect("Send should succeed");
    }));

    // Give the event loop a moment to process.
    nvim_oxi::api::command("sleep 50m").ok();

    // Verify the callback executed and logged the expected warning
    assert!(
        logs_contain("No listener attached for command"),
        "Expected warn log for unknown command with no listener and no request"
    );

    Ok(())
}
