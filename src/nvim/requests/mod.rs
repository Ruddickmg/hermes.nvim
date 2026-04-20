pub mod request;
use crate::{
    PluginState,
    acp::{Result, error::Error},
    nvim::terminal::{TerminalInfo, TerminalManager},
    utilities::{NvimMessenger, NvimRuntime},
};
use async_trait::async_trait;
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
    nvim_runtime: NvimRuntime,
}

impl Requests {
    pub fn new(nvim_runtime: NvimRuntime, state: Arc<Mutex<PluginState>>) -> Result<Self> {
        let list = Arc::new(Mutex::new(HashMap::new()));
        let pending = list.clone();
        let nvim_handler = NvimMessenger::initialize(nvim_runtime.clone(), move |id| {
            let list = list.clone();
            async move {
                let mut lock = list.lock().await;
                lock.remove(&id);
                drop(lock);
            }
        })?;
        Ok(Self {
            nvim_runtime,
            state: state.clone(),
            pending,
            nvim_handler,
            terminal_manager: TerminalManager::new(),
        })
    }
}

#[async_trait(?Send)]
pub trait RequestHandler {
    async fn default_response(&self, request_id: &Uuid, data: serde_json::Value) -> Result<()>;
    async fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()>;
    async fn cancel_session_requests(&self, session_id: String) -> Result<()>;
    async fn add_request(&self, session_id: String, responder: Responder) -> Uuid;
    async fn get_request(&self, request_id: &Uuid) -> Option<Request>;
}

#[async_trait(?Send)]
impl RequestHandler for Requests {
    async fn default_response(&self, request_id: &Uuid, data: serde_json::Value) -> Result<()> {
        let pending = self.pending.lock().await;
        let retrieved = pending.get(request_id).cloned();
        drop(pending);
        if let Some(mut request) = retrieved {
            request.default(data, self.terminal_manager.clone()).await
        } else {
            Err(Error::Internal(format!(
                "No pending request found for ID: '{}'",
                request_id
            )))
        }
    }

    async fn add_request(&self, session_id: String, responder: Responder) -> Uuid {
        let mut pending = self.pending.lock().await;
        let finisher = self.nvim_handler.clone();
        let request = Request::new(
            session_id,
            finisher,
            responder,
            self.state.clone(),
            self.nvim_runtime.clone(),
        );
        let request_id = request.id();
        pending.insert(request_id, request);
        drop(pending);
        request_id
    }

    async fn get_request(&self, request_id: &Uuid) -> Option<Request> {
        let pending = self.pending.lock().await;
        let request = pending.get(request_id).cloned();
        drop(pending);
        request
    }

    async fn cancel_session_requests(&self, session_id: String) -> Result<()> {
        let mut pending = self.pending.lock().await;
        futures::future::try_join_all(
            pending
                .extract_if(|_, request| {
                    request.is_permission_request() && request.is_session(session_id.clone())
                })
                .map(|(_, request)| async move { request.cancel().await }),
        )
        .await?;
        drop(pending);
        Ok(())
    }

    async fn handle_response(&self, request_id: &Uuid, response: nvim_oxi::Object) -> Result<()> {
        let pending = self.pending.lock().await;
        let retrieved = pending.get(request_id).cloned();
        drop(pending);

        if let Some(request) = retrieved {
            request
                .respond(response)
                .await
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
