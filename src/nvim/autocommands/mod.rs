use crate::apc::{Result, error::Error};
use core::fmt;
use nvim_oxi::libuv::AsyncHandle;
use serde::Serialize;
use std::fmt::{Debug, Display};
use tokio::sync::mpsc::Sender;

mod event;
mod response;

pub use response::*;

#[derive(Clone)]
pub struct AutoCommands {
    group: String,
    handle: AsyncHandle,
    channel: Sender<(Commands, serde_json::Value)>,
}

impl AutoCommands {
    pub fn new(
        group: String,
        channel: Sender<(Commands, serde_json::Value)>,
        handle: AsyncHandle,
    ) -> Self {
        Self {
            group,
            channel,
            handle,
        }
    }

    async fn schedule_autocommand<T: ToString, S: Serialize>(
        &self,
        command: T,
        data: S,
    ) -> Result<()> {
        let serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        self
            .channel
            .send((Commands::AgentConnectionInitialized, serialized))
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
        self.handle.send().map_err(|e|Error::Internal(e.to_string()))
    }
}

#[derive(Debug)]
pub enum Commands {
    AgentConnectionInitialized,
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
