use agent_client_protocol::AuthenticateRequest;

use crate::{
    acp::{Result, error::Error},
    api::Api,
};

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn authenticate(&self, id: String) -> Result<()> {
        let args: AuthenticateRequest = AuthenticateRequest::new(id);
        let connection = self
            .connection
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;
        connection.authenticate(args).await
    }
}
