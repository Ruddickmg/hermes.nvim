use crate::{
    acp::{Result, error::Error},
    api::Api,
};

/// Tuple for two positional arguments: (session_id, mode_id)
pub type SetModeArgs = (String, String);

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_mode(&self, (session_id, mode_id): SetModeArgs) -> Result<()> {
        let request = agent_client_protocol::SetSessionModeRequest::new(session_id, mode_id);

        let connection = self
            .connection
            .get_current_connection()
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        connection.set_mode(request)
    }
}
