//! Integration tests for Handler Client trait implementations
use agent_client_protocol::{
    Client, ReadTextFileRequest, WriteTextFileRequest,
};
use hermes::acp::handler::Handler;
use hermes::nvim::state::PluginState;
use crate::helpers::{MockClient, MockRequestHandler};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

#[nvim_oxi::test]
fn test_write_text_file_permissions_allowed() -> nvim_oxi::Result<()> {
    let _mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let req = WriteTextFileRequest::new("session_id", "test.txt", "test");
    let _ = tokio_test::block_on(handler.write_text_file(req));
    
    Ok(())
}

#[nvim_oxi::test]
fn test_write_text_file_calls_handler() -> nvim_oxi::Result<()> {
    let mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let req = WriteTextFileRequest::new("session_id", "test.txt", "test");
    let _ = tokio_test::block_on(handler.write_text_file(req));
    
    let was_called = tokio_test::block_on(async {
        *mock.write_called.lock().await
    });
    assert!(was_called, "Handler should have been called");
    
    Ok(())
}

#[nvim_oxi::test]
fn test_write_text_file_permissions_denied() -> nvim_oxi::Result<()> {
    let _mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.fs_write_access = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let req = WriteTextFileRequest::new("session_id", "test.txt", "test");
    let res = tokio_test::block_on(handler.write_text_file(req));
    assert!(res.is_err(), "Should return error when permissions denied");
    
    Ok(())
}

#[nvim_oxi::test]
fn test_write_text_file_permissions_denied_does_not_call_handler() -> nvim_oxi::Result<()> {
    let mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.fs_write_access = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let req = WriteTextFileRequest::new("session_id", "test.txt", "test");
    let _ = tokio_test::block_on(handler.write_text_file(req));
    
    let was_called = tokio_test::block_on(async {
        *mock.write_called.lock().await
    });
    assert!(!was_called, "Handler should NOT have been called when permissions denied");
    
    Ok(())
}

#[nvim_oxi::test]
fn test_read_text_file_permissions_allowed() -> nvim_oxi::Result<()> {
    let _mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let req = ReadTextFileRequest::new("session_id", "test.txt");
    let _ = tokio_test::block_on(handler.read_text_file(req));
    
    Ok(())
}

#[nvim_oxi::test]
fn test_read_text_file_calls_handler() -> nvim_oxi::Result<()> {
    let mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let req = ReadTextFileRequest::new("session_id", "test.txt");
    let _ = tokio_test::block_on(handler.read_text_file(req));
    
    let was_called = tokio_test::block_on(async {
        *mock.read_called.lock().await
    });
    assert!(was_called, "Handler should have been called");
    
    Ok(())
}

#[nvim_oxi::test]
fn test_read_text_file_permissions_denied() -> nvim_oxi::Result<()> {
    let _mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.fs_read_access = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let req = ReadTextFileRequest::new("session_id", "test.txt");
    let res = tokio_test::block_on(handler.read_text_file(req));
    assert!(res.is_err(), "Should return error when permissions denied");
    
    Ok(())
}

#[nvim_oxi::test]
fn test_read_text_file_permissions_denied_does_not_call_handler() -> nvim_oxi::Result<()> {
    let mock = MockClient::new();
    let state = Arc::new(Mutex::new(PluginState::default()));
    state.blocking_lock().config.permissions.fs_read_access = false;

    let handler = Handler::new(state.clone(), Rc::new(MockRequestHandler::new())).expect("Handler creation should succeed");

    let req = ReadTextFileRequest::new("session_id", "test.txt");
    let _ = tokio_test::block_on(handler.read_text_file(req));
    
    let was_called = tokio_test::block_on(async {
        *mock.read_called.lock().await
    });
    assert!(!was_called, "Handler should NOT have been called when permissions denied");
    
    Ok(())
}
