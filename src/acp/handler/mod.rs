pub mod client;
pub mod message;
pub mod response;

use crate::{
    PluginState,
    acp::{Result, connection::Assistant},
    nvim::{
        GROUP,
        requests::{RequestHandler, Responder},
    },
    utilities::{NvimMessenger, TransmitToNvim},
};
use nvim_oxi::{Array, Dictionary, Object, api::opts::ExecAutocmdsOpts};
use serde::Serialize;
use std::fmt::Debug;
use std::{fmt::Display, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, error, instrument, warn};

type NvimHandleArgs = (String, serde_json::Value, Option<(Responder, String)>);

pub struct Handler {
    pub channel: NvimMessenger<NvimHandleArgs>,
    pub state: Arc<Mutex<PluginState>>,
}

impl Handler {
    #[instrument(level = "trace", skip_all)]
    pub fn new<R: RequestHandler + 'static>(
        state: Arc<Mutex<PluginState>>,
        requests: Arc<R>,
    ) -> Result<Self> {
        let nvim_requests = requests.clone();
        let channel = NvimMessenger::<NvimHandleArgs>::initialize(
            move |(command, mut data, response_data)| {
                debug!("Received autocommand: {}, with data: {:#?}", command, data);
                if Self::listener_attached(command.to_string()) {
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
                } else if let Some((res, session_id)) = response_data {
                    warn!(
                        "No listener attached for command '{}'. Using default implementation",
                        command
                    );
                    let request_id = nvim_requests.add_request(session_id, res);
                    data["requestId"] = serde_json::Value::String(request_id.to_string());
                    nvim_requests
                        .default_response(&request_id, data)
                        .map_err(|e| {
                            error!(
                                "Failed to send default response for command '{}': {:#?}",
                                command, e
                            )
                        })
                        .ok();
                } else {
                    warn!("No listener attached for command '{}'", command);
                }
            },
        )?;
        Ok(Self { channel, state })
    }

    #[instrument(level = "trace")]
    pub fn listener_attached<S>(pattern: S) -> bool
    where
        S: Display + Debug,
    {
        let command = pattern.to_string();

        // Workaround for nvim-oxi bug: use call_function with properly constructed arguments
        // The builder pattern sends buffer=nil which Neovim rejects

        let mut opts_dict = Dictionary::default();
        opts_dict.insert("group", GROUP);
        opts_dict.insert("event", Array::from((Object::from("User"),)));
        opts_dict.insert("pattern", Array::from((Object::from(command.clone()),)));

        nvim_oxi::api::call_function::<(Object,), Array>("nvim_get_autocmds", (opts_dict.into(),))
            .map(|commands| !commands.is_empty())
            .map_err(|e| {
                error!("Error detecting autocommand: {:?}", e);
                e
            })
            .unwrap_or(false)
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_write(&self) -> bool {
        let state = self.state.lock().await;
        let write_access = state.config.permissions.fs_write_access;
        drop(state);
        write_access
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_read(&self) -> bool {
        let state = self.state.lock().await;
        let read_access = state.config.permissions.fs_read_access;
        drop(state);
        read_access
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_access_terminal(&self) -> bool {
        let state = self.state.lock().await;
        let terminal_access = state.config.permissions.terminal_access;
        drop(state);
        terminal_access
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn get_agent(&self) -> Assistant {
        let state = self.state.lock().await;
        let agent = state.agent.clone();
        drop(state);
        agent
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn set_agent_info(
        &self,
        agent: Assistant,
        info: agent_client_protocol::InitializeResponse,
    ) {
        let mut config = self.state.lock().await;
        config.set_agent_info(agent.clone(), info.clone());
        drop(config);
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_request_permissions(&self) -> bool {
        let config = self.state.lock().await;
        let can_request_permissions = config.config.permissions.can_request_permissions;
        drop(config);
        can_request_permissions
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_receive_notifications(&self) -> bool {
        let config = self.state.lock().await;
        let allow_notifications = config.config.permissions.allow_notifications;
        drop(config);
        allow_notifications
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn execute_autocommand<C: ToString + Debug, S: Serialize + Debug>(
        &self,
        command: C,
        data: S,
    ) -> Result<()> {
        self.send_autocommand(command, data, None).await
    }

    #[instrument(level = "trace", skip(self))]
    async fn send_autocommand<C, S>(
        &self,
        command: C,
        data: S,
        respons_data: Option<(Responder, String)>,
    ) -> Result<()>
    where
        C: ToString + Debug,
        S: Serialize + Debug,
    {
        let serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        debug!("Serialized data: {:#?}", serialized);
        self.channel
            .send((command.to_string(), serialized, respons_data))
            .await
    }

    #[instrument(level = "trace", skip(self, responder))]
    pub async fn execute_autocommand_request<C: ToString + Debug, S: Serialize + Debug>(
        &self,
        session_id: String,
        command: C,
        data: S,
        responder: Responder,
    ) -> Result<()> {
        self.send_autocommand(command, data, Some((responder, session_id)))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::handler::client::tests::MockClient;
    use crate::nvim::state::PluginState;
    use agent_client_protocol::{
        ContentBlock, ContentChunk, SessionNotification, SessionUpdate, TextContent,
    };
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn create_test_notification() -> SessionNotification {
        let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new("test message")));
        SessionNotification::new("session_id", SessionUpdate::UserMessageChunk(chunk))
    }

    #[tokio::test]
    async fn test_can_receive_notifications_returns_true_by_default() {
        let state = Arc::new(Mutex::new(PluginState::default()));
        let handler = Handler::new(state.clone(), MockClient::new());

        let result = handler.can_receive_notifications().await;
        assert!(result, "Should return true by default");
    }

    #[tokio::test]
    async fn test_can_receive_notifications_returns_false_when_disabled() {
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.allow_notifications = false;

        let handler = Handler::new(state.clone(), MockClient::new());

        let result = handler.can_receive_notifications().await;
        assert!(!result, "Should return false when disabled");
    }

    #[tokio::test]
    async fn test_session_notification_permissions_allowed() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));

        let handler = Handler::new(state.clone(), mock.clone());

        let notification = create_test_notification();
        let res: Result<(), agent_client_protocol::Error> =
            handler.session_notification(notification).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_session_notification_calls_handler() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));

        let handler = Handler::new(state.clone(), mock.clone());

        let notification = create_test_notification();
        let _ = handler.session_notification(notification).await;
        assert!(*mock.notification_called.lock().await);
    }

    #[tokio::test]
    async fn test_session_notification_permissions_denied() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.allow_notifications = false;

        let handler = Handler::new(state.clone(), mock.clone());

        let notification = create_test_notification();
        let res = handler.session_notification(notification).await;
        assert_eq!(res, Err(agent_client_protocol::Error::method_not_found()));
    }

    #[tokio::test]
    async fn test_session_notification_permissions_denied_does_not_call_handler() {
        let mock = MockClient::new();
        let state = Arc::new(Mutex::new(PluginState::default()));
        state.lock().await.config.permissions.allow_notifications = false;

        let handler = Handler::new(state.clone(), mock.clone());

        let notification = create_test_notification();
        let _ = handler.session_notification(notification).await;
        assert!(!*mock.notification_called.lock().await);
    }
}
