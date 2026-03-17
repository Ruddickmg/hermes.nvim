use crate::acp::{error::Error, Result};
use agent_client_protocol::EnvVariable;
use nvim_oxi::{Array, Dictionary, Function, Object};
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use strip_ansi_escapes;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TerminalInfo {
    id: Uuid,
    job_id: Option<i64>,
    pub truncated: Rc<RefCell<Option<bool>>>,
    pub output: Rc<RefCell<String>>,
    pub exit_status: Rc<RefCell<Option<(Option<u32>, Option<String>)>>>,
    pub exit_response: Rc<RefCell<Option<oneshot::Sender<(Option<u32>, Option<String>)>>>>,
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
            let excess = current_bytes - limit;
            output.drain(0..excess as usize);
        }
    }
}

/// Pure function to handle terminal exit
/// Parses exit_code (i64 from Neovim) to Option<u32> (None if negative)
/// Parses event String to Option<String> (None if empty)
/// Returns Err if the oneshot channel is closed (recipient dropped)
pub fn handle_terminal_exit(
    exit_code: i64,
    event: String,
    exit_status: &mut Option<(Option<u32>, Option<String>)>,
    exit_response: &mut Option<oneshot::Sender<(Option<u32>, Option<String>)>>,
) -> std::result::Result<(), String> {
    // Parse exit_code: None if negative, else Some(exit_code as u32)
    let parsed_exit = if exit_code < 0 {
        None
    } else {
        Some(exit_code as u32)
    };

    // Parse event: None if empty, else Some(event)
    let parsed_event = if event.is_empty() { None } else { Some(event) };

    if let Some(sender) = exit_response.take() {
        sender.send((parsed_exit, parsed_event)).map_err(|_| {
            "Error occurred while sending terminal exit notification: channel closed".to_string()
        })
    } else {
        *exit_status = Some((parsed_exit, parsed_event));
        Ok(())
    }
}

impl TerminalInfo {
    pub fn new(byte_limit: Option<u64>) -> Self {
        let output = Rc::new(RefCell::new(String::new()));
        let exit_status: Rc<RefCell<Option<(Option<u32>, Option<String>)>>> =
            Rc::new(RefCell::new(None));
        let exit_response: Rc<RefCell<Option<oneshot::Sender<(Option<u32>, Option<String>)>>>> =
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
            let mut input = output.borrow_mut();
            let mut trunc = truncated.borrow_mut();
            process_terminal_input(data, &mut *input, &mut *trunc, byte_limit);
            Ok(())
        })
    }

    fn create_exit_callback(
        exit_status: Rc<RefCell<Option<(Option<u32>, Option<String>)>>>,
        exit_response: Rc<RefCell<Option<oneshot::Sender<(Option<u32>, Option<String>)>>>>,
    ) -> ExitCallback {
        Function::from_fn(move |(_, exit_code, event): (i64, i64, String)| {
            let mut status = exit_status.borrow_mut();
            let mut response = exit_response.borrow_mut();
            match handle_terminal_exit(exit_code, event, &mut *status, &mut *response) {
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
    fn close(&self) -> Result<()>;
    fn report_exit_to(&self, sender: oneshot::Sender<(Option<u32>, Option<String>)>) -> Result<()>;
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

    fn report_exit_to(&self, sender: oneshot::Sender<(Option<u32>, Option<String>)>) -> Result<()> {
        if let Some(exit_code) = self.exit_status.borrow_mut().take() {
            sender.send(exit_code).map_err(|e| {
                Error::Internal(format!(
                    "Error occurred while sending terminal exit notification: {:?}",
                    e
                ))
            })
        } else {
            *self.exit_response.borrow_mut() = Some(sender);
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
        self.job_id = Some(job_id as i64);
        self.buffer = Some(buffer);
        Ok(job_id)
    }

    fn content(&self) -> String {
        self.output.borrow().clone()
    }

    fn close(&self) -> Result<()> {
        self.job_id
            .map(|id| nvim_oxi::api::call_function::<(i64,), ()>("jobstop", (id,)))
            .transpose()
            .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(())
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

    // === Tests for handle_terminal_exit ===

    #[test]
    fn handle_exit_sends_immediately_when_sender_available() {
        let mut exit_status: Option<(Option<u32>, Option<String>)> = None;
        let mut exit_response: Option<oneshot::Sender<(Option<u32>, Option<String>)>> = None;

        let (sender, mut receiver) = oneshot::channel();
        exit_response = Some(sender);

        let _result =
            handle_terminal_exit(42, "exit".to_string(), &mut exit_status, &mut exit_response);

        assert_eq!(
            receiver.try_recv().unwrap(),
            (Some(42), Some("exit".to_string()))
        );
        assert!(exit_status.is_none());
    }

    #[test]
    fn handle_exit_stores_status_when_no_sender() {
        let mut exit_status: Option<(Option<u32>, Option<String>)> = None;
        let mut exit_response: Option<oneshot::Sender<(Option<u32>, Option<String>)>> = None;

        let result =
            handle_terminal_exit(1, "error".to_string(), &mut exit_status, &mut exit_response);

        assert!(result.is_ok());
        assert_eq!(exit_status, Some((Some(1), Some("error".to_string()))));
        assert!(exit_response.is_none());
    }

    #[test]
    fn handle_exit_parses_negative_code_as_none() {
        let mut exit_status: Option<(Option<u32>, Option<String>)> = None;
        let mut exit_response: Option<oneshot::Sender<(Option<u32>, Option<String>)>> = None;

        let result = handle_terminal_exit(
            -1,
            "SIGTERM".to_string(),
            &mut exit_status,
            &mut exit_response,
        );

        assert!(result.is_ok());
        assert_eq!(exit_status, Some((None, Some("SIGTERM".to_string()))));
    }

    #[test]
    fn handle_exit_parses_empty_event_as_none() {
        let mut exit_status: Option<(Option<u32>, Option<String>)> = None;
        let mut exit_response: Option<oneshot::Sender<(Option<u32>, Option<String>)>> = None;

        let result = handle_terminal_exit(0, "".to_string(), &mut exit_status, &mut exit_response);

        assert!(result.is_ok());
        assert_eq!(exit_status, Some((Some(0), None)));
    }
}
