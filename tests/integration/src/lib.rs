use hermes::nvim::{
    autocommands::{AutoCommand, Commands},
    requests::Requests,
};
use nvim_oxi::api::opts::{CreateAugroupOpts, CreateAutocmdOpts};
use std::sync::Arc;

pub mod helpers;
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
        !AutoCommand::<Requests>::listener_attached(Commands::ToolCall),
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
        AutoCommand::<Requests>::listener_attached(Commands::PermissionRequest),
        "Should return true when a listener is attached"
    );
    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn test_autocommand_new_creates_valid_instance() -> nvim_oxi::Result<()> {
    // Integration: Verify AutoCommand can be instantiated with Requests handler
    // This tests the constructor which sets up mpsc channel and AsyncHandle
    let requests = Arc::new(Requests::new()?);
    let autocommand = AutoCommand::new(requests)?;
    
    // If we get here without error, the integration worked
    // The instance is valid and ready to use
    drop(autocommand);
    Ok(())
}

#[tracing_test::traced_test]
#[nvim_oxi::test]
fn test_execute_autocommand_sends_to_channel() -> nvim_oxi::Result<()> {
    // Integration: Verify message is queued via mpsc channel
    // Uses: channel.send(), AsyncHandle.send()
    let requests = Arc::new(Requests::new()?);
    let autocommand = AutoCommand::new(requests)?;
    
    // Execute an autocommand with test data
    let test_data = serde_json::json!({"test": "data"});
    tokio_test::block_on(async {
        autocommand
            .execute_autocommand(Commands::ToolCall, test_data)
            .await
    })?;
    
    // If no error occurred, the message was successfully sent to the channel
    // and AsyncHandle was triggered
    Ok(())
}

