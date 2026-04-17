use crate::{
    acp::{Result, error::Error},
    api::Api,
};
use agent_client_protocol::CancelNotification;

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn cancel(&self, session_id: String) -> Result<()> {
        let connection = self
            .connection
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        connection.cancel(CancelNotification::new(session_id.clone())).await?;

        crate::nvim::requests::RequestHandler::cancel_session_requests(
            &*self.request_handler,
            session_id,
        )
        .await
    }
}
