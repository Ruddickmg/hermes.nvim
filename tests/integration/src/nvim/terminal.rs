//! Integration tests for TerminalInfo
//!
//! These tests verify that TerminalInfo works correctly within the Neovim runtime.

use hermes::acp::Result;
use hermes::nvim::configuration::TerminalConfig;
use hermes::nvim::terminal::{Terminal, TerminalInfo, handle_terminal_exit};
use hermes::utilities::buf_options::get_buf_option;
use pretty_assertions::assert_eq;

/// Integration test: Verifies report_exit_to sends exit code when already occurred
#[nvim_oxi::test]
fn terminal_info_report_exit_to_sends_exit_code_when_already_occurred() -> nvim_oxi::Result<()> {
    let terminal = TerminalInfo::new(None);

    // Manually set exit status (simulating what exit callback would do)
    *terminal
        .exit_status
        .try_borrow_mut()
        .expect("Failed to borrow exit_status for test setup") =
        Some((Some(5), Some("error".to_string())));

    let (sender, receiver) = async_channel::bounded::<Result<(Option<u32>, Option<String>)>>(1);
    terminal.report_exit_to(sender).expect("report failed");

    let received = receiver.try_recv().expect("Should receive message");
    assert_eq!(received.unwrap(), (Some(5), Some("error".to_string())));

    Ok(())
}

/// Integration test: Verifies report_exit_to returns Ok when sender is available
#[nvim_oxi::test]
fn terminal_info_report_exit_to_returns_ok_when_sender_available() -> nvim_oxi::Result<()> {
    let terminal = TerminalInfo::new(None);

    *terminal
        .exit_status
        .try_borrow_mut()
        .expect("Failed to borrow exit_status for test setup") =
        Some((Some(5), Some("error".to_string())));

    let (sender, _receiver) = async_channel::bounded::<Result<(Option<u32>, Option<String>)>>(1);
    let result = terminal.report_exit_to(sender);
    assert!(
        result.is_ok(),
        "report_exit_to should return Ok when sender is available"
    );

    Ok(())
}

/// Integration test: Verifies report_exit_to stores sender for later notification
#[nvim_oxi::test]
fn terminal_info_report_exit_to_stores_sender_for_later() -> nvim_oxi::Result<()> {
    let terminal = TerminalInfo::new(None);

    let (sender, receiver) = async_channel::bounded::<Result<(Option<u32>, Option<String>)>>(1);
    terminal.report_exit_to(sender).expect("report failed");

    // Verify the sender was stored by simulating an exit and receiving through the original channel
    let mut exit_status = terminal
        .exit_status
        .try_borrow_mut()
        .expect("Failed to borrow exit_status");
    let mut exit_response = terminal
        .exit_response
        .try_borrow_mut()
        .expect("Failed to borrow exit_response");

    handle_terminal_exit(
        42,
        "exit".to_string(),
        &mut *exit_status,
        &mut *exit_response,
    )
    .expect("handle_terminal_exit failed");

    let received = receiver
        .try_recv()
        .expect("Should receive exit notification");
    assert_eq!(received.unwrap(), (Some(42), None));

    Ok(())
}

/// Integration test: Verifies run() returns positive job ID
#[nvim_oxi::test]
fn terminal_info_run_returns_positive_job_id() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    // Start a simple echo command with detailed error logging for CI debugging
    let job_id = terminal
        .run("echo".to_string(), vec!["hello".to_string()])
        .expect("Failed to start terminal job");

    assert!(job_id > 0, "Job ID should be positive, got: {}", job_id);

    Ok(())
}

/// Integration test: Verifies stop() returns error on non-running terminal
#[nvim_oxi::test]
fn terminal_info_stop_fails_on_non_running_terminal() -> nvim_oxi::Result<()> {
    let terminal = TerminalInfo::new(None);

    // Stop on a terminal that was never run should fail (no job ID)
    let result = terminal.stop();

    assert!(
        matches!(result, Err(hermes::acp::error::Error::Internal(_))),
        "stop() on non-running terminal should return Internal error"
    );

    Ok(())
}

/// Integration test: Verifies FromRequest with cwd sets correct path
#[nvim_oxi::test]
fn terminal_info_from_request_applies_cwd_path() -> nvim_oxi::Result<()> {
    use agent_client_protocol::v1::schema::{CreateTerminalRequest, SessionId};
    use std::path::PathBuf;

    let mut request =
        CreateTerminalRequest::new(SessionId::from("test-session"), "test".to_string());
    request.cwd = Some(PathBuf::from("/tmp"));

    let terminal = TerminalInfo::from(request);

    // Verify cwd was set in configuration by extracting and comparing the value
    let cwd_obj = terminal.configuration.get("cwd").unwrap();
    let cwd_oxi_str: nvim_oxi::String = cwd_obj.clone().try_into().unwrap();
    let cwd_str: String = cwd_oxi_str.to_string();
    assert_eq!(cwd_str, "/tmp");

    Ok(())
}

/// Integration test: Verifies FromRequest with env sets correct variables
#[nvim_oxi::test]
fn terminal_info_from_request_applies_environment_variables() -> nvim_oxi::Result<()> {
    use agent_client_protocol::v1::schema::{CreateTerminalRequest, EnvVariable, SessionId};

    let mut request =
        CreateTerminalRequest::new(SessionId::from("test-session"), "test".to_string());
    request.env = vec![EnvVariable::new(
        "TEST_KEY".to_string(),
        "test_value".to_string(),
    )];

    let terminal = TerminalInfo::from(request);

    // Verify env was set in configuration by extracting and checking the value
    let env_dict_obj = terminal.configuration.get("env").unwrap();
    let env_dict: nvim_oxi::Dictionary = env_dict_obj.clone().try_into().unwrap();
    let test_key_obj = env_dict.get("TEST_KEY").unwrap();
    let test_key_oxi_str: nvim_oxi::String = test_key_obj.clone().try_into().unwrap();
    let test_key_value: String = test_key_oxi_str.to_string();
    assert_eq!(test_key_value, "test_value");

    Ok(())
}

/// Integration test: Verifies working_directory builder sets correct path
#[nvim_oxi::test]
fn terminal_info_working_directory_builder_sets_path() -> nvim_oxi::Result<()> {
    use std::path::PathBuf;

    let terminal = TerminalInfo::new(None).working_directory(PathBuf::from("/home/user"));

    // Verify cwd was set by extracting and comparing the value
    let cwd_obj = terminal.configuration.get("cwd").unwrap();
    let cwd_oxi_str: nvim_oxi::String = cwd_obj.clone().try_into().unwrap();
    let cwd_str: String = cwd_oxi_str.to_string();
    assert_eq!(cwd_str, "/home/user");

    Ok(())
}

/// Integration test: Verifies environment builder sets correct variables
#[nvim_oxi::test]
fn terminal_info_environment_builder_sets_variables() -> nvim_oxi::Result<()> {
    use agent_client_protocol::v1::schema::EnvVariable;

    let terminal = TerminalInfo::new(None)
        .environment(vec![EnvVariable::new("FOO".to_string(), "bar".to_string())]);

    // Verify env was set by extracting and checking the value
    let env_dict_obj = terminal.configuration.get("env").unwrap();
    let env_dict: nvim_oxi::Dictionary = env_dict_obj.clone().try_into().unwrap();
    let foo_obj = env_dict.get("FOO").unwrap();
    let foo_oxi_str: nvim_oxi::String = foo_obj.clone().try_into().unwrap();
    let foo_value: String = foo_oxi_str.to_string();
    assert_eq!(foo_value, "bar");

    Ok(())
}

/// Integration test: Verifies run() sets buftype to terminal
#[nvim_oxi::test]
fn terminal_info_run_sets_buftype_to_terminal() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    terminal
        .run("echo".to_string(), vec!["hello".to_string()])
        .expect("Failed to start terminal job");

    // Verify the buffer was created and buftype is set
    let buffer = terminal.buffer().expect("Buffer should be created");
    let buftype: String = get_buf_option("buftype", &buffer)?;

    assert_eq!(buftype, "terminal");

    Ok(())
}

/// Integration test: Verifies run() sets swapfile to false
#[nvim_oxi::test]
fn terminal_info_run_sets_swapfile_to_false() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    terminal
        .run("echo".to_string(), vec!["hello".to_string()])
        .expect("Failed to start terminal job");

    let buffer = terminal.buffer().expect("Buffer should be created");
    let swapfile: bool = get_buf_option("swapfile", &buffer)?;

    assert!(!swapfile, "swapfile should be false for terminal buffers");

    Ok(())
}

/// Integration test: Verifies run() sets bufhidden to hide
#[nvim_oxi::test]
fn terminal_info_run_sets_bufhidden_to_hide() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    terminal
        .run("echo".to_string(), vec!["hello".to_string()])
        .expect("Failed to start terminal job");

    let buffer = terminal.buffer().expect("Buffer should be created");
    let bufhidden: String = get_buf_option("bufhidden", &buffer)?;

    assert_eq!(bufhidden, "hide");

    Ok(())
}

/// Integration test: Verifies run() sets scrollback to 10000
#[nvim_oxi::test]
fn terminal_info_run_sets_scrollback_to_10000() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    terminal
        .run("echo".to_string(), vec!["hello".to_string()])
        .expect("Failed to start terminal job");

    let buffer = terminal.buffer().expect("Buffer should be created");
    let scrollback: i64 = get_buf_option("scrollback", &buffer)?;

    assert_eq!(scrollback, 10000);

    Ok(())
}

/// Integration test: Verifies run() sets modified to false
#[nvim_oxi::test]
fn terminal_info_run_sets_modified_to_false() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    terminal
        .run("echo".to_string(), vec!["hello".to_string()])
        .expect("Failed to start terminal job");

    let buffer = terminal.buffer().expect("Buffer should be created");
    let modified: bool = get_buf_option("modified", &buffer)?;

    assert!(!modified, "modified should be false for terminal buffers");

    Ok(())
}

/// Integration test: Verifies configure() with enabled=true sets term option
#[nvim_oxi::test]
fn terminal_info_configure_enabled_true_sets_term_option() -> nvim_oxi::Result<()> {
    let config = TerminalConfig {
        enabled: true,
        delete: false,
        hidden: true,
        buffered: false,
    };

    let terminal = TerminalInfo::new(None).configure(config);

    let term_value = terminal
        .configuration
        .get("term")
        .expect("term should be set");
    let term_enabled = unsafe { term_value.as_boolean_unchecked() };

    assert!(term_enabled, "term option should be true when enabled=true");

    Ok(())
}

/// Integration test: Verifies configure() with enabled=false sets term to false
#[nvim_oxi::test]
fn terminal_info_configure_enabled_false_sets_term_to_false() -> nvim_oxi::Result<()> {
    let config = TerminalConfig {
        enabled: false,
        delete: false,
        hidden: true,
        buffered: false,
    };

    let terminal = TerminalInfo::new(None).configure(config);

    let term_value = terminal
        .configuration
        .get("term")
        .expect("term should be set");
    let term_enabled = unsafe { term_value.as_boolean_unchecked() };

    assert!(
        !term_enabled,
        "term option should be false when enabled=false"
    );

    Ok(())
}

/// Integration test: Verifies configure() with buffered=true sets stdout_buffered to true
#[nvim_oxi::test]
fn terminal_info_configure_buffered_true_sets_stdout_buffered() -> nvim_oxi::Result<()> {
    let config = TerminalConfig {
        enabled: true,
        delete: false,
        hidden: true,
        buffered: true,
    };

    let terminal = TerminalInfo::new(None).configure(config);

    let stdout_buffered = terminal
        .configuration
        .get("stdout_buffered")
        .expect("stdout_buffered should be set");
    let stdout_val = unsafe { stdout_buffered.as_boolean_unchecked() };

    assert!(stdout_val, "stdout_buffered should be true");

    Ok(())
}

/// Integration test: Verifies configure() with buffered=true sets stderr_buffered to true
#[nvim_oxi::test]
fn terminal_info_configure_buffered_true_sets_stderr_buffered() -> nvim_oxi::Result<()> {
    let config = TerminalConfig {
        enabled: true,
        delete: false,
        hidden: true,
        buffered: true,
    };

    let terminal = TerminalInfo::new(None).configure(config);

    let stderr_buffered = terminal
        .configuration
        .get("stderr_buffered")
        .expect("stderr_buffered should be set");
    let stderr_val = unsafe { stderr_buffered.as_boolean_unchecked() };

    assert!(stderr_val, "stderr_buffered should be true");

    Ok(())
}

/// Integration test: Verifies configure() with buffered=false sets stdout_buffered to false
#[nvim_oxi::test]
fn terminal_info_configure_buffered_false_sets_stdout_buffered_to_false() -> nvim_oxi::Result<()> {
    let config = TerminalConfig {
        enabled: true,
        delete: false,
        hidden: true,
        buffered: false,
    };

    let terminal = TerminalInfo::new(None).configure(config);

    let stdout_buffered = terminal
        .configuration
        .get("stdout_buffered")
        .expect("stdout_buffered should be set");
    let stdout_val = unsafe { stdout_buffered.as_boolean_unchecked() };

    assert!(!stdout_val, "stdout_buffered should be false");

    Ok(())
}

/// Integration test: Verifies configure() with buffered=false sets stderr_buffered to false
#[nvim_oxi::test]
fn terminal_info_configure_buffered_false_sets_stderr_buffered_to_false() -> nvim_oxi::Result<()> {
    let config = TerminalConfig {
        enabled: true,
        delete: false,
        hidden: true,
        buffered: false,
    };

    let terminal = TerminalInfo::new(None).configure(config);

    let stderr_buffered = terminal
        .configuration
        .get("stderr_buffered")
        .expect("stderr_buffered should be set");
    let stderr_val = unsafe { stderr_buffered.as_boolean_unchecked() };

    assert!(!stderr_val, "stderr_buffered should be false");

    Ok(())
}

/// Integration test: Verifies run() errors for invalid command without crashing Neovim
#[nvim_oxi::test]
fn terminal_info_run_errors_for_invalid_command() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    let result = terminal.run("nonexistent_command_xyz".to_string(), vec![]);
    assert!(
        matches!(result, Err(hermes::acp::error::Error::Internal(_))),
        "run() with invalid command should return Internal error (not abort Neovim)"
    );

    Ok(())
}

/// Integration test: Verifies stop() succeeds on a running terminal
#[nvim_oxi::test]
fn terminal_info_stop_succeeds_on_running_terminal() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    terminal
        .run("sleep".to_string(), vec!["10".to_string()])
        .expect("Failed to start terminal job");

    let result = terminal.stop();
    assert!(
        result.is_ok(),
        "stop() should succeed on a running terminal: {:?}",
        result
    );

    Ok(())
}

/// Integration test: Verifies delete() succeeds on a terminal with a buffer
#[nvim_oxi::test]
fn terminal_info_delete_succeeds_on_terminal_with_buffer() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    terminal
        .run("echo".to_string(), vec!["hello".to_string()])
        .expect("Failed to start terminal job");

    let result = terminal.delete();
    assert!(
        result.is_ok(),
        "delete() should succeed on a terminal with a buffer: {:?}",
        result
    );

    Ok(())
}

/// Integration test: Verifies delete() removes the terminal buffer
#[nvim_oxi::test]
fn terminal_info_delete_removes_terminal_buffer() -> nvim_oxi::Result<()> {
    let config = TerminalConfig::default();
    let mut terminal = TerminalInfo::new(None).configure(config);

    terminal
        .run("echo".to_string(), vec!["hello".to_string()])
        .expect("Failed to start terminal job");

    let buffer = terminal
        .buffer()
        .expect("Buffer should exist before delete");
    let handle = buffer.handle();

    terminal.delete().expect("delete should succeed");

    let bufs: Vec<_> = nvim_oxi::api::list_bufs().collect();
    assert!(
        !bufs.iter().any(|b| b.handle() == handle),
        "Buffer should no longer exist after terminal deletion"
    );

    Ok(())
}

/// Integration test: Verifies delete() errors when no buffer exists
#[nvim_oxi::test]
fn terminal_info_delete_errors_when_no_buffer() -> nvim_oxi::Result<()> {
    let mut terminal = TerminalInfo::new(None);

    let result = terminal.delete();
    assert!(
        matches!(result, Err(hermes::acp::error::Error::Internal(_))),
        "delete() should return Internal error when there is no buffer"
    );

    Ok(())
}
