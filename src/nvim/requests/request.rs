use agent_client_protocol::{
    RequestPermissionOutcome, RequestPermissionRequest, SelectedPermissionOutcome,
    WriteTextFileResponse,
};
use nvim_oxi::conversion::FromObject;
use tracing::error;
use uuid::Uuid;

use crate::acp::Result;
use crate::acp::error::Error;
use crate::nvim::requests::{
    Responder, acquire_or_create_buffer, mark_buffer_modified, refresh_view, save_buffer_to_disk,
    show_permission_ui, update_buffer_content,
};

#[derive(Debug)]
pub struct Request {
    id: Uuid,
    session_id: String,
    responder: Option<Responder>,
}

impl Request {
    pub fn new(session_id: String, responder: Responder) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            responder: Some(responder),
        }
    }

    pub fn is_permission_request(&self) -> bool {
        matches!(self.responder, Some(Responder::PermissionResponse(..)))
    }

    pub fn is_session(&self, session_id: String) -> bool {
        self.session_id == session_id
    }

    pub fn cancel(&mut self) -> Result<()> {
        let session_id = self.session_id.clone();
        if let Some(Responder::PermissionResponse(sender, ..)) = self.responder.take() {
            if let Err(e) = sender.send(RequestPermissionOutcome::Cancelled) {
                return Err(Error::Internal(format!(
                    "Failed to send cancellation for request '{}', in session '{}': {:?}",
                    self.id, session_id, e
                )));
            };
        }
        Ok(())
    }

    pub fn respond(&mut self, response: nvim_oxi::Object) -> Result<()> {
        let session_id = self.session_id.clone();
        if let Some(responder) = self.responder.take() {
            match responder {
                Responder::WriteFileResponse(sender, _) => {
                    sender.send(WriteTextFileResponse::new()).map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send response for request '{}': {:?}",
                            self.id, e
                        ))
                    })?;
                    Ok(())
                }
                Responder::PermissionResponse(sender, ..) => {
                    let option_id: String = String::from_object(response)
                        .map_err(|e| Error::Internal(e.to_string()))?;
                    let outcome = RequestPermissionOutcome::Selected(
                        SelectedPermissionOutcome::new(option_id),
                    );
                    sender.send(outcome).map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send response for request '{}': {:?}",
                            self.id, e
                        ))
                    })?;
                    Ok(())
                }
                Responder::Cancelled => Err(Error::Internal(format!(
                    "Request was responded to after it was cancelled. request id: '{}', session id: '{}'",
                    self.id, session_id
                ))),
            }
        } else {
            Err(Error::Internal(format!(
                "No responder found for request '{}', in session; {}",
                self.id, session_id
            )))
        }
    }

    pub fn default(mut self, data: serde_json::Value) -> Result<()> {
        if self.is_permission_request() {
            let data: RequestPermissionRequest = serde_json::from_value(data)?;
            let request_id = self.id.to_string();
            let session_id = self.session_id.clone();
            show_permission_ui(&data.options, "Do the thing!", move |option_id| {
                self.respond(option_id.into()).unwrap_or_else(|e| {
                    error!(
                        "Failed to send permission response for request '{}', session '{}': {:?}",
                        request_id, session_id, e
                    )
                });
            })?;
        } else if let Some(responder) = self.responder.take() {
            match responder {
                Responder::PermissionResponse(..) => panic!("Permission requests should have been handled in the if branch above"),
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
                Responder::Cancelled => {}
            }
        }
        Ok(())
    }
}
