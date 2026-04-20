//! Integration tests for nvim API entry points
//!
//! Note: API function tests should verify actual logic (error handling,
//! connection management, etc.) not just struct construction.
//!
//! See request/handler.rs for examples of actual integration tests.

use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::helpers::mock_runtime;
use hermes::acp::handler::Handler;
use hermes::nvim::state::PluginState;

use agent_client_protocol::{
    AgentCapabilities, InitializeResponse, McpCapabilities, PromptCapabilities, ProtocolVersion,
    SessionCapabilities, SessionForkCapabilities, SessionListCapabilities,
    SessionResumeCapabilities,
};

use crate::helpers::MockRequestHandler;
use pretty_assertions::assert_eq;

/// Helper to set up agent info with specific capabilities
async fn setup_agent_with_capabilities(
    handler: &Handler,
    load_session: bool,
    images: bool,
    audio: bool,
    embedded_context: bool,
    mcp_http: bool,
    mcp_sse: bool,
    list_sessions: bool,
    fork_sessions: bool,
    resume_sessions: bool,
) {
    let agent = hermes::acp::connection::Assistant::from("test-agent");
    let session_caps = SessionCapabilities::new()
        .list(if list_sessions {
            Some(SessionListCapabilities::new())
        } else {
            None
        })
        .fork(if fork_sessions {
            Some(SessionForkCapabilities::new())
        } else {
            None
        })
        .resume(if resume_sessions {
            Some(SessionResumeCapabilities::new())
        } else {
            None
        });
    let info = InitializeResponse::new(ProtocolVersion::V1).agent_capabilities(
        AgentCapabilities::new()
            .load_session(load_session)
            .prompt_capabilities(
                PromptCapabilities::new()
                    .image(images)
                    .audio(audio)
                    .embedded_context(embedded_context),
            )
            .mcp_capabilities(McpCapabilities::new().http(mcp_http).sse(mcp_sse))
            .session_capabilities(session_caps),
    );
    handler.set_agent_info(agent.clone(), info).await;
}

#[nvim_oxi::test]
fn test_agent_with_all_capabilities_disabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    // Set up agent with all capabilities disabled
    tokio_test::block_on(setup_agent_with_capabilities(
        &handler, false, false, false, false, false, false, false, false, false,
    ));

    // Verify all capabilities are disabled (single assertion comparing all)
    let mut state_guard = state.blocking_lock();
    let agent = hermes::acp::connection::Assistant::from("test-agent");
    state_guard.agent_info.set_agent(agent);
    assert_eq!(
        (
            state_guard.agent_info.can_load_session(),
            state_guard.agent_info.can_send_images(),
            state_guard.agent_info.can_send_audio(),
            state_guard.agent_info.can_send_embedded_context(),
            state_guard.agent_info.can_connect_to_mcp_over_http(),
            state_guard.agent_info.can_connect_to_mcp_over_sse(),
            state_guard.agent_info.can_list_sessions(),
            state_guard.agent_info.can_fork_sessions(),
            state_guard.agent_info.can_resume_sessions(),
        ),
        (
            false, false, false, false, false, false, false, false, false
        )
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_agent_with_all_capabilities_enabled() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    // Set up agent with all capabilities enabled
    tokio_test::block_on(setup_agent_with_capabilities(
        &handler, true, true, true, true, true, true, true, true, true,
    ));

    // Verify all capabilities are enabled (single assertion comparing all)
    let mut state_guard = state.blocking_lock();
    let agent = hermes::acp::connection::Assistant::from("test-agent");
    state_guard.agent_info.set_agent(agent);
    assert_eq!(
        (
            state_guard.agent_info.can_load_session(),
            state_guard.agent_info.can_send_images(),
            state_guard.agent_info.can_send_audio(),
            state_guard.agent_info.can_send_embedded_context(),
            state_guard.agent_info.can_connect_to_mcp_over_http(),
            state_guard.agent_info.can_connect_to_mcp_over_sse(),
            state_guard.agent_info.can_list_sessions(),
            state_guard.agent_info.can_fork_sessions(),
            state_guard.agent_info.can_resume_sessions(),
        ),
        (true, true, true, true, true, true, true, true, true)
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_agent_with_mixed_capabilities() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    // Set up agent with mixed capabilities
    tokio_test::block_on(setup_agent_with_capabilities(
        &handler, true, false, true, false, true, false, true, false, true,
    ));

    // Verify mixed capabilities (single assertion comparing all)
    let mut state_guard = state.blocking_lock();
    let agent = hermes::acp::connection::Assistant::from("test-agent");
    state_guard.agent_info.set_agent(agent);
    assert_eq!(
        (
            state_guard.agent_info.can_load_session(),
            state_guard.agent_info.can_send_images(),
            state_guard.agent_info.can_send_audio(),
            state_guard.agent_info.can_send_embedded_context(),
            state_guard.agent_info.can_connect_to_mcp_over_http(),
            state_guard.agent_info.can_connect_to_mcp_over_sse(),
            state_guard.agent_info.can_list_sessions(),
            state_guard.agent_info.can_fork_sessions(),
            state_guard.agent_info.can_resume_sessions(),
        ),
        (true, false, true, false, true, false, true, false, true)
    );

    Ok(())
}
