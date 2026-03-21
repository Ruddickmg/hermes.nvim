pub mod request;
use crate::{
    PluginState,
    acp::{Result, error::Error},
    nvim::terminal::{TerminalInfo, TerminalManager},
    utilities::NvimMessenger,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use tracing::error;
use uuid::Uuid;

pub use request::*;

pub struct Requests {
    pending: Arc<Mutex<HashMap<Uuid, Request>>>,
    nvim_handler: NvimMessenger<Uuid>,
    terminal_manager: TerminalManager<TerminalInfo>,
    state: Arc<Mutex<PluginState>>,
}

impl Requests {
    pub fn new(state: Arc<Mutex<PluginState>>) -> Result<Self> {
        let list = Arc::new(Mutex::new(HashMap::new()));
        let pending = list.clone();
        let nvim_handler = NvimMessenger::initialize(move |id| {
            let mut lock = list.blocking_lock();
            lock.remove(&id);
            drop(lock);
        })?;
        Ok(Self {
            state: state.clone(),
            pending,
            nvim_handler,
            terminal_manager: TerminalManager::new(),
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
        if let Some(mut request) = retrieved {
            request.default(data, self.terminal_manager.clone())
        } else {
            Err(Error::Internal(format!(
                "No pending request found for ID: '{}'",
                request_id
            )))
        }
    }

    fn add_request(&self, session_id: String, responder: Responder) -> Uuid {
        let mut pending = self.pending.blocking_lock();
        let finisher = self.nvim_handler.clone();
        let request = Request::new(session_id, finisher, responder, self.state.clone());
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
