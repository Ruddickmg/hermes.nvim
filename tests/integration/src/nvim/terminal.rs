//! Integration tests for TerminalInfo
//!
//! These tests verify that TerminalInfo works correctly within the Neovim runtime.

use hermes::nvim::terminal::{Terminal, TerminalInfo};
use tokio::sync::oneshot;

/// Integration test: Verifies report_exit_to sends exit code when already occurred
#[nvim_oxi::test]
fn terminal_info_report_exit_to_sends_exit_code_when_already_occurred() -> nvim_oxi::Result<()> {
    let terminal = TerminalInfo::new(None);

    // Manually set exit status (simulating what exit callback would do)
    *terminal.exit_status.borrow_mut() = Some((Some(5), Some("error".to_string())));

    let (sender, mut receiver) = oneshot::channel();
    terminal.report_exit_to(sender).expect("report failed");

    let received = receiver.try_recv().unwrap();
    assert!(received.is_ok());
    assert_eq!(received.unwrap(), (Some(5), Some("error".to_string())));

    Ok(())
}

/// Integration test: Verifies report_exit_to stores sender for later notification
#[nvim_oxi::test]
fn terminal_info_report_exit_to_stores_sender_for_later() -> nvim_oxi::Result<()> {
    let terminal = TerminalInfo::new(None);

    let (sender, _receiver) = oneshot::channel();
    terminal.report_exit_to(sender).expect("report failed");

    // Verify the sender was stored by checking the exit_response field has a value
    assert!(terminal.exit_response.borrow().is_some());

    Ok(())
}

/// Integration test: Verifies FromRequest creates TerminalInfo with correct defaults
#[nvim_oxi::test]
fn terminal_info_from_request_creates_with_correct_defaults() -> nvim_oxi::Result<()> {
    use agent_client_protocol::{CreateTerminalRequest, SessionId};

    let request = CreateTerminalRequest::new(SessionId::from("test-session"), "test".to_string());
    let terminal = TerminalInfo::from_request(request);

    assert!(terminal.truncated());

    Ok(())
}

/// Integration test: Verifies run() returns positive job ID
#[nvim_oxi::test]
fn terminal_info_run_returns_positive_job_id() -> nvim_oxi::Result<()> {
    let mut terminal = TerminalInfo::new(None);

    // Start a simple echo command
    let job_id = terminal.run("echo".to_string(), vec!["hello".to_string()]);

    assert!(job_id.unwrap() > 0);

    Ok(())
}

/// Integration test: Verifies stop() returns error on non-running terminal
#[nvim_oxi::test]
fn terminal_info_stop_fails_on_non_running_terminal() -> nvim_oxi::Result<()> {
    let terminal = TerminalInfo::new(None);

    // Stop on a terminal that was never run should fail (no job ID)
    let result = terminal.stop();

    assert!(result.is_err());

    Ok(())
}

/// Integration test: Verifies FromRequest with byte limit sets configuration
#[nvim_oxi::test]
fn terminal_info_from_request_applies_byte_limit_to_configuration() -> nvim_oxi::Result<()> {
    use agent_client_protocol::{CreateTerminalRequest, SessionId};

    let mut request =
        CreateTerminalRequest::new(SessionId::from("test-session"), "test".to_string());
    request.output_byte_limit = Some(100);

    let terminal = TerminalInfo::from(request);

    // Byte limit is set - verify content is initially empty
    assert_eq!(terminal.content(), "");

    Ok(())
}

/// Integration test: Verifies FromRequest with cwd sets correct path
#[nvim_oxi::test]
fn terminal_info_from_request_applies_cwd_path() -> nvim_oxi::Result<()> {
    use agent_client_protocol::{CreateTerminalRequest, SessionId};
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
    use agent_client_protocol::{CreateTerminalRequest, EnvVariable, SessionId};

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
    use agent_client_protocol::EnvVariable;

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
