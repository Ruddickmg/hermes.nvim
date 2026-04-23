use agent_client_protocol::{
    CreateTerminalRequest, CreateTerminalResponse, KillTerminalRequest, KillTerminalResponse,
    ReadTextFileRequest, ReadTextFileResponse, ReleaseTerminalRequest, ReleaseTerminalResponse,
    RequestPermissionOutcome, RequestPermissionRequest, SelectedPermissionOutcome,
    TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
    WriteTextFileRequest, WriteTextFileResponse,
};
use nvim_oxi::conversion::FromObject;
use std::sync::Arc;
use async_lock::Mutex;
use async_channel::Sender;
use tracing::error;
use uuid::Uuid;

use crate::PluginState;
use crate::acp::Result;
use crate::acp::error::Error;
use crate::nvim::autocommands::Commands;
use crate::nvim::configuration::dict_from_object;
use crate::nvim::terminal::{Terminal, TerminalManager, parse_exit_code};
use crate::utilities::{
    NvimMessenger, NvimRuntime, TransmitToNvim, acquire_or_create_buffer, mark_buffer_modified,
    refresh_view, save_buffer_to_disk, show_permission_ui, update_buffer_content,
};
use crate::utilities::{find_existing_buffer, get_permission_prompt, read_file_content};

/// Type alias for oneshot channel sender (async_channel::bounded(1))
pub type OneshotSender<T> = Sender<T>;

#[derive(Debug)]
pub enum Responder {
    PermissionResponse(OneshotSender<RequestPermissionOutcome>),
    ReadFileResponse(
        OneshotSender<agent_client_protocol::Result<ReadTextFileResponse>>,
        ReadTextFileRequest,
    ),
    WriteFileResponse(OneshotSender<WriteTextFileResponse>, WriteTextFileRequest),
    TerminalCreate(
        OneshotSender<Result<CreateTerminalResponse>>,
        CreateTerminalRequest,
    ),
    TerminalOutput(
        OneshotSender<Result<TerminalOutputResponse>>,
        TerminalOutputRequest,
    ),
    TerminalExit(
        OneshotSender<Result<(Option<u32>, Option<String>)>>,
        WaitForTerminalExitRequest,
    ),
    TerminalRelease(
        OneshotSender<Result<ReleaseTerminalResponse>>,
        ReleaseTerminalRequest,
    ),
    TerminalKill(
        OneshotSender<Result<KillTerminalResponse>>,
        KillTerminalRequest,
    ),
}

impl From<Responder> for Commands {
    fn from(responder: Responder) -> Self {
        match responder {
            Responder::TerminalOutput(..) => Commands::TerminalOutput,
            Responder::TerminalKill(..) => Commands::TerminalKill,
            Responder::ReadFileResponse(..) => Commands::ReadTextFile,
            Responder::PermissionResponse(..) => Commands::PermissionRequest,
            Responder::WriteFileResponse(..) => Commands::WriteTextFile,
            Responder::TerminalCreate(..) => Commands::TerminalCreate,
            Responder::TerminalExit(..) => Commands::TerminalExit,
            Responder::TerminalRelease(..) => Commands::TerminalRelease,
        }
    }
}

#[derive(Clone)]
pub struct Request {
    id: Uuid,
    nvim_runtime: NvimRuntime,
    session_id: String,
    responder: Arc<Mutex<Option<Responder>>>,
    remove: NvimMessenger<Uuid>,
    state: Arc<Mutex<PluginState>>,
    is_permission_request: bool,
}

impl Request {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn new(
        session_id: String,
        remove: NvimMessenger<Uuid>,
        responder: Responder,
        state: Arc<Mutex<PluginState>>,
        nvim_runtime: NvimRuntime,
    ) -> Self {
        Self {
            state,
            nvim_runtime,
            id: Uuid::new_v4(),
            session_id,
            is_permission_request: matches!(responder, Responder::PermissionResponse(..)),
            responder: Arc::new(Mutex::new(Some(responder))),
            remove,
        }
    }

    async fn finish(&self) -> Result<()> {
        // Clone what we need for the thread
        let id = self.id;

        self.remove.send(id).await.map_err(|e| {
            Error::Internal(format!(
                "Failed to send finish signal for request '{}', in session '{}': {:?}",
                id, self.session_id, e
            ))
        })
    }

    pub fn is_permission_request(&self) -> bool {
        self.is_permission_request
    }

    pub async fn terminal(&self) -> bool {
        let responder = self.responder.lock().await;
        let is_terminal = matches!(
            responder.as_ref(),
            Some(
                Responder::TerminalCreate(..)
                    | Responder::TerminalOutput(..)
                    | Responder::TerminalExit(..)
                    | Responder::TerminalRelease(..)
                    | Responder::TerminalKill(..)
            )
        );
        drop(responder);
        is_terminal
    }

    pub fn is_session(&self, session_id: String) -> bool {
        self.session_id == session_id
    }

    async fn get_responder(&self) -> Result<Responder> {
        let mut lock = self.responder.lock().await;
        let responder = lock.take();
        drop(lock);
        responder.ok_or_else(|| {
            Error::Internal(format!(
                "No responder found for request '{}', in session '{}'",
                self.id, self.session_id
            ))
        })
    }

    pub async fn cancel(&self) -> Result<()> {
        let session_id = self.session_id.clone();
        if let Responder::PermissionResponse(sender, ..) = self.get_responder().await? {
            sender
                .send(RequestPermissionOutcome::Cancelled)
                .await
                .map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send cancellation for request '{}', in session '{}': {:?}",
                        self.id, session_id, e
                    ))
                })?;
        }
        Ok(())
    }

    fn parse_terminal_output_response(data: nvim_oxi::Object) -> Result<(String, bool)> {
        // First, try to parse as a plain String
        match String::from_object(data.clone()) {
            Ok(output) => Ok((output, false)),
            Err(_) => {
                // Not a string, try Dictionary
                let dict =
                    dict_from_object(data).map_err(|e| Error::InvalidInput(e.to_string()))?;

                // "output" field is required and must be a String
                let output = dict
                    .get("output")
                    .cloned()
                    .ok_or(Error::InvalidInput(
                        "Missing 'output' field in terminal output response".to_string(),
                    ))
                    .and_then(|o| {
                        String::from_object(o).map_err(|e| Error::InvalidInput(e.to_string()))
                    })?;

                // "truncated" field is optional, defaults to false
                let truncated = match dict.get("truncated").cloned() {
                    Some(t) => {
                        bool::from_object(t).map_err(|e| Error::InvalidInput(e.to_string()))?
                    }
                    None => false,
                };

                Ok((output, truncated))
            }
        }
    }

    fn parse_terminal_exit_response(
        data: nvim_oxi::Object,
    ) -> Result<(Option<u32>, Option<String>)> {
        // First, try to parse as a plain String (signal name only)
        match String::from_object(data.clone()) {
            Ok(signal) => Ok(if signal.is_empty() {
                return Err(Error::InvalidInput(
                    "Signal string cannot be empty".to_string(),
                ));
            } else {
                (None, Some(signal))
            }),
            Err(_) => {
                // Not a string, try Integer (exit code)
                match i64::from_object(data.clone()) {
                    Ok(exit_code) => Ok(parse_exit_code(exit_code)),
                    Err(_) => {
                        let dict = dict_from_object(data)
                            .map_err(|e| Error::InvalidInput(e.to_string()))?;

                        // "exitCode" field is optional
                        let exit_code = match dict.get("exitCode").cloned() {
                            Some(ec) => {
                                let code: i64 = i64::from_object(ec)
                                    .map_err(|e| Error::InvalidInput(e.to_string()))?;
                                Some(code)
                            }
                            None => None,
                        };

                        // "signal" field is optional
                        let signal = match dict.get("signal").cloned() {
                            Some(s) => {
                                let sig: String = String::from_object(s)
                                    .map_err(|e| Error::InvalidInput(e.to_string()))?;
                                if sig.is_empty() { None } else { Some(sig) }
                            }
                            None => None,
                        };

                        if signal.is_none() && exit_code.is_none() {
                            Err(Error::InvalidInput(
                                "Terminal exit response must contain at least 'exitCode' or 'signal'".to_string(),
                            ))
                        } else if let Some(code) = exit_code {
                            let (parsed_exit_code, parsed_signal) = parse_exit_code(code);
                            let final_signal = match (signal, parsed_signal) {
                                (Some(explicit_sig), _) => Some(explicit_sig),
                                (None, inferred_sig) => inferred_sig,
                            };
                            Ok((parsed_exit_code, final_signal))
                        } else {
                            Ok((None, signal))
                        }
                    }
                }
            }
        }
    }

    pub async fn respond(&self, response: nvim_oxi::Object) -> Result<()> {
        match self.get_responder().await? {
            Responder::ReadFileResponse(sender, ..) => {
                let outcome =
                    String::from_object(response).map_err(|e| Error::Internal(e.to_string()))?;
                sender
                    .send(Ok(ReadTextFileResponse::new(outcome)))
                    .await
                    .map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send response for request '{}': {:?}",
                            self.id, e
                        ))
                    })?;
            }
            Responder::WriteFileResponse(sender, _) => {
                sender.send(WriteTextFileResponse::new()).await.map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
            Responder::PermissionResponse(sender, ..) => {
                let option_id: String =
                    String::from_object(response).map_err(|e| Error::Internal(e.to_string()))?;
                let outcome = if option_id.is_empty() {
                    RequestPermissionOutcome::Cancelled
                } else {
                    RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id))
                };
                sender.send(outcome).await.map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
            Responder::TerminalCreate(sender, _) => {
                let result = String::from_object(response)
                    .map(CreateTerminalResponse::new)
                    .map_err(|e| Error::InvalidInput(e.to_string()));
                sender.send(result).await.map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send terminal response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
            Responder::TerminalOutput(sender, _) => {
                let result = Self::parse_terminal_output_response(response)
                    .map(|(output, truncated)| TerminalOutputResponse::new(output, truncated));
                sender.send(result).await.map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send terminal output response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
            Responder::TerminalExit(sender, _) => {
                let result = Self::parse_terminal_exit_response(response);
                sender.send(result).await.map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send terminal exit response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
            Responder::TerminalRelease(sender, _) => {
                sender
                    .send(Ok(ReleaseTerminalResponse::new()))
                    .await
                    .map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send terminal release response for request '{}': {:?}",
                            self.id, e
                        ))
                    })?;
            }
            Responder::TerminalKill(sender, _) => {
                sender.send(Ok(KillTerminalResponse::new())).await.map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send terminal kill response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
        };
        self.finish().await
    }

    async fn ask_user_for_permission(&self, data: serde_json::Value) -> Result<()> {
        let data: RequestPermissionRequest = serde_json::from_value(data)?;
        let request_id = self.id.to_string();
        let session_id = self.session_id.clone();
        let response_handler = self.clone();
        let prompt = get_permission_prompt();
        let nvim_runtime = self.nvim_runtime.clone();
        show_permission_ui(&data.options, &prompt, move |option_id| {
            let response_handler = response_handler.clone();
            let request_id = request_id.clone();
            let session_id = session_id.clone();
            nvim_runtime.run(async move {
                response_handler.respond(option_id.into()).await.unwrap_or_else(|e| {
                    error!(
                        "Failed to send permission response for request '{}', session '{}': {:?}",
                        request_id, session_id, e
                    )
                })
            });
        })
    }

    fn read_file(
        data: ReadTextFileRequest,
    ) -> std::result::Result<ReadTextFileResponse, agent_client_protocol::Error> {
        // compensate for 1-based indexing in the ACP spec
        let compensate_for_one_based_index = |n: u32| {
            if n < 1 {
                Err(agent_client_protocol::Error::invalid_params())
            } else {
                Ok(n - 1)
            }
        };
        let line = data.line.map(compensate_for_one_based_index).transpose()?;
        let limit = data.limit.map(compensate_for_one_based_index).transpose()?;

        if let Some(buffer_content) = find_existing_buffer(&data.path) {
            let count = buffer_content
                .line_count()
                .map_err(|_| agent_client_protocol::Error::internal_error())?;
            let start = line.unwrap_or(0);
            let end = limit.unwrap_or(count as u32);
            buffer_content
                .get_lines((start as usize)..(end as usize), true)
                .map_err(|e| {
                    error!("Error: {}", e);
                    agent_client_protocol::Error::invalid_params()
                })
                .map(|result| {
                    // Preserve line breaks by joining with '\n' and add a trailing newline
                    let mut content = result
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>()
                        .join("\n");
                    content.push('\n');
                    ReadTextFileResponse::new(content)
                })
        } else if let Ok(file_content) = read_file_content(&data.path, line, limit) {
            Ok(ReadTextFileResponse::new(file_content))
        } else {
            let display_path = data.path.display();
            error!("Failed to read content for file '{}'", display_path);
            Err(agent_client_protocol::Error::resource_not_found(Some(
                display_path.to_string(),
            )))
        }
    }

    // TODO: return error to both the caller and agent
    pub async fn default<T: Terminal + Clone>(
        &mut self,
        data: serde_json::Value,
        mut terminal_manager: TerminalManager<T>,
    ) -> Result<()> {
        if self.is_permission_request() {
            self.ask_user_for_permission(data).await?;
        } else {
            match self.get_responder().await? {
                Responder::PermissionResponse(..) => {
                    error!("Permission requests should have been handled in the if branch above");
                    return Err(Error::Internal(
                        "Permission request reached default handler unexpectedly".to_string(),
                    ));
                }
                Responder::ReadFileResponse(responder, data) => {
                    responder.send(Self::read_file(data)).await.map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send file content response for request '{}': {:?}",
                            self.id, e
                        ))
                    })?;
                }
                Responder::WriteFileResponse(responder, data) => {
                    let path = data.path.clone();
                    let (mut buffer, was_already_open) = acquire_or_create_buffer(&path)?;
                    let state = self.state.lock().await;
                    let auto_save = state.config.buffer.auto_save;
                    drop(state);

                    update_buffer_content(&mut buffer, &data.content)?;

                    if was_already_open {
                        mark_buffer_modified(&buffer)?;
                        if auto_save {
                            save_buffer_to_disk(&buffer)?;
                        }
                        refresh_view()?;
                    } else {
                        save_buffer_to_disk(&buffer)?;
                    }

                    responder.send(WriteTextFileResponse::new()).await.map_err(|_| {
                        Error::Internal(
                            "Failed to respond to ACP about successful file write".to_string(),
                        )
                    })?;
                }
                Responder::TerminalCreate(sender, data) => {
                    let state = self.state.lock().await;
                    let config = state.config.terminal.clone();
                    drop(state);
                    let mut terminal = T::from_request(data.clone()).configure(config);
                    let terminal_id = terminal.id().to_string();
                    let response = terminal.run(data.command.clone(), data.args);
                    terminal_manager.initialize_terminal(terminal_id.clone(), terminal);
                    sender
                        .send(response.map(|_| CreateTerminalResponse::new(terminal_id)))
                        .await
                        .map_err(|e| {
                            Error::Internal(format!(
                                "Failed to send terminal creation response for request '{}': {:?}",
                                self.id, e
                            ))
                        })?;
                }
                Responder::TerminalOutput(sender, data) => {
                    let result = terminal_manager
                        .get_terminal(&data.terminal_id.to_string())
                        .map(|terminal| {
                            TerminalOutputResponse::new(terminal.content(), terminal.truncated())
                        })
                        .ok_or_else(|| Error::Internal("No terminal found".to_string()));
                    sender.send(result).await.map_err(|e| {
                        Error::Internal(
                            format!("Failed to send terminal output response: {:?}", e,),
                        )
                    })?;
                }
                Responder::TerminalExit(sender, data) => {
                    terminal_manager.notify_when_finished(&data.terminal_id.to_string(), sender)?;
                }
                Responder::TerminalRelease(sender, data) => {
                    let state = self.state.lock().await;
                    let delete_on_release = state.config.terminal.delete;
                    drop(state);
                    let response = terminal_manager
                        .release(&data.terminal_id.to_string())
                        .and_then(|mut terminal| {
                            if delete_on_release {
                                terminal.delete().map(|_| ReleaseTerminalResponse::new())
                            } else {
                                Ok(ReleaseTerminalResponse::new())
                            }
                        });

                    sender.send(response).await.map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send terminal release response for request '{}': {:?}",
                            self.id, e
                        ))
                    })?;
                }
                Responder::TerminalKill(sender, data) => {
                    let response = terminal_manager
                        .kill(&data.terminal_id.to_string())
                        .map(|_| KillTerminalResponse::new());
                    sender.send(response).await.map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send terminal kill response for request '{}': {:?}",
                            self.id, e
                        ))
                    })?;
                }
            }
            self.finish().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvim_oxi::Object;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_terminal_output_accepts_plain_string() {
        let obj = Object::from("hello world");
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("hello world".to_string(), false));
    }

    #[test]
    fn parse_terminal_output_accepts_empty_string() {
        let obj = Object::from("");
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("".to_string(), false));
    }

    #[test]
    fn parse_terminal_output_accepts_dictionary_with_output_field() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from("test output"));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("test output".to_string(), false));
    }

    #[test]
    fn parse_terminal_output_accepts_dictionary_with_truncated_true() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from("test output"));
        dict.insert("truncated", Object::from(true));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("test output".to_string(), true));
    }

    #[test]
    fn parse_terminal_output_accepts_dictionary_with_truncated_false() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from("test output"));
        dict.insert("truncated", Object::from(false));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert_eq!(result.unwrap(), ("test output".to_string(), false));
    }

    #[test]
    fn parse_terminal_output_rejects_missing_output_field() {
        let dict = nvim_oxi::Dictionary::default();
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert!(result.is_err());
    }

    #[test]
    fn parse_terminal_output_rejects_invalid_output_type() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from(123i64));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert!(result.is_err());
    }

    #[test]
    fn parse_terminal_output_rejects_invalid_truncated_type() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("output", Object::from("test"));
        dict.insert("truncated", Object::from("yes"));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_output_response(obj);
        assert!(result.is_err());
    }

    // Tests for parse_terminal_exit_response

    #[test]
    fn parse_terminal_exit_response_accepts_signal_string() {
        let obj = Object::from("SIGTERM");
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (None, Some("SIGTERM".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_exit_code_integer() {
        let obj = Object::from(42i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(42), None));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_exit_code_zero() {
        let obj = Object::from(0i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(0), None));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_negative_signal_number() {
        let obj = Object::from(-9i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (None, Some("SIGKILL".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_exit_code_128_plus_range() {
        // 137 = 128 + 9, should return BOTH exit code AND signal
        let obj = Object::from(137i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(137), Some("SIGKILL".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_dictionary_with_both_fields() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("exitCode", Object::from(9i64));
        dict.insert("signal", Object::from("SIGKILL"));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        // Exit code 9 is in 0-127 range, so parse_exit_code returns (Some(9), None)
        // But explicit signal from dict takes precedence over inferred signal
        assert_eq!(result.unwrap(), (Some(9), Some("SIGKILL".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_dictionary_exit_code_only() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("exitCode", Object::from(42i64));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(42), None));
    }

    #[test]
    fn parse_terminal_exit_response_accepts_dictionary_signal_only() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("signal", Object::from("SIGTERM"));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (None, Some("SIGTERM".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_handles_empty_signal_string() {
        let obj = Object::from("");
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_err());
    }

    #[test]
    fn parse_terminal_exit_response_handles_empty_dictionary_signal() {
        let mut dict = nvim_oxi::Dictionary::default();
        dict.insert("exitCode", Object::from(1i64));
        dict.insert("signal", Object::from(""));
        let obj = Object::from(dict);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        // When signal is empty but exitCode is present, signal becomes None
        assert_eq!(result.unwrap(), (Some(1), None));
    }

    #[test]
    fn parse_terminal_exit_response_handles_unknown_negative_signal() {
        let obj = Object::from(-999i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (None, Some("UNKNOWN(-999)".to_string())));
    }

    #[test]
    fn parse_terminal_exit_response_handles_unknown_128_plus_range() {
        // 255 = 128 + 127, which is in the 128..=255 range
        // 127 is not a standard signal, so map_codes returns None
        let obj = Object::from(255i64);
        let result = Request::parse_terminal_exit_response(obj);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (Some(255), None));
    }

    // Tests for Responder -> Commands conversion
    #[test]
    fn responder_terminal_output_maps_to_terminal_output_command() {
        let (sender, _receiver) = async_channel::bounded::<Result<TerminalOutputResponse>>(1);
        let responder = Responder::TerminalOutput(
            sender,
            TerminalOutputRequest::new(
                agent_client_protocol::SessionId::from("test"),
                agent_client_protocol::TerminalId::from("term-1"),
            ),
        );
        let command: Commands = responder.into();
        assert_eq!(command, Commands::TerminalOutput);
    }

    #[test]
    fn responder_terminal_kill_maps_to_terminal_kill_command() {
        let (sender, _receiver) = async_channel::bounded::<Result<KillTerminalResponse>>(1);
        let responder = Responder::TerminalKill(
            sender,
            KillTerminalRequest::new(
                agent_client_protocol::SessionId::from("test"),
                agent_client_protocol::TerminalId::from("term-1"),
            ),
        );
        let command: Commands = responder.into();
        assert_eq!(command, Commands::TerminalKill);
    }

    #[test]
    fn responder_read_file_maps_to_read_text_file_command() {
        let (sender, _receiver) = async_channel::bounded::<agent_client_protocol::Result<ReadTextFileResponse>>(1);
        let responder = Responder::ReadFileResponse(
            sender,
            ReadTextFileRequest::new(
                agent_client_protocol::SessionId::from("test"),
                std::path::PathBuf::from("/test.txt"),
            ),
        );
        let command: Commands = responder.into();
        assert_eq!(command, Commands::ReadTextFile);
    }

    #[test]
    fn responder_permission_maps_to_permission_request_command() {
        let (sender, _receiver) = async_channel::bounded::<RequestPermissionOutcome>(1);
        let responder = Responder::PermissionResponse(sender);
        let command: Commands = responder.into();
        assert_eq!(command, Commands::PermissionRequest);
    }

    #[test]
    fn responder_write_file_maps_to_write_text_file_command() {
        let (sender, _receiver) = async_channel::bounded::<WriteTextFileResponse>(1);
        let responder = Responder::WriteFileResponse(
            sender,
            WriteTextFileRequest::new(
                agent_client_protocol::SessionId::from("test"),
                std::path::PathBuf::from("/test.txt"),
                "content".to_string(),
            ),
        );
        let command: Commands = responder.into();
        assert_eq!(command, Commands::WriteTextFile);
    }

    #[test]
    fn responder_terminal_create_maps_to_terminal_create_command() {
        let (sender, _receiver) = async_channel::bounded::<Result<CreateTerminalResponse>>(1);
        let responder = Responder::TerminalCreate(
            sender,
            CreateTerminalRequest::new(
                agent_client_protocol::SessionId::from("test"),
                "echo".to_string(),
            ),
        );
        let command: Commands = responder.into();
        assert_eq!(command, Commands::TerminalCreate);
    }

    #[test]
    fn responder_terminal_exit_maps_to_terminal_exit_command() {
        let (sender, _receiver) = async_channel::bounded::<Result<(Option<u32>, Option<String>)>>(1);
        let responder = Responder::TerminalExit(
            sender,
            WaitForTerminalExitRequest::new(
                agent_client_protocol::SessionId::from("test"),
                agent_client_protocol::TerminalId::from("term-1"),
            ),
        );
        let command: Commands = responder.into();
        assert_eq!(command, Commands::TerminalExit);
    }

    #[test]
    fn responder_terminal_release_maps_to_terminal_release_command() {
        let (sender, _receiver) = async_channel::bounded::<Result<ReleaseTerminalResponse>>(1);
        let responder = Responder::TerminalRelease(
            sender,
            ReleaseTerminalRequest::new(
                agent_client_protocol::SessionId::from("test"),
                agent_client_protocol::TerminalId::from("term-1"),
            ),
        );
        let command: Commands = responder.into();
        assert_eq!(command, Commands::TerminalRelease);
    }
}
