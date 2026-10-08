use crate::{
    acp::{Result, error::Error, session_info::SessionDetails},
    api::Api,
};

/// Tuple for two positional arguments: (session_id, mode_id)
pub type SetModeArgs = (String, String);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_mode(&self, (session_id, mode_id): SetModeArgs) -> Result<()> {
        let state = self.state.lock().await;
        let legacy = state
            .session_info
            .get(&session_id)
            .map(|info: &SessionDetails| info.mode_is_legacy());
        drop(state);

        if legacy.is_none() {
            return Err(Error::SessionNotFound(session_id));
        }

        let config_type = "mode".to_string();

        if let Some(is_legacy) = legacy.unwrap_or_default() {
            let connection = self
                .connection_manager
                .get_current_connection()
                .await
                .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

            if is_legacy {
                connection
                    .set_mode(
                        agent_client_protocol::schema::v1::SetSessionModeRequest::new(
                            session_id, mode_id,
                        ),
                    )
                    .await?;
            } else {
                connection
                    .set_config_option(
                        agent_client_protocol::schema::v1::SetSessionConfigOptionRequest::new(
                            session_id,
                            config_type.clone(),
                            agent_client_protocol::schema::v1::SessionConfigOptionValue::value_id(
                                mode_id,
                            ),
                        ),
                    )
                    .await?;
            }
            Ok(())
        } else {
            Err(Error::Unsupported(config_type))
        }
    }
}
