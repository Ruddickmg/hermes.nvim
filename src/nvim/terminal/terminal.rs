use crate::{
    acp::{Result, error::Error},
    nvim::{self, terminal::parse_exit_code},
};
use agent_client_protocol::EnvVariable;
use nvim_oxi::{
    Array, Dictionary, Function, Object,
    api::opts::{OptionOpts, OptionScope},
};
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use strip_ansi_escapes;
use tokio::sync::oneshot;
use uuid::Uuid;

pub type ExitStatus = (Option<u32>, Option<String>);

#[derive(Debug, Clone)]
pub struct TerminalInfo {
    id: Uuid,
    job_id: Option<i64>,
    pub truncated: Rc<RefCell<Option<bool>>>,
    pub output: Rc<RefCell<String>>,
    pub exit_status: Rc<RefCell<Option<ExitStatus>>>,
    pub exit_response: Rc<RefCell<Option<oneshot::Sender<Result<ExitStatus>>>>>,
    buffer: Option<nvim_oxi::api::Buffer>,
    pub configuration: Dictionary,
}

impl From<agent_client_protocol::CreateTerminalRequest> for TerminalInfo {
    fn from(request: agent_client_protocol::CreateTerminalRequest) -> Self {
        let mut info = TerminalInfo::new(request.output_byte_limit).environment(request.env);
        if let Some(cwd) = request.cwd {
            info = info.working_directory(cwd);
        }
        info
    }
}

type InputCallback = Function<(i64, Vec<String>), std::result::Result<(), nvim_oxi::lua::Error>>;
type ExitCallback = Function<(i64, i64, String), std::result::Result<(), nvim_oxi::lua::Error>>;

/// Pure function to process terminal input data
/// Joins lines, strips ANSI codes, and applies byte limit truncation
#[tracing::instrument(level = "trace")]
pub fn process_terminal_input(
    data: Vec<String>,
    output: &mut String,
    truncated: &mut Option<bool>,
    byte_limit: Option<u64>,
) {
    let combined = data.join("\n");
    let clean = strip_ansi_escapes::strip_str(&combined);
    output.push_str(&clean);
    if let Some(limit) = byte_limit {
        let current_bytes = output.len() as u64;
        if current_bytes > limit {
            *truncated = Some(true);
            let excess = (current_bytes - limit) as usize;
            let safe_boundary = output.ceil_char_boundary(excess);
            output.drain(0..safe_boundary);
        }
    }
}

/// Pure function to handle terminal exit
/// Maps exit_code (i64 from Neovim) to (Option<u32>, Option<String>) using Unix signal conventions
/// Returns Err if the oneshot channel is closed (recipient dropped)
#[tracing::instrument(level = "trace", skip(exit_status, exit_response))]
pub fn handle_terminal_exit(
    exit_code: i64,
    _event: String,
    exit_status: &mut Option<ExitStatus>,
    exit_response: &mut Option<oneshot::Sender<Result<ExitStatus>>>,
) -> std::result::Result<(), String> {
    // Use signal mapping function to convert exit code
    let data: ExitStatus = parse_exit_code(exit_code);
    if let Some(sender) = exit_response.take() {
        sender.send(Ok(data)).map_err(|_| {
            "Error occurred while sending terminal exit notification: channel closed".to_string()
        })
    } else {
        *exit_status = Some(data);
        Ok(())
    }
}

impl TerminalInfo {
    pub fn new(byte_limit: Option<u64>) -> Self {
        let output = Rc::new(RefCell::new(String::new()));
        let exit_status: Rc<RefCell<Option<ExitStatus>>> = Rc::new(RefCell::new(None));
        let exit_response: Rc<RefCell<Option<oneshot::Sender<Result<ExitStatus>>>>> =
            Rc::new(RefCell::new(None));
        let truncated = Rc::new(RefCell::new(None));

        let update_content =
            Self::create_input_callback(output.clone(), truncated.clone(), byte_limit);
        let on_exit = Self::create_exit_callback(exit_status.clone(), exit_response.clone());
        let configuration = Self::configuration(update_content, on_exit);
        Self {
            buffer: None,
            truncated,
            configuration,
            id: Uuid::new_v4(),
            job_id: None,
            output,
            exit_status,
            exit_response,
        }
    }

    fn create_input_callback(
        output: Rc<RefCell<String>>,
        truncated: Rc<RefCell<Option<bool>>>,
        byte_limit: Option<u64>,
    ) -> InputCallback {
        Function::from_fn(move |(_, data): (i64, Vec<String>)| {
            tracing::trace!("Terminal input callback: {:?}", data);
            let mut input = output.try_borrow_mut().map_err(|e| {
                nvim_oxi::lua::Error::MemoryError(format!("Failed to borrow output buffer: {}", e))
            })?;
            let mut trunc = truncated.try_borrow_mut().map_err(|e| {
                nvim_oxi::lua::Error::MemoryError(format!("Failed to borrow truncated flag: {}", e))
            })?;
            process_terminal_input(data, &mut input, &mut trunc, byte_limit);
            Ok(())
        })
    }

    fn create_exit_callback(
        exit_status: Rc<RefCell<Option<ExitStatus>>>,
        exit_response: Rc<RefCell<Option<oneshot::Sender<Result<ExitStatus>>>>>,
    ) -> ExitCallback {
        Function::from_fn(move |(_, exit_code, event): (i64, i64, String)| {
            tracing::trace!("On exit callback: (code: {}, event: {})", exit_code, event);
            let mut status = exit_status.try_borrow_mut().map_err(|e| {
                nvim_oxi::lua::Error::MemoryError(format!("Failed to borrow exit status: {}", e))
            })?;
            let mut response = exit_response.try_borrow_mut().map_err(|e| {
                nvim_oxi::lua::Error::MemoryError(format!("Failed to borrow exit response: {}", e))
            })?;
            match handle_terminal_exit(exit_code, event, &mut status, &mut response) {
                Ok(()) => Ok(()),
                Err(e) => Err(nvim_oxi::lua::Error::MemoryError(e)),
            }
        })
    }

    pub fn configuration(handle_input: InputCallback, handle_exit: ExitCallback) -> Dictionary {
        let mut opts = Dictionary::new();
        opts.insert("term", true);
        opts.insert("stdout_buffered", true);
        opts.insert("stderr_buffered", true);
        opts.insert("on_stdout", handle_input.clone());
        opts.insert("on_stderr", handle_input);
        opts.insert("on_exit", handle_exit);
        opts
    }

    pub fn working_directory(mut self, cwd: PathBuf) -> Self {
        self.configuration
            .insert("cwd", cwd.to_string_lossy().to_string());
        self
    }

    pub fn environment(mut self, env: Vec<EnvVariable>) -> Self {
        let env_dict = Dictionary::from_iter(env.into_iter().map(|env| (env.name, env.value)));
        self.configuration.insert("env", env_dict);
        self
    }

    fn start_terminal(
        command: String,
        args: Vec<String>,
        configuration: Dictionary,
    ) -> Result<i64> {
        let commands: Array =
            Array::from_iter(vec![command].into_iter().chain(args).map(Object::from));
        nvim_oxi::api::call_function::<(Array, Dictionary), i64>(
            "jobstart",
            (commands, configuration),
        )
        .map_err(|e| Error::Internal(e.to_string()))
    }
}

pub trait Terminal {
    fn run(&mut self, command: String, args: Vec<String>) -> Result<i64>;
    fn content(&self) -> String;
    fn truncated(&self) -> bool;
    fn stop(&self) -> Result<()>;
    fn report_exit_to(&self, sender: oneshot::Sender<Result<ExitStatus>>) -> Result<()>;
    fn id(&self) -> Uuid;
    fn from_request(data: agent_client_protocol::CreateTerminalRequest) -> Self;
}

impl Terminal for TerminalInfo {
    fn from_request(data: agent_client_protocol::CreateTerminalRequest) -> Self {
        Self::from(data)
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn truncated(&self) -> bool {
        self.truncated.borrow().is_some()
    }

    fn report_exit_to(&self, sender: oneshot::Sender<Result<ExitStatus>>) -> Result<()> {
        let mut exit_status = self
            .exit_status
            .try_borrow_mut()
            .map_err(|e| Error::Internal(format!("Failed to borrow exit status: {}", e)))?;
        if let Some(exit_code) = exit_status.take() {
            sender.send(Ok(exit_code)).map_err(|e| {
                Error::Internal(format!(
                    "Error occurred while sending terminal exit notification: {:?}",
                    e
                ))
            })
        } else {
            drop(exit_status);
            let mut exit_response = self
                .exit_response
                .try_borrow_mut()
                .map_err(|e| Error::Internal(format!("Failed to borrow exit response: {}", e)))?;
            *exit_response = Some(sender);
            Ok(())
        }
    }

    fn run(&mut self, command: String, args: Vec<String>) -> Result<i64> {
        let buffer =
            nvim_oxi::api::create_buf(false, true).map_err(|e| Error::Internal(e.to_string()))?;
        let configuration = self.configuration.clone();
        let job_id = buffer
            .call(|_| Self::start_terminal(command, args, configuration))
            .map_err(|e| Error::Internal(e.to_string()))?;
        let opts = OptionOpts::builder()
            .buffer(buffer.clone())
            .scope(OptionScope::Local)
            .build();
        nvim_oxi::api::set_option_value("buftype", "terminal", &opts);
        nvim_oxi::api::set_option_value("swapfile", false, &opts);
        nvim_oxi::api::set_option_value("bufhidden", "hide", &opts);
        nvim_oxi::api::set_option_value("nonumber", "norelativenumber", &opts);
        nvim_oxi::api::set_option_value("signcolumn", "no", &opts);
        nvim_oxi::api::set_option_value("wrap", "nofoldcolumn", &opts);
        nvim_oxi::api::set_option_value("scrollback", "10000", &opts);
        nvim_oxi::api::set_option_value("nomodified", "true", &opts);
        self.job_id = Some(job_id as i64);
        self.buffer = Some(buffer);
        Ok(job_id)
    }

    fn content(&self) -> String {
        self.output.borrow().clone()
    }

    fn stop(&self) -> Result<()> {
        if let Some(id) = self.job_id {
            nvim_oxi::api::call_function::<(i64,), ()>("jobstop", (id,))
                .map_err(|e| Error::Internal(e.to_string()))
        } else {
            Err(Error::Internal(
                "Cannot stop terminal: job ID not found".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // === Tests for process_terminal_input ===

    #[test]
    fn process_input_joins_lines_with_newlines() {
        let mut output = String::new();
        let mut truncated = None;

        process_terminal_input(
            vec!["line1".to_string(), "line2".to_string()],
            &mut output,
            &mut truncated,
            None,
        );

        assert_eq!(output, "line1\nline2");
    }

    #[test]
    fn process_input_strips_ansi_codes() {
        let mut output = String::new();
        let mut truncated = None;
        let ansi_string = "\x1b[31mred\x1b[0m \x1b[1mbold\x1b[0m text";

        process_terminal_input(
            vec![ansi_string.to_string()],
            &mut output,
            &mut truncated,
            None,
        );

        assert_eq!(output, "red bold text");
    }

    #[test]
    fn process_input_truncates_when_over_byte_limit() {
        let mut output = String::new();
        let mut truncated = None;

        process_terminal_input(
            vec!["this is a long string".to_string()],
            &mut output,
            &mut truncated,
            Some(10),
        );

        assert_eq!(output.len(), 10);
    }

    #[test]
    fn process_input_sets_truncated_flag_when_over_limit() {
        let mut output = String::new();
        let mut truncated = None;

        process_terminal_input(
            vec!["too long".to_string()],
            &mut output,
            &mut truncated,
            Some(5),
        );

        assert_eq!(truncated, Some(true));
    }

    #[test]
    fn process_input_does_not_truncate_when_under_limit() {
        let mut output = String::new();
        let mut truncated = None;

        process_terminal_input(
            vec!["short".to_string()],
            &mut output,
            &mut truncated,
            Some(100),
        );

        assert_eq!(output, "short");
        assert_eq!(truncated, None);
    }

    #[test]
    fn process_input_does_not_truncate_when_no_limit() {
        let mut output = String::new();
        let mut truncated = None;
        let long_string = "a".repeat(10000);

        process_terminal_input(vec![long_string.clone()], &mut output, &mut truncated, None);

        assert_eq!(output.len(), 10000);
        assert_eq!(truncated, None);
    }

    #[test]
    fn process_input_truncates_multibyte_utf8_safely() {
        // Test that truncation doesn't panic when excess falls in middle of multi-byte char
        let mut output = String::new();
        let mut truncated = None;
        let emoji_string = "Hello 🎉 World 🎉 End".to_string();

        process_terminal_input(vec![emoji_string], &mut output, &mut truncated, Some(15));

        // Should not panic and should produce valid UTF-8
        assert!(
            std::str::from_utf8(output.as_bytes()).is_ok(),
            "Output must be valid UTF-8"
        );
    }

    #[test]
    fn process_input_truncates_multibyte_utf8_sets_truncated_flag() {
        let mut output = String::new();
        let mut truncated = None;
        let emoji_string = "Hello 🎉 World 🎉 End".to_string();

        process_terminal_input(vec![emoji_string], &mut output, &mut truncated, Some(15));

        assert_eq!(truncated, Some(true));
    }

    #[test]
    fn process_input_truncates_multibyte_utf8_respects_limit() {
        let mut output = String::new();
        let mut truncated = None;
        let emoji_string = "Hello 🎉 World 🎉 End".to_string();

        process_terminal_input(vec![emoji_string], &mut output, &mut truncated, Some(15));

        // After truncation, string should be at or under the limit
        assert!(output.len() <= 15);
    }

    #[test]
    fn process_input_handles_exact_char_boundary() {
        // Test when excess falls exactly on a char boundary
        let mut output = String::new();
        let mut truncated = None;
        let test_string = "Test 🎉 More text here".to_string();

        process_terminal_input(vec![test_string], &mut output, &mut truncated, Some(9));

        assert!(
            std::str::from_utf8(output.as_bytes()).is_ok(),
            "Output must be valid UTF-8"
        );
    }

    #[test]
    fn process_input_handles_exact_char_boundary_sets_truncated_flag() {
        let mut output = String::new();
        let mut truncated = None;
        let test_string = "Test 🎉 More text here".to_string();

        process_terminal_input(vec![test_string], &mut output, &mut truncated, Some(9));

        assert_eq!(truncated, Some(true));
    }

    #[test]
    fn process_input_handles_emoji_only_content() {
        // Test with content that is all multi-byte characters
        let mut output = String::new();
        let mut truncated = None;
        let emoji_string = "🎉🎊🎁🎄🎃".to_string();

        process_terminal_input(vec![emoji_string], &mut output, &mut truncated, Some(10));

        assert!(
            std::str::from_utf8(output.as_bytes()).is_ok(),
            "Output must be valid UTF-8"
        );
    }

    #[test]
    fn process_input_handles_emoji_only_content_sets_truncated_flag() {
        let mut output = String::new();
        let mut truncated = None;
        let emoji_string = "🎉🎊🎁🎄🎃".to_string();

        process_terminal_input(vec![emoji_string], &mut output, &mut truncated, Some(10));

        assert_eq!(truncated, Some(true));
    }

    #[test]
    fn process_input_handles_emoji_only_content_respects_limit() {
        let mut output = String::new();
        let mut truncated = None;
        let emoji_string = "🎉🎊🎁🎄🎃".to_string();

        process_terminal_input(vec![emoji_string], &mut output, &mut truncated, Some(10));

        assert!(output.len() <= 10);
    }

    // === Tests for handle_terminal_exit ===

    #[test]
    fn handle_exit_sends_immediately_when_sender_available() {
        let mut exit_status: Option<ExitStatus> = None;
        let mut exit_response: Option<oneshot::Sender<Result<ExitStatus>>>;

        let (sender, mut receiver) = oneshot::channel();
        exit_response = Some(sender);

        let _result = handle_terminal_exit(
            42,
            "ignored".to_string(),
            &mut exit_status,
            &mut exit_response,
        );

        // Exit code 42 is normal (0-127 range), no signal
        let received = receiver.try_recv().unwrap();
        assert!(received.is_ok());
        assert_eq!(received.unwrap(), (Some(42), None));
        assert!(exit_status.is_none());
    }

    #[test]
    fn handle_exit_stores_status_when_no_sender() {
        let mut exit_status: Option<ExitStatus> = None;
        let mut exit_response: Option<oneshot::Sender<Result<ExitStatus>>> = None;

        let result = handle_terminal_exit(
            1,
            "ignored".to_string(),
            &mut exit_status,
            &mut exit_response,
        );

        assert!(result.is_ok());
        // Exit code 1 is normal (0-127 range), no signal
        assert_eq!(exit_status, Some((Some(1), None)));
        assert!(exit_response.is_none());
    }

    #[test]
    fn handle_exit_maps_negative_signal_code_to_signal_name() {
        let mut exit_status: Option<ExitStatus> = None;
        let mut exit_response: Option<oneshot::Sender<Result<ExitStatus>>> = None;

        let result = handle_terminal_exit(
            -15,
            "ignored".to_string(),
            &mut exit_status,
            &mut exit_response,
        );

        assert!(result.is_ok());
        // -15 maps to SIGTERM
        assert_eq!(exit_status, Some((None, Some("SIGTERM".to_string()))));
    }

    #[test]
    fn handle_exit_maps_exit_code_128_plus_range_to_signal() {
        let mut exit_status: Option<ExitStatus> = None;
        let mut exit_response: Option<oneshot::Sender<Result<ExitStatus>>> = None;

        // 137 = 128 + 9 = SIGKILL
        // Returns BOTH exit code AND signal
        let result = handle_terminal_exit(
            137,
            "ignored".to_string(),
            &mut exit_status,
            &mut exit_response,
        );

        assert!(result.is_ok());
        // Exit code 137 AND signal SIGKILL
        assert_eq!(exit_status, Some((Some(137), Some("SIGKILL".to_string()))));
    }

    #[test]
    fn handle_exit_maps_sigkill_negative_code() {
        let mut exit_status: Option<ExitStatus> = None;
        let mut exit_response: Option<oneshot::Sender<Result<ExitStatus>>> = None;

        let result = handle_terminal_exit(
            -9,
            "ignored".to_string(),
            &mut exit_status,
            &mut exit_response,
        );

        assert!(result.is_ok());
        assert_eq!(exit_status, Some((None, Some("SIGKILL".to_string()))));
    }

    #[test]
    fn handle_exit_maps_sigint_negative_code() {
        let mut exit_status: Option<ExitStatus> = None;
        let mut exit_response: Option<oneshot::Sender<Result<ExitStatus>>> = None;

        let result = handle_terminal_exit(
            -2,
            "ignored".to_string(),
            &mut exit_status,
            &mut exit_response,
        );

        assert!(result.is_ok());
        assert_eq!(exit_status, Some((None, Some("SIGINT".to_string()))));
    }

    #[test]
    fn handle_exit_maps_unknown_signal_to_generic_name() {
        let mut exit_status: Option<ExitStatus> = None;
        let mut exit_response: Option<oneshot::Sender<Result<ExitStatus>>> = None;

        let result = handle_terminal_exit(
            -999,
            "ignored".to_string(),
            &mut exit_status,
            &mut exit_response,
        );

        assert!(result.is_ok());
        assert_eq!(exit_status, Some((None, Some("UNKNOWN(-999)".to_string()))));
    }

    #[test]
    fn handle_exit_maps_exit_code_zero_to_normal_exit() {
        let mut exit_status: Option<ExitStatus> = None;
        let mut exit_response: Option<oneshot::Sender<Result<ExitStatus>>> = None;

        let result = handle_terminal_exit(
            0,
            "ignored".to_string(),
            &mut exit_status,
            &mut exit_response,
        );

        assert!(result.is_ok());
        assert_eq!(exit_status, Some((Some(0), None)));
    }
}
