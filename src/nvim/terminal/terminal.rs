use crate::acp::{Result, error::Error};
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
    truncated: Rc<RefCell<Option<bool>>>,
    output: Rc<RefCell<String>>,
    exit_status: Rc<RefCell<Option<(u32, String)>>>,
    exit_response: Rc<RefCell<Option<oneshot::Sender<(u32, String)>>>>,
    buffer: Option<nvim_oxi::api::Buffer>,
    configuration: Dictionary,
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

impl TerminalInfo {
    pub fn new(byte_limit: Option<u64>) -> Self {
        let output = Rc::new(RefCell::new(String::new()));
        let exit_status: Rc<RefCell<Option<(u32, String)>>> = Rc::new(RefCell::new(None));
        let exit_response: Rc<RefCell<Option<oneshot::Sender<(u32, String)>>>> =
            Rc::new(RefCell::new(None));
        let response = exit_response.clone();
        let cloned_output = output.clone();
        let status = exit_status.clone();
        let truncated = Rc::new(RefCell::new(None));
        let is_truncated = truncated.clone();
        let update_content: InputCallback =
            Function::from_fn(move |(_, data): (i64, Vec<String>)| {
                let combined = data.join("\n");
                // Strip ANSI escape codes from terminal output
                let clean = strip_ansi_escapes::strip_str(&combined);
                let mut input = output.borrow_mut();
                input.push_str(&clean);
                if let Some(limit) = byte_limit {
                    let current_bytes = input.len() as u64;
                    if current_bytes > limit {
                        *is_truncated.borrow_mut() = Some(true);
                        let excess = current_bytes - limit;
                        input.drain(0..excess as usize);
                    }
                }
                drop(input);
                Ok(())
            });
        let on_exit: ExitCallback =
            Function::from_fn(move |(_, exit_code, event): (i64, i64, String)| {
                if let Some(sender) = response.take() {
                    sender
                        .send((exit_code as u32, event.to_string()))
                        .map_err(|e| {
                            nvim_oxi::lua::Error::MemoryError(format!(
                                "Error occurred while sending terminal exit notification: {:?}",
                                e
                            ))
                        })
                } else {
                    *exit_status.borrow_mut() = Some((exit_code as u32, event));
                    Ok(())
                }
            });
        let configuration = Self::configuration(update_content, on_exit);
        Self {
            buffer: None,
            truncated,
            configuration,
            id: Uuid::new_v4(),
            job_id: None,
            output: cloned_output,
            exit_status: status,
            exit_response,
        }
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
    fn report_exit_to(&self, sender: oneshot::Sender<(u32, String)>) -> Result<()>;
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

    fn report_exit_to(&self, sender: oneshot::Sender<(u32, String)>) -> Result<()> {
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
        // start the terminal in a hidden buffer (the user can toggle visibility but default to hidden)
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
