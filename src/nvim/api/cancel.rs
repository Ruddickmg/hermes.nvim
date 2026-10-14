use crate::{
    acp::{Result, error::Error},
    api::Api,
    nvim::requests::RequestHandler,
};
use agent_client_protocol::schema::v1::CancelNotification;
use futures::future::try_join;

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn cancel(&self, id: String) -> Result<()> {
        let state = self.state.lock().await;
        let session = state.get_session_info(&id);
        let is_session_id = session.is_some();
        drop(state);
        let connection_manager = self.connection_manager.lock().await;
        let connection = connection_manager
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        if is_session_id {
            connection
                .cancel(CancelNotification::new(id.clone()))
                .await?;
            drop(connection_manager);

            self.request_handler.cancel_session_requests(id).await?;
        } else {
            let request_id = uuid::Uuid::parse_str(&id).map_err(|_| {
                Error::InvalidInput(format!(
                    "No session found and id is invalid type for a request_id: {}",
                    id
                ))
            })?;
            let request = self.request_handler.get(&request_id).await.ok_or_else(|| {
                Error::InvalidInput(format!("No request found for id: {}", request_id))
            })?;

            try_join(
                connection.cancel_request(request.request_id()),
                self.response_handler.cancel_request(&request.request_id()),
            )
            .await?;

            drop(connection_manager);
        }
        Ok(())
    }
}
