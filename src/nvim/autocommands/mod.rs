use crate::{
    acp::{Result, error::Error},
    nvim::GROUP,
};
use core::fmt;
use agent_client_protocol::RequestPermissionOutcome;
use nvim_oxi::{Object, api::opts::ExecAutocmdsOpts, libuv::AsyncHandle};
use serde::{Serialize};
use uuid::Uuid;
use std::{
    collections::HashMap,
    fmt::{Debug, Display}, sync::Arc,
};
use tokio::sync::{Mutex, mpsc::{Sender, channel}, oneshot};
use tracing::{debug, error, instrument, trace};

mod event;
mod response;

pub use response::*;

#[derive(Debug)]
pub enum Responder {
    PermissionResponse(oneshot::Sender<RequestPermissionOutcome>),
}

pub struct AutoCommand {
    handle: AsyncHandle,
    channel: Sender<(String, serde_json::Value)>,
    pending: Arc<Mutex<HashMap<Uuid, Responder>>>,
}

impl AutoCommand {
    #[instrument(level = "trace", skip_all)]
    pub fn new() -> Result<Self> {
        let (sender, mut receiver) = channel::<(String, serde_json::Value)>(100);
        let handle = nvim_oxi::libuv::AsyncHandle::new(move || {
            while let Ok((command, data)) = receiver.try_recv() {
                debug!("Received autocommand: {}, with data: {:#?}", command, data);
                match serde_json::from_value::<Object>(data) {
                    Ok(obj) => {
                        let opts = ExecAutocmdsOpts::builder()
                            .patterns(command.to_string())
                            .data(obj)
                            .group(GROUP)
                            .build();
                        debug!(
                            "Executing autocommand: {} with options: {:#?}",
                            command, opts
                        );
                        if let Err(err) = nvim_oxi::api::exec_autocmds(["User"], &opts) {
                            error!("Error executing autocommand: '{}': {:#?}", command, err);
                        }
                    }
                    Err(e) => error!(
                        "Failed to deserialize autocommand data for '{}': {:#?}",
                        command, e
                    ),
                }
            }
        })
        .map_err(|e| Error::Internal(e.to_string()))?;
        Ok(Self {
            channel: sender,
            handle,
            pending: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    async fn execute_autocommand<C: Debug + ToString, S: Debug + Serialize>(
        &self,
        command: C,
        data: S,
    ) -> Result<()> {
        let serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        debug!("Serialized data: {:#?}", serialized);
        self.channel
            .send((command.to_string(), serialized))
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
        trace!("Triggering callback in Neovim thread");
        self.handle
            .send()
            .map_err(|e| Error::Internal(e.to_string()))
    }

    async fn execute_autocommand_request<
        C: Debug + ToString,
        S: Debug + Serialize,
    >(
        &self,
        command: C,
        data: S,
        sender: Responder,
    ) -> Result<()> {
        let request_id = Uuid::new_v4();
        let mut serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        serialized["requestId"] = serde_json::Value::String(request_id.to_string());
        self.execute_autocommand(command, serialized).await?;
        let mut pending = self.pending.lock().await;
        pending.insert(request_id, sender);
        drop(pending);
        Ok(())
    }

    fn get_response_sender(&self, request_id: &Uuid) -> Option<Responder> {
        let mut pending = self.pending.blocking_lock();
        let sender = pending.remove(request_id);
        drop(pending);
        sender
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Commands {
    ConnectionInitialized,
    CreatedSession,
    AgentPrompted,
    ClientAuthenticated,
    AgentConfigUpdated,
    ModeUpdated,
    LoadedSession,
    ListedSessions,
    ForkedSession,
    ResumedSession,
    SessionModelUpdated,
}

impl Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}
