pub mod request;
use crate::acp::{Result, error::Error};
use nvim_oxi::libuv::AsyncHandle;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{
    Mutex,
    mpsc::{Sender, channel},
};
use tracing::error;
use uuid::Uuid;

pub use request::*;

pub struct Requests {
    pending: Arc<Mutex<HashMap<Uuid, Request>>>,
    finisher: Arc<AsyncHandle>,
    remove: Arc<Sender<Uuid>>,
}

impl Requests {
    pub fn new() -> Result<Self> {
        let list = Arc::new(Mutex::new(HashMap::new()));
        let pending = list.clone();
        let (sender, mut receiver) = channel::<Uuid>(100);
        let finisher = AsyncHandle::new(move || {
            while let Ok(id) = receiver.try_recv() {
                let mut lock = list.blocking_lock();
                lock.remove(&id);
                drop(lock);
            }
        })
        .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(Self {
            pending,
            finisher: Arc::new(finisher),
            remove: Arc::new(sender),
        })
    }
}

pub trait RequestHandler {
    fn default_response(&self, request_id: &Uuid, data: serde_json::Value) -> Result<()>;
    fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()>;
    fn cancel_session_requests(&self, session_id: String) -> Result<()>;
    fn add_request(&self, session_id: String, responder: Responder) -> Uuid;
    fn get_request(&self, request_id: &Uuid) -> Option<Request>;
}

impl RequestHandler for Requests {
    fn default_response(&self, request_id: &Uuid, data: serde_json::Value) -> Result<()> {
        let pending = self.pending.blocking_lock();
        let retrieved = pending.get(request_id).cloned();
        drop(pending);
        if let Some(request) = retrieved {
            request.default(data)
        } else {
            Err(Error::Internal(format!(
                "No pending request found for ID: '{}'",
                request_id
            )))
        }
    }

    fn add_request(&self, session_id: String, responder: Responder) -> Uuid {
        let mut pending = self.pending.blocking_lock();
        let finisher = self.finisher.clone();
        let remove = self.remove.clone();
        let request = Request::new(session_id, remove, finisher, responder);
        let request_id = request.id();
        pending.insert(request_id, request);
        drop(pending);
        request_id
    }

    fn get_request(&self, request_id: &Uuid) -> Option<Request> {
        let pending = self.pending.blocking_lock();
        let request = pending.get(request_id).cloned();
        drop(pending);
        request
    }

    fn cancel_session_requests(&self, session_id: String) -> Result<()> {
        let mut pending = self.pending.blocking_lock();
        pending
            .extract_if(|_, request| {
                request.is_permission_request() && request.is_session(session_id.clone())
            })
            .map(|(_, request)| request.cancel())
            .collect::<Result<Vec<()>>>()?;
        drop(pending);
        Ok(())
    }

    fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()> {
        let pending = self.pending.blocking_lock();
        let retrieved = pending.get(request_id).cloned();
        drop(pending);

        if let Some(request) = retrieved {
            request
                .respond(response)
                .map_err(|e| Error::Internal(format!("Failed to respond to request: {}", e)))?;
            Ok(())
        } else {
            error!("No pending request found for ID: '{}'", request_id);
            Err(Error::Internal(
                "No matching request found: This usually means the request was cancelled before your response could be made."
                    .to_string(),
            ))
        }
    }
}
