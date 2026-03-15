use std::sync::Arc;

use agent_client_protocol::{
    ReadTextFileRequest, ReadTextFileResponse, RequestPermissionOutcome, RequestPermissionRequest,
    SelectedPermissionOutcome, WriteTextFileRequest, WriteTextFileResponse,
};
use nvim_oxi::conversion::FromObject;
use nvim_oxi::libuv::AsyncHandle;
use tokio::sync::{Mutex, oneshot};
use tracing::error;
use uuid::Uuid;

use crate::acp::Result;
use crate::acp::error::Error;
use crate::utilities::{
    acquire_or_create_buffer, mark_buffer_modified, refresh_view, save_buffer_to_disk,
    show_permission_ui, update_buffer_content,
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
}

#[derive(Clone)]
pub struct Request {
    id: Uuid,
    session_id: String,
    responder: Arc<Mutex<Option<Responder>>>,
    finisher: Arc<AsyncHandle>,
    remove: Arc<tokio::sync::mpsc::Sender<Uuid>>,
}

impl Request {
    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn new(
        session_id: String,
        remove: Arc<tokio::sync::mpsc::Sender<Uuid>>,
        finisher: Arc<AsyncHandle>,
        responder: Responder,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            responder: Arc::new(Mutex::new(Some(responder))),
            finisher,
            remove,
        }
    }

    fn finish(&self) -> Result<()> {
        self.remove.blocking_send(self.id).map_err(|e| {
            Error::Internal(format!(
                "Failed to send finish signal for request '{}', in session '{}': {:?}",
                self.id, self.session_id, e
            ))
        })?;
        self.finisher.send().map_err(|e| {
            Error::Internal(format!(
                "Failed to send finisher signal for request '{}', in session '{}': {:?}",
                self.id, self.session_id, e
            ))
        })?;
        Ok(())
    }

    pub fn is_permission_request(&self) -> bool {
        let responder = self.responder.blocking_lock();
        let is_permission = matches!(*responder, Some(Responder::PermissionResponse(..)));
        drop(responder);
        is_permission
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

    pub fn default(&self, data: serde_json::Value) -> Result<()> {
        if self.is_permission_request() {
            self.ask_user_for_permission(data)?;
        } else {
            match self.get_responder()? {
                Responder::PermissionResponse(..) => {
                    panic!("Permission requests should have been handled in the if branch above")
                }
                Responder::ReadFileResponse(responder, data) => {
                    let content = if let Some(buffer_content) = find_existing_buffer(&data.path) {
                        let start = data.line.unwrap_or(0);
                        let end = data.limit.unwrap_or(
                            buffer_content
                                .line_count()
                                .map_err(|e| Error::Internal(e.to_string()))?
                                as u32,
                        );
                        buffer_content
                            .get_lines((start as usize)..(end as usize), true)
                            .map_err(|e| {
                                error!("Error: {}", e);
                                agent_client_protocol::Error::invalid_params()
                            })
                            .map(|result| {
                                ReadTextFileResponse::new(
                                    result
                                        .map(|s| s.to_string().to_string())
                                        .collect::<String>(),
                                )
                            })
                    } else if let Ok(file_content) = read_file_content(&data.path, data.line, data.limit)
                    {
                        Ok(ReadTextFileResponse::new(file_content))
                    } else {
                        let display_path = data.path.display();
                        error!("Failed to read content for file '{}'", display_path);
                        Err(agent_client_protocol::Error::resource_not_found(Some(
                            display_path.to_string(),
                        )))
                    };
                    responder.send(content).map_err(|e| {
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
            }
            self.finish()?;
        }
        Ok(())
    }
}
