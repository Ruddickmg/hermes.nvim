use hermes::nvim::{
    autocommands::{AutoCommand, Commands},
    requests::Requests,
};
use nvim_oxi::api::opts::{CreateAugroupOpts, CreateAutocmdOpts};

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
