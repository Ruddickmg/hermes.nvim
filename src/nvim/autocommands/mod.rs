use crate::{
    acp::{Result, error::Error},
    nvim::GROUP,
};
use core::fmt;
use nvim_oxi::{Object, api::opts::ExecAutocmdsOpts, libuv::AsyncHandle};
use serde::Serialize;
use std::fmt::{Debug, Display};
use tokio::sync::mpsc::{Sender, channel};
use tracing::{debug, error, instrument};

mod event;
mod response;

pub use response::*;

#[derive(Clone)]
pub struct AutoCommand {
    handle: AsyncHandle,
    channel: Sender<(String, serde_json::Value)>,
}

impl AutoCommand {
    #[instrument(level = "trace", skip_all)]
    pub fn producer(channel: Sender<(String, serde_json::Value)>, handle: AsyncHandle) -> Self {
        Self { channel, handle }
    }

    #[instrument(level = "trace", skip_all)]
    pub fn listener() -> Result<(AsyncHandle, Sender<(String, serde_json::Value)>)> {
        let (sender, mut receiver) = channel::<(String, serde_json::Value)>(100);
        let handle = nvim_oxi::libuv::AsyncHandle::new(move || {
            while let Ok((command, data)) = receiver.try_recv() {
                debug!("Received autocommand: {}, with data: {:?}", command, data);
                match serde_json::from_value::<Object>(data) {
                    Ok(obj) => {
                        let opts = ExecAutocmdsOpts::builder()
                            .patterns(command.to_string())
                            .data(obj)
                            .group(GROUP)
                            .build();
                        debug!("Executing autocommand: {} with options: {:?}", command, opts);
                        if let Err(err) = nvim_oxi::api::exec_autocmds(["User"], &opts) {
                            error!("Error executing autocommand: '{}': {:?}", command, err);
                        }
                    }
                    Err(e) => error!(
                        "Failed to deserialize autocommand data for '{}': {:?}",
                        command, e
                    ),
                }
            }
        })
        .map_err(|e| Error::Internal(e.to_string()))?;
        Ok((handle, sender))
    }

    #[instrument(level = "trace", skip(self))]
    async fn schedule_autocommand<T: Debug + ToString, S: Debug + Serialize>(
        &self,
        command: T,
        data: S,
    ) -> Result<()> {
        let serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        self.channel
            .send((command.to_string(), serialized))
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
        self.handle
            .send()
            .map_err(|e| Error::Internal(e.to_string()))
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

    async fn schedule_autocommand<T: ToString, S: Serialize>(
        &self,
        command: T,
        data: S,
    ) -> Result<()> {
        let serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        self.channel
            .send((command.to_string(), serialized))
            .await
            .map_err(|e| Error::Internal(e.to_string()))?;
        self.handle
            .send()
            .map_err(|e| Error::Internal(e.to_string()))
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
