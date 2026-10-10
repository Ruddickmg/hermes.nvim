use crate::{
    acp::{Result, error::Error},
    api::Api,
};
use agent_client_protocol::schema::v1::CancelNotification;

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn cancel(&self, session_id: String) -> Result<()> {
        let connection_manager = self.connection_manager.lock().await;
        let connection = connection_manager
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;
        let result = connection
            .cancel(CancelNotification::new(session_id.clone()))
            .await;
        drop(connection_manager);
        result?;

        crate::nvim::requests::RequestHandler::cancel_session_requests(
            &*self.request_handler,
            session_id,
        )
        .await
    }
}
