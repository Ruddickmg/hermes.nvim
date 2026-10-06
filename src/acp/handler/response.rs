use agent_client_protocol::schema::v1::{
    AuthenticateResponse, CloseSessionResponse, DeleteSessionResponse, ExtResponse,
    ForkSessionResponse, InitializeResponse, ListSessionsResponse, LoadSessionResponse,
    LogoutResponse, NewSessionResponse, PromptResponse, ResumeSessionResponse, SessionConfigKind,
    SessionConfigOption, SessionConfigOptionCategory, SetSessionConfigOptionResponse,
    SetSessionModeResponse,
};
use serde::Serialize;
use tracing::instrument;

use crate::Handler;
use crate::acp::connection::Assistant;
use crate::acp::error::Error;
use crate::api::authenticate::TerminalAuthenticateRequest;
use crate::nvim::autocommands::Commands;

#[derive(Serialize, Debug)]
struct WithSessionId<T: Serialize> {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(flatten)]
    inner: T,
}

impl Handler {
    #[instrument(level = "trace", skip(self))]
    pub async fn initialized(
        &self,
        agent: &Assistant,
        info: InitializeResponse,
    ) -> Result<(), Error> {
        self.set_agent_info(agent.clone(), info.clone()).await;

        let value = serde_json::json!({
            "protocolVersion": info.protocol_version.as_u16(),
            "agentCapabilities": {
                "loadSession": info.agent_capabilities.load_session,
                "promptCapabilities": {
                    "image": info.agent_capabilities.prompt_capabilities.image,
                    "audio": info.agent_capabilities.prompt_capabilities.audio,
                    "embeddedContext": info.agent_capabilities.prompt_capabilities.embedded_context,
                },
                "mcpCapabilities": {
                    "http": info.agent_capabilities.mcp_capabilities.http,
                    "sse": info.agent_capabilities.mcp_capabilities.sse,
                    "acp": info.agent_capabilities.mcp_capabilities.acp,
                },
                "sessionCapabilities": {
                    "list": info.agent_capabilities.session_capabilities.list.is_some(),
                    "fork": info.agent_capabilities.session_capabilities.fork.is_some(),
                    "resume": info.agent_capabilities.session_capabilities.resume.is_some(),
                    "close": info.agent_capabilities.session_capabilities.close.is_some(),
                    "additionalDirectories": info.agent_capabilities.session_capabilities.additional_directories.is_some(),
                    "delete": info.agent_capabilities.session_capabilities.delete.is_some(),
                },
                "auth": {
                    "logout": info.agent_capabilities.auth.logout.is_some()
                },
            },
            // TODO: handle each type of auth method
            "authMethods": info.auth_methods.iter().map(|m| serde_json::json!({
                "id": m.id().0,
                "name": m.name(),
                "description": m.description(),
            })).collect::<Vec<_>>(),
            "agentInfo": info.agent_info.map(|i| serde_json::json!({
                "name": i.name,
                "version": i.version,
                "title": i.title,
            })),
        });
        self.execute_autocommand(Commands::ConnectionInitialized, value)
            .await
    }
    #[instrument(level = "trace", skip(self))]
    pub async fn session_created(&self, session: NewSessionResponse) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        state.set_session_info(&session);
        drop(state);
        self.execute_autocommand(Commands::SessionCreated, session)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn prompted(&self, response: PromptResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::Prompted, response).await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn authenticated(&self, response: AuthenticateResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::Authenticated, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn terminal_authentication(
        &self,
        response: TerminalAuthenticateRequest,
    ) -> Result<(), Error> {
        self.execute_autocommand(Commands::TerminalAuthentication, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn logged_out(&self, response: LogoutResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::LoggedOut, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn config_option_set(
        &self,
        session_id: &str,
        response: SetSessionConfigOptionResponse,
    ) -> Result<(), Error> {
        let model_config_options = response.config_options.clone();
        let model_config_ref = &model_config_options;

        let futures = response
            .config_options
            .iter()
            .filter_map(|option| {
                let category = option.category.clone()?;
                let SessionConfigKind::Select(select) = &option.kind else {
                    return None;
                };
                Some((category, select.current_value.0.to_string()))
            })
            .map(async move |(category, current_value)| match category {
                SessionConfigOptionCategory::Mode => {
                    self.session_mode_set(
                        session_id,
                        &current_value,
                        SetSessionModeResponse::default(),
                    )
                    .await
                }
                SessionConfigOptionCategory::Model => {
                    self.session_model_set(session_id, &current_value).await
                }
                SessionConfigOptionCategory::ThoughtLevel => {
                    self.session_thought_level_set(session_id, &current_value)
                        .await
                }
                SessionConfigOptionCategory::ModelConfig => {
                    self.session_model_config_set(session_id, model_config_ref)
                        .await
                }
                _ => Ok(()),
            })
            .collect::<Vec<_>>();

        futures::future::try_join_all(futures).await?;

        // Every response carries the authoritative option set, including any
        // boolean options the agent advertises.
        {
            let mut state = self.state.lock().await;
            if let Some(details) = state.session_info.get_mut(session_id) {
                details.replace_config_options(response.config_options.clone());
            }
            drop(state);
        }

        self.execute_autocommand(Commands::ConfigurationUpdated, response)
            .await
    }

    pub async fn session_mode_set(
        &self,
        session_id: &str,
        updated_to: &str,
        _response: SetSessionModeResponse,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        let session_info = state.get_session_info_mut(session_id);
        if let Some(session) = session_info {
            let updated = session.get_mode(updated_to).cloned();

            if let Some(current) = updated {
                session.set_current_mode(current.clone());
                drop(state);
                self.execute_autocommand(Commands::ModeUpdated, current)
                    .await
            } else {
                drop(state);
                Err(Error::Internal(format!(
                    "Mode '{}' not found for session: {}",
                    updated_to, session_id
                )))
            }
        } else {
            drop(state);
            Err(Error::SessionNotFound(session_id.to_string()))
        }
    }

    pub async fn session_model_set(&self, session_id: &str, updated_to: &str) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        let session_info = state.get_session_info_mut(session_id);
        if let Some(session) = session_info {
            let updated = session.get_model(updated_to).cloned();

            if let Some(current) = updated {
                session.set_current_model(current.clone());
                drop(state);
                self.execute_autocommand(Commands::SessionModelUpdated, current)
                    .await
            } else {
                drop(state);
                Err(Error::Internal(format!(
                    "Model '{}' not found for session: {}",
                    updated_to, session_id
                )))
            }
        } else {
            drop(state);
            Err(Error::SessionNotFound(session_id.to_string()))
        }
    }

    pub async fn session_thought_level_set(
        &self,
        session_id: &str,
        updated_to: &str,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        let session_info = state.get_session_info_mut(session_id);
        if let Some(session) = session_info {
            let updated = session.get_thought_level(updated_to).cloned();

            if let Some(current) = updated {
                session.set_current_thought_level(current.clone());
                drop(state);
                self.execute_autocommand(Commands::ThoughtLevelUpdated, current)
                    .await
            } else {
                drop(state);
                Err(Error::Internal(format!(
                    "Thought level '{}' not found for session: {}",
                    updated_to, session_id
                )))
            }
        } else {
            drop(state);
            Err(Error::SessionNotFound(session_id.to_string()))
        }
    }

    pub async fn session_model_config_set(
        &self,
        session_id: &str,
        config_options: &[SessionConfigOption],
    ) -> Result<(), Error> {
        let model_configs =
            crate::acp::session_info::SessionDetails::parse_model_configs_from_options(
                config_options,
            );

        let mut state = self.state.lock().await;
        let session_info = state.get_session_info_mut(session_id);
        if let Some(session) = session_info {
            session.merge_or_replace_model_configs(model_configs);
            let merged = session.model_configs.clone();
            drop(state);
            self.execute_autocommand(Commands::ModelConfigurationUpdated, merged)
                .await
        } else {
            drop(state);
            Err(Error::SessionNotFound(session_id.to_string()))
        }
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_loaded(
        &self,
        session_id: String,
        response: LoadSessionResponse,
    ) -> Result<(), Error> {
        let details = response.clone();
        let mut state = self.state.lock().await;
        let session = NewSessionResponse::new(session_id)
            .modes(details.modes)
            .config_options(details.config_options);
        state.set_session_info(&session);
        drop(state);
        self.execute_autocommand(Commands::SessionLoaded, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn custom_command_executed(&self, _response: ExtResponse) -> Result<(), Error> {
        Ok(())
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn sessions_listed(&self, response: ListSessionsResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::SessionsListed, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_forked(&self, response: ForkSessionResponse) -> Result<(), Error> {
        self.execute_autocommand(Commands::SessionForked, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_resumed(
        &self,
        session_id: String,
        response: ResumeSessionResponse,
    ) -> Result<(), Error> {
        let details = response.clone();
        let mut state = self.state.lock().await;
        let session = NewSessionResponse::new(session_id)
            .modes(details.modes)
            .config_options(details.config_options);
        state.set_session_info(&session);
        drop(state);
        self.execute_autocommand(Commands::SessionResumed, response)
            .await
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_closed(
        &self,
        session_id: String,
        response: CloseSessionResponse,
    ) -> Result<(), Error> {
        let data = WithSessionId {
            session_id: session_id.clone(),
            inner: response,
        };
        self.execute_autocommand(Commands::SessionClosed, data)
            .await?;
        let mut state = self.state.lock().await;
        state.session_info.remove(&session_id);
        state.prompt.remove(&session_id);
        drop(state);
        Ok(())
    }

    #[instrument(level = "trace", skip(self))]
    pub async fn session_deleted(
        &self,
        session_id: String,
        response: DeleteSessionResponse,
    ) -> Result<(), Error> {
        let data = WithSessionId {
            session_id: session_id.clone(),
            inner: response,
        };
        self.execute_autocommand(Commands::SessionDeleted, data)
            .await?;
        let mut state = self.state.lock().await;
        state.session_info.remove(&session_id);
        state.prompt.remove(&session_id);
        drop(state);
        Ok(())
    }
}
