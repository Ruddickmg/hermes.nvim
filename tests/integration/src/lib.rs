//! Integration tests for autocommand functionality via Handler
//!
//! Tests verify that the Handler struct properly manages autocommands
//! and communicates with the Neovim main thread.
#![allow(clippy::arc_with_non_send_sync)]

use crate::helpers::mock_runtime;
use hermes::acp::handler::Handler;
use hermes::nvim::{autocommands::Commands, requests::Requests, state::PluginState};
use nvim_oxi::api::opts::{CreateAugroupOpts, CreateAutocmdOpts};
use std::rc::Rc;
use std::sync::Arc;
use async_lock::Mutex;

pub mod acp;
pub mod helpers;
pub mod nvim;
pub mod request;
pub mod utilities;

const GROUP: &str = "hermes";

fn create_test_autogroup() -> nvim_oxi::Result<u32> {
    let _ = nvim_oxi::api::create_buf(false, true)?;
    let _buffer = nvim_oxi::api::get_current_buf();
    nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default())?;
    Ok(0)
}

fn create_test_autocmd(command: Commands) -> nvim_oxi::Result<u32> {
    let opts = CreateAutocmdOpts::builder()
        .patterns([command.to_string().as_str()])
        .group(GROUP)
        .command("echo 'test'")
        .build();

    let id = nvim_oxi::api::create_autocmd(["User"], &opts)?;

    Ok(id)
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn test_listener_attached_no_listener() -> nvim_oxi::Result<()> {
    create_test_autogroup()?;
    assert!(
        !Handler::listener_attached(Commands::ToolCall),
        "Should return false when no listener is attached"
    );
    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn test_listener_attached_with_listener() -> nvim_oxi::Result<()> {
    create_test_autogroup()?;
    create_test_autocmd(Commands::PermissionRequest)?;
    assert!(
        Handler::listener_attached(Commands::PermissionRequest),
        "Should return true when a listener is attached"
    );
    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn test_handler_new_creates_valid_instance() -> nvim_oxi::Result<()> {
    // Integration: Verify Handler can be instantiated with Requests handler
    // This tests the constructor which sets up mpsc channel and AsyncHandle
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests = Rc::new(Requests::new(mock_runtime(), state.clone())?);
    let handler =
        Handler::new(state, mock_runtime(), requests).expect("Handler creation should succeed");

    // If we get here without error, the integration worked
    // The instance is valid and ready to use
    drop(handler);
    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn test_execute_autocommand_sends_to_channel() -> nvim_oxi::Result<()> {
    // Integration: Verify message is queued via mpsc channel
    // Uses: channel.send(), AsyncHandle.send()
    let state = Arc::new(Mutex::new(PluginState::default()));
    let requests = Rc::new(Requests::new(mock_runtime(), state.clone())?);
    let handler =
        Handler::new(state, mock_runtime(), requests).expect("Handler creation should succeed");

    // Execute an autocommand with test data
    let test_data = serde_json::json!({"test": "data"});
    smol::block_on(async {
        handler
            .execute_autocommand(Commands::ToolCall, test_data)
            .await
    })?;

    // If no error occurred, the message was successfully sent to the channel
    // and AsyncHandle was triggered
    Ok(())
}
