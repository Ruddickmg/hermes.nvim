use crate::helpers::{MockRequestHandler, mock_connection_manager, mock_runtime};
use agent_client_protocol::schema::v1::{
    AuthMethodTerminal, AuthenticateResponse, CloseSessionResponse, DeleteSessionResponse,
    ForkSessionResponse, ListSessionsResponse, ResumeSessionResponse,
};
use async_lock::Mutex;
use hermes::acp::connection::Assistant;
use hermes::acp::handler::Handler;
use hermes::acp::session_info::SessionDetails;
use hermes::nvim::state::PluginState;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn create_handler() -> Handler {
    let state = Arc::new(Mutex::new(PluginState::default()));
    Handler::new(
        state.clone(),
        mock_connection_manager(&state),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed")
}

/// Drives a future to completion on the main thread, pumping Neovim's event
/// loop between polls so `AsyncHandle` callbacks and `vim.schedule` callbacks
/// can run. Returns `None` when the deadline elapses.
fn drive<F: std::future::Future>(future: F) -> Option<F::Output> {
    let mut future = std::pin::pin!(future);
    let waker = futures::task::noop_waker();
    let mut context = std::task::Context::from_waker(&waker);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(value) => return Some(value),
            std::task::Poll::Pending => {
                if Instant::now() >= deadline {
                    return None;
                }
                nvim_oxi::api::command("sleep 10m").ok();
            }
        }
    }
}

#[nvim_oxi::test]
fn authenticated_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let response = AuthenticateResponse::default();
    let result = smol::block_on(handler.authenticated(response));
    assert!(result.is_ok(), "authenticated should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn terminal_authentication_succeeds() -> nvim_oxi::Result<()> {
    let handler = Arc::new(create_handler());
    let result = drive(handler.terminal_authentication(
        Assistant::Opencode,
        handler.clone(),
        AuthMethodTerminal::new("tui-auth".to_string(), "Terminal Auth"),
    ))
    .expect("terminal_authentication should settle before the deadline");
    assert!(result.is_ok(), "terminal_authentication should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn custom_command_executed_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let raw = serde_json::value::RawValue::from_string("{}".to_string())
        .map(std::sync::Arc::from)
        .expect("RawValue creation should succeed");
    let response = agent_client_protocol::schema::v1::ExtResponse::new(raw);
    let result = smol::block_on(handler.custom_command_executed(response));
    assert!(result.is_ok(), "custom_command_executed should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn sessions_listed_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let response = ListSessionsResponse::new(vec![]);
    let result = smol::block_on(handler.sessions_listed(response));
    assert!(result.is_ok(), "sessions_listed should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn session_forked_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let response = ForkSessionResponse::new("forked-session");
    let result = smol::block_on(handler.session_forked(response));
    assert!(result.is_ok(), "session_forked should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn session_resumed_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let session_id = String::from("test-session");
    let response = ResumeSessionResponse::default();
    let result = smol::block_on(handler.session_resumed(session_id, response));
    assert!(result.is_ok(), "session_resumed should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn session_closed_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let session_id = String::from("test-session");
    let response = CloseSessionResponse::default();
    let result = smol::block_on(handler.session_closed(session_id, response));
    assert!(result.is_ok(), "session_closed should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn session_closed_removes_session_info() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_connection_manager(&state),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session_id = String::from("test-session");
    // Insert session info using SessionDetails
    {
        let mut locked = smol::block_on(state.lock());
        locked
            .session_info
            .insert(session_id.clone(), SessionDetails::default());
        locked
            .prompt
            .insert(session_id.clone(), "stale-prompt".to_string());
    }

    let response = CloseSessionResponse::default();
    smol::block_on(handler.session_closed(session_id.clone(), response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let locked = smol::block_on(state.lock());
    assert!(
        !locked.session_info.contains_key(&session_id),
        "session_info should be removed after close"
    );
    Ok(())
}

#[nvim_oxi::test]
fn session_closed_removes_prompt() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_connection_manager(&state),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session_id = String::from("test-session");
    // Insert prompt data
    {
        let mut locked = smol::block_on(state.lock());
        locked
            .prompt
            .insert(session_id.clone(), "stale-prompt".to_string());
    }

    let response = CloseSessionResponse::default();
    smol::block_on(handler.session_closed(session_id.clone(), response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let locked = smol::block_on(state.lock());
    assert!(
        !locked.prompt.contains_key(&session_id),
        "prompt should be removed after close"
    );
    Ok(())
}

#[nvim_oxi::test]
fn session_notification_session_info_update_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let info = agent_client_protocol::schema::v1::SessionInfoUpdate::new();
    let notification = agent_client_protocol::schema::v1::SessionNotification::new(
        "test-session",
        agent_client_protocol::schema::v1::SessionUpdate::SessionInfoUpdate(info),
    );
    let result = smol::block_on(handler.session_notification(notification));
    assert_eq!(
        result,
        Ok(()),
        "SessionInfoUpdate should map to Commands::SessionUpdate and succeed"
    );
    Ok(())
}

#[nvim_oxi::test]
fn session_deleted_succeeds() -> nvim_oxi::Result<()> {
    let handler = create_handler();
    let session_id = String::from("test-session");
    let response = DeleteSessionResponse::default();
    let result = smol::block_on(handler.session_deleted(session_id, response));
    assert!(result.is_ok(), "session_deleted should succeed");
    Ok(())
}

#[nvim_oxi::test]
fn session_deleted_removes_session_info() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_connection_manager(&state),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session_id = String::from("test-session");
    // Insert session info using SessionDetails
    {
        let mut locked = smol::block_on(state.lock());
        locked
            .session_info
            .insert(session_id.clone(), SessionDetails::default());
        locked
            .prompt
            .insert(session_id.clone(), "stale-prompt".to_string());
    }

    let response = DeleteSessionResponse::default();
    smol::block_on(handler.session_deleted(session_id.clone(), response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let locked = smol::block_on(state.lock());
    assert!(
        !locked.session_info.contains_key(&session_id),
        "session_info should be removed after delete"
    );
    Ok(())
}

#[nvim_oxi::test]
fn session_deleted_removes_prompt() -> nvim_oxi::Result<()> {
    let state = Arc::new(Mutex::new(PluginState::default()));
    let handler = Handler::new(
        state.clone(),
        mock_connection_manager(&state),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed");

    let session_id = String::from("test-session");
    // Insert prompt data
    {
        let mut locked = smol::block_on(state.lock());
        locked
            .prompt
            .insert(session_id.clone(), "stale-prompt".to_string());
    }

    let response = DeleteSessionResponse::default();
    smol::block_on(handler.session_deleted(session_id.clone(), response))
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    let locked = smol::block_on(state.lock());
    assert!(
        !locked.prompt.contains_key(&session_id),
        "prompt should be removed after delete"
    );
    Ok(())
}
