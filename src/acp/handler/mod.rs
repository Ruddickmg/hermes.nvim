pub mod builder;
pub mod client;
pub mod response;

use agent_client_protocol::schema::v1::RequestId;
pub use builder::build_client;

use crate::{
    PluginState,
    acp::{
        Result,
        connection::{Assistant, ConnectionManager},
    },
    nvim::{
        GROUP,
        autocommands::Commands,
        requests::{RequestHandler, Responder},
    },
    utilities::{
        NvimMessenger, NvimRuntime, TransmitToNvim, autocmd_listeners_attached, exec_autocmd,
    },
};
use async_lock::Mutex;
use serde::Serialize;
use std::{
    fmt::{Debug, Display},
    rc::Rc,
    sync::Arc,
};
use tracing::{debug, error, instrument, warn};

#[derive(Clone)]
pub enum NvimHandleArgs {
    Event(String, serde_json::Value),
    Request(String, serde_json::Value, Responder, String, RequestId),
    Cancel(RequestId),
}

pub struct Handler {
    pub connection_manager: Arc<Mutex<ConnectionManager>>,
    pub channel: NvimMessenger<NvimHandleArgs>,
    pub state: Arc<Mutex<PluginState>>,
}

impl Handler {
    #[instrument(level = "trace", skip_all)]
    pub fn new<R: RequestHandler + 'static>(
        state: Arc<Mutex<PluginState>>,
        connection_manager: Arc<Mutex<ConnectionManager>>,
        nvim_runtime: NvimRuntime,
        requests: Rc<R>,
    ) -> Result<Self> {
        let channel = NvimMessenger::<NvimHandleArgs>::initialize(nvim_runtime.clone(), {
            let request_tracker = requests.clone();
            move |event| {
                let events = event.clone();
                let nvim_requests = request_tracker.clone();
                async move {
                    let (command, data, request) = match events {
                        NvimHandleArgs::Event(command, data) => (command, data, None),
                        NvimHandleArgs::Request(
                            command,
                            mut data,
                            responder,
                            session_id,
                            agent_request_id,
                        ) => {
                            let request_id = nvim_requests
                                .add(session_id, agent_request_id, responder)
                                .await;
                            data["requestId"] = serde_json::Value::String(request_id.to_string());
                            debug!("Request data: {:#?}", data);
                            (command, data, Some(request_id))
                        }
                        NvimHandleArgs::Cancel(request_id) => {
                            if let Some(request) =
                                nvim_requests.remove_by_request_id(&request_id).await
                            {
                                (
                                    Commands::RequestCancelled.to_string(),
                                    request
                                        .details()
                                        .serialize(serde_json::value::Serializer)
                                        .unwrap(),
                                    None,
                                )
                            } else {
                                error!(
                                    "Agent attempted to cancel request with id: \"{}\" but no matching request was found",
                                    request_id
                                );
                                return;
                            }
                        }
                    };
                    debug!("Received autocommand: {}, with data: {:#?}", command, data);
                    if listener_attached(command.to_string()) {
                        match serde_json::from_value::<nvim_oxi::Object>(data.clone()) {
                            Ok(obj) => {
                                let _ = exec_autocmd(GROUP, "User", &command, obj)
                                    .inspect_err(|err| error!("{}", err));
                            }
                            Err(e) => {
                                error!(
                                    "Failed to convert JSON to Neovim Object for command '{}': {:?}",
                                    command, e
                                );
                            }
                        };
                    } else if let Some(request_id) = request {
                        warn!(
                            "No listener attached for command '{}'. Using default implementation",
                            command
                        );
                        let _ = nvim_requests
                            .default_response(&request_id, data)
                            .await
                            .map_err(|e| {
                                error!(
                                    "Failed to send default response for command '{}': {:#?}",
                                    command, e
                                )
                            });
                    } else {
                        warn!("No listener attached for command '{}'", command);
                    }
                }
            }
        })?;
        Ok(Self {
            channel,
            state,
            connection_manager,
        })
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn set_prompt_id(&self, session_id: String, prompt_id: String) {
        let mut state = self.state.lock().await;
        state.update_session_prompt_id(session_id, prompt_id);
        drop(state);
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn get_prompt_id(&self, session_id: &str) -> Result<String> {
        let mut state = self.state.lock().await;
        let prompt_id = state.get_session_prompt(session_id);
        drop(state);
        Ok(prompt_id)
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
        let agent = state.agent_info.current.clone();
        drop(state);
        agent
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn set_agent_info(
        &self,
        agent: Assistant,
        info: agent_client_protocol::schema::v1::InitializeResponse,
    ) {
        let mut config = self.state.lock().await;
        config.set_agent_info(agent.clone(), info.clone());
        drop(config);
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_request_permissions(&self) -> bool {
        let state = self.state.lock().await;
        let can_request_permissions = state.config.permissions.request_permissions;
        drop(state);
        can_request_permissions
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_receive_notifications(&self) -> bool {
        let config = self.state.lock().await;
        let send_notifications = config.config.permissions.send_notifications;
        drop(config);
        send_notifications
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_request_form_elicitation(&self) -> bool {
        let state = self.state.lock().await;
        let elicitation_form = state.config.permissions.elicitation.form;
        drop(state);
        elicitation_form
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn can_request_url_elicitation(&self) -> bool {
        let state = self.state.lock().await;
        let elicitation_url = state.config.permissions.elicitation.url;
        drop(state);
        elicitation_url
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn elicitation_enabled(&self) -> bool {
        let state = self.state.lock().await;
        let elicitation_url = state.config.permissions.elicitation.url;
        let elicitation_form = state.config.permissions.elicitation.form;
        drop(state);
        elicitation_url || elicitation_form
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn execute_autocommand<C: ToString + Debug, S: Serialize + Debug>(
        &self,
        command: C,
        data: S,
    ) -> Result<()> {
        let serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        debug!("Serialized data: {:#?}", serialized);
        self.channel
            .send(NvimHandleArgs::Event(command.to_string(), serialized))
            .await?;
        Ok(())
    }

    #[instrument(level = "trace", skip(self, responder))]
    pub async fn execute_autocommand_request<C: ToString + Debug, S: Serialize + Debug>(
        &self,
        session_id: String,
        request_id: RequestId,
        command: C,
        data: S,
        responder: Responder,
    ) -> Result<()> {
        let serialized: serde_json::Value = data.serialize(serde_json::value::Serializer)?;
        debug!("Serialized data: {:#?}", serialized);
        self.channel
            .send(NvimHandleArgs::Request(
                command.to_string(),
                serialized,
                responder,
                session_id,
                request_id,
            ))
            .await?;
        Ok(())
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn cancel_request(&self, request_id: &RequestId) -> Result<()> {
        self.channel
            .send(NvimHandleArgs::Cancel(request_id.clone()))
            .await?;
        Ok(())
    }
}

#[instrument(level = "trace")]
fn listener_attached<S>(pattern: S) -> bool
where
    S: Display + Debug,
{
    autocmd_listeners_attached(GROUP, "User", &pattern.to_string())
}
