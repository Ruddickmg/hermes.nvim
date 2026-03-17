use agent_client_protocol::{
    CreateTerminalRequest, CreateTerminalResponse, KillTerminalCommandRequest,
    KillTerminalCommandResponse, ReadTextFileRequest, ReadTextFileResponse, ReleaseTerminalRequest,
    ReleaseTerminalResponse, RequestPermissionOutcome, RequestPermissionRequest,
    SelectedPermissionOutcome, TerminalOutputRequest, TerminalOutputResponse,
    WaitForTerminalExitRequest, WriteTextFileRequest, WriteTextFileResponse,
};
use nvim_oxi::conversion::FromObject;
use nvim_oxi::Dictionary;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tracing::error;
use uuid::Uuid;

use crate::acp::error::Error;
use crate::acp::Result;
use crate::nvim::autocommands::Commands;
use crate::nvim::terminal::{Terminal, TerminalManager};
use crate::utilities::{
    acquire_or_create_buffer, mark_buffer_modified, refresh_view, save_buffer_to_disk,
    show_permission_ui, update_buffer_content, NvimMessenger, TransmitToNvim,
};
use crate::utilities::{find_existing_buffer, get_permission_prompt, read_file_content};

#[derive(Debug)]
pub enum Responder {
    PermissionResponse(oneshot::Sender<RequestPermissionOutcome>),
    ReadFileResponse(
        oneshot::Sender<agent_client_protocol::Result<ReadTextFileResponse>>,
        ReadTextFileRequest,
    ),
    WriteFileResponse(oneshot::Sender<WriteTextFileResponse>, WriteTextFileRequest),
    TerminalCreate(
        oneshot::Sender<agent_client_protocol::Result<CreateTerminalResponse>>,
        CreateTerminalRequest,
    ),
    TerminalOutput(
        oneshot::Sender<agent_client_protocol::Result<TerminalOutputResponse>>,
        TerminalOutputRequest,
    ),
    TerminalExit(
        oneshot::Sender<(Option<u32>, Option<String>)>,
        WaitForTerminalExitRequest,
    ),
    TerminalRelease(
        oneshot::Sender<agent_client_protocol::Result<ReleaseTerminalResponse>>,
        ReleaseTerminalRequest,
    ),
    TerminalKill(
        oneshot::Sender<agent_client_protocol::Result<KillTerminalCommandResponse>>,
        KillTerminalCommandRequest,
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
    session_id: String,
    responder: Arc<Mutex<Option<Responder>>>,
    remove: NvimMessenger<Uuid>,
}

impl Request {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn new(session_id: String, remove: NvimMessenger<Uuid>, responder: Responder) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            responder: Arc::new(Mutex::new(Some(responder))),
            remove,
        }
    }

    fn finish(&self) -> Result<()> {
        self.remove.blocking_send(self.id).map_err(|e| {
            Error::Internal(format!(
                "Failed to send finish signal for request '{}', in session '{}': {:?}",
                self.id, self.session_id, e
            ))
        })
    }

    pub fn is_permission_request(&self) -> bool {
        let responder = self.responder.blocking_lock();
        let is_permission = matches!(*responder, Some(Responder::PermissionResponse(..)));
        drop(responder);
        is_permission
    }

    pub fn terminal(&self) -> bool {
        let responder = self.responder.blocking_lock();
        let requries = matches!(responder.as_ref(), Some(Responder::TerminalCreate(..)));
        drop(responder);
        requries
    }

    pub fn is_session(&self, session_id: String) -> bool {
        self.session_id == session_id
    }

    fn get_responder(&self) -> Result<Responder> {
        let mut lock = self.responder.blocking_lock();
        let responder = lock.take();
        drop(lock);
        responder.ok_or_else(|| {
            Error::Internal(format!(
                "No responder found for request '{}', in session '{}'",
                self.id, self.session_id
            ))
        })
    }

    pub fn cancel(&self) -> Result<()> {
        let session_id = self.session_id.clone();
        if let Responder::PermissionResponse(sender, ..) = self.get_responder()? {
            sender
                .send(RequestPermissionOutcome::Cancelled)
                .map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send cancellation for request '{}', in session '{}': {:?}",
                        self.id, session_id, e
                    ))
                })?;
        }
        Ok(())
    }

    fn parse_terminal_output_response(
        data: nvim_oxi::Object,
    ) -> agent_client_protocol::Result<(String, bool)> {
        // First, try to parse as a plain String
        match String::from_object(data.clone()) {
            Ok(output) => Ok((output, false)),
            Err(_) => {
                // Not a string, try Dictionary
                let dict = Dictionary::from_object(data)
                    .map_err(|_| agent_client_protocol::Error::invalid_params())?;

                // "output" field is required and must be a String
                let output = dict
                    .get("output")
                    .cloned()
                    .ok_or(agent_client_protocol::Error::invalid_params())
                    .and_then(|o| {
                        String::from_object(o)
                            .map_err(|_| agent_client_protocol::Error::invalid_params())
                    })?;

                // "truncated" field is optional, defaults to false
                let truncated = match dict.get("truncated").cloned() {
                    Some(t) => bool::from_object(t)
                        .map_err(|_| agent_client_protocol::Error::invalid_params())?,
                    None => false,
                };

                Ok((output, truncated))
            }
        }
    }

    pub fn respond(&self, response: nvim_oxi::Object) -> Result<()> {
        match self.get_responder()? {
            Responder::ReadFileResponse(sender, ..) => {
                let outcome =
                    String::from_object(response).map_err(|e| Error::Internal(e.to_string()))?;
                sender
                    .send(Ok(ReadTextFileResponse::new(outcome)))
                    .map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send response for request '{}': {:?}",
                            self.id, e
                        ))
                    })?;
            }
            Responder::WriteFileResponse(sender, _) => {
                sender.send(WriteTextFileResponse::new()).map_err(|e| {
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
                sender.send(outcome).map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
            Responder::TerminalCreate(sender, _) => {
                let result = String::from_object(response)
                    .map(CreateTerminalResponse::new)
                    .map_err(|_| agent_client_protocol::Error::invalid_params());
                sender.send(result).map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send terminal response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
            Responder::TerminalOutput(sender, _) => {
                let result = Self::parse_terminal_output_response(response)
                    .map(|(output, truncated)| TerminalOutputResponse::new(output, truncated));
                sender.send(result).map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send terminal output response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
            Responder::TerminalExit(sender, _) => {
                sender.send((Some(0), Some(String::new()))).map_err(|e| {
                    Error::Internal(format!(
                        "Failed to send terminal exit response for request '{}': {:?}",
                        self.id, e
                    ))
                })?;
            }
            Responder::TerminalRelease(_sender, _) => {
                unimplemented!(
                    "Terminal release is not yet implemented. Request ID '{}'",
                    self.id
                );
            }
            Responder::TerminalKill(_sender, _) => {
                unimplemented!(
                    "Terminal kill is not yet implemented. Request ID '{}'",
                    self.id
                );
            }
        };
        self.finish()
    }

    fn ask_user_for_permission(&self, data: serde_json::Value) -> Result<()> {
        let data: RequestPermissionRequest = serde_json::from_value(data)?;
        let request_id = self.id.to_string();
        let session_id = self.session_id.clone();
        let response_handler = self.clone();
        let prompt = get_permission_prompt();
        show_permission_ui(&data.options, &prompt, move |option_id| {
            response_handler
                .respond(option_id.into())
                .unwrap_or_else(|e| {
                    error!(
                        "Failed to send permission response for request '{}', session '{}': {:?}",
                        request_id, session_id, e
                    )
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

    pub fn default<T: Terminal + Clone>(
        &mut self,
        data: serde_json::Value,
        mut terminal_manager: TerminalManager<T>,
    ) -> Result<()> {
        if self.is_permission_request() {
            self.ask_user_for_permission(data)?;
        } else {
            match self.get_responder()? {
                Responder::PermissionResponse(..) => {
                    panic!("Permission requests should have been handled in the if branch above")
                }
                Responder::ReadFileResponse(responder, data) => {
                    responder.send(Self::read_file(data)).map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send file content response for request '{}': {:?}",
                            self.id, e
                        ))
                    })?;
                }
                Responder::WriteFileResponse(responder, data) => {
                    let path = data.path.clone();
                    let (mut buffer, was_already_open) = acquire_or_create_buffer(&path)?;

                    update_buffer_content(&mut buffer, &data.content)?;

                    if was_already_open {
                        mark_buffer_modified(&buffer)?;
                        // TODO: Make auto-save configurable
                        // if auto_save_enabled {
                        //     save_buffer_to_disk(&buf)?;
                        refresh_view()?;
                    } else {
                        save_buffer_to_disk(&buffer)?;
                    }

                    responder.send(WriteTextFileResponse::new()).map_err(|_| {
                        Error::Internal(
                            "Failed to respond to ACP about successful file write".to_string(),
                        )
                    })?;
                }
                Responder::TerminalCreate(sender, data) => {
                    // Create terminal using the TerminalManager
                    let terminal = T::from_request(data);
                    let terminal_id = terminal.id().to_string();
                    terminal_manager.intitialize_terminal(terminal_id.clone(), terminal);
                    sender
                        .send(Ok(CreateTerminalResponse::new(terminal_id)))
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
                        .ok_or_else(|| agent_client_protocol::Error::resource_not_found(None));
                    sender.send(result).map_err(|e| {
                        Error::Internal(
                            format!("Failed to send terminal output response: {:?}", e,),
                        )
                    })?;
                }
                Responder::TerminalExit(sender, data) => {
                    terminal_manager.notify_when_finished(&data.terminal_id.to_string(), sender)?;
                }
                Responder::TerminalRelease(_sender, _data) => {
                    unimplemented!(
                        "Terminal release is not yet implemented. Request ID '{}'",
                        self.id
                    );
                }
                Responder::TerminalKill(_sender, _data) => {
                    unimplemented!(
                        "Terminal kill is not yet implemented. Request ID '{}'",
                        self.id
                    );
                }
            }
            self.finish()?;
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
}
