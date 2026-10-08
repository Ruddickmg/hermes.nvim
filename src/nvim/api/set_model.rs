use agent_client_protocol::schema::v1::SetSessionConfigOptionRequest;

use crate::{
    acp::{Result, error::Error},
    api::Api,
};

/// Tuple for two positional arguments: (session_id, model_id)
pub type SetModelArgs = (String, String);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_model(&self, (session_id, model_id): SetModelArgs) -> Result<()> {
        let connection = self
            .connection_manager
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        connection
            .set_config_option(SetSessionConfigOptionRequest::new(
                session_id,
                "model".to_string(),
                agent_client_protocol::schema::v1::SessionConfigOptionValue::value_id(model_id),
            ))
            .await?;

        Ok(())
    }
}
