use crate::{
    acp::{Result, error::Error},
    api::Api,
};

/// Tuple for two positional arguments: (session_id, level)
pub type SetThoughtLevelArgs = (String, String);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_thought_level(&self, (session_id, level): SetThoughtLevelArgs) -> Result<()> {
        let state = self.state.lock().await;
        let has_thought_levels = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.clone()))?
            .thought_level_options()
            .is_some();
        drop(state);

        if !has_thought_levels {
            return Err(Error::Unsupported("thought_level".to_string()));
        }

        let connection_manager = self.connection_manager.lock().await;
        let connection = connection_manager
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;
        let result = connection
            .set_config_option(
                agent_client_protocol::schema::v1::SetSessionConfigOptionRequest::new(
                    session_id,
                    "thought_level".to_string(),
                    agent_client_protocol::schema::v1::SessionConfigOptionValue::value_id(level),
                ),
            )
            .await;
        drop(connection_manager);
        result
    }
}
