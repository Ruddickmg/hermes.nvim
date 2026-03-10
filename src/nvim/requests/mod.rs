use std::{collections::HashMap, sync::Arc};

use crate::acp::{Result, error::Error};
use agent_client_protocol::{RequestPermissionOutcome, SelectedPermissionOutcome};
use nvim_oxi::conversion::FromObject;
use tokio::sync::{Mutex, oneshot};
use tracing::warn;
use uuid::Uuid;

#[derive(Debug)]
pub enum Responder {
    Cancelled,
    PermissionResponse(oneshot::Sender<RequestPermissionOutcome>),
}

pub struct Request {
    session_id: String,
    responder: Responder,
}

impl Request {
    pub fn new(session_id: String, responder: Responder) -> Self {
        Self {
            session_id,
            responder,
        }
    }
}

pub struct Requests {
    pending: Arc<Mutex<HashMap<Uuid, Request>>>,
}

impl Requests {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Requests {
    fn default() -> Self {
        Self {
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub trait RequestHandler {
    fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()>;
    fn cancel_session_requests(&self, session_id: String) -> Result<()>;
    fn add_request(&self, session_id: String, request_id: Uuid, responder: Responder);
}

impl RequestHandler for Requests {
    fn add_request(&self, session_id: String, request_id: Uuid, responder: Responder) {
        let mut pending = self.pending.blocking_lock();
        pending.insert(request_id, Request::new(session_id, responder));
        drop(pending);
    }

    fn cancel_session_requests(&self, session_id: String) -> Result<()> {
        let mut pending = self.pending.blocking_lock();
        let cancelled = 
            pending
            .extract_if(|_, request| match request.responder {
                Responder::PermissionResponse(_) => request.session_id == session_id,
                _ => false,
            })
            .map(|(id, request)| {
                match request.responder {
                    Responder::PermissionResponse(sender) => {
                        if let Err(e) = sender.send(RequestPermissionOutcome::Cancelled) {
                            return Err(Error::Internal(format!(
                                "Failed to send cancellation for request '{}': {:?}",
                                id, e
                            )));
                        };
                    }
                    _ => panic!("Unexpected responder type when cancelling session requests. This should never happen."),
                };
                Ok((
                    id,
                    Request::new(request.session_id.clone(), Responder::Cancelled),
                ))
            })
            .collect::<Result<Vec<(Uuid, Request)>>>()?;
        pending.extend(cancelled);
        drop(pending);
        Ok(())
    }

    fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()> {
        let mut pending = self.pending.blocking_lock();
        let retrieved = pending.remove(request_id);
        drop(pending);
        if let Some(request) = retrieved {
            match request.responder {
                Responder::Cancelled => warn!(
                    "Request was responded to after it was cancelled. request id: '{}', session id: '{}'",
                    request_id, request.session_id
                ),
                Responder::PermissionResponse(sender) => {
                    let option_id: String = String::from_object(response)
                        .map_err(|e| Error::Internal(e.to_string()))?;
                    let outcome = RequestPermissionOutcome::Selected(
                        SelectedPermissionOutcome::new(option_id),
                    );
                    sender.send(outcome).map_err(|e| {
                        Error::Internal(format!(
                            "Failed to send response for request '{}': {:?}",
                            request_id, e
                        ))
                    })?;
                }
            };
            Ok(())
        } else {
            Err(Error::Internal(format!(
                "No pending request found for ID: '{}'",
                request_id
            )))
        }
    }
}
