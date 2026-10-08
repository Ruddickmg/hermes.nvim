use agent_client_protocol::schema::v1::{CloseSessionRequest, SessionId};

use crate::{
    acp::{Result, error::Error},
    api::Api,
    nvim::requests::RequestHandler,
};

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn close_session(&self, session_id: String) -> Result<()> {
        let state = self.state.lock().await;
        let agent_info = state.agent_info.clone();
        drop(state);

        if !agent_info.can_close_session() {
            return Ok(());
        }

        self.request_handler
            .cancel_session_requests(session_id.clone())
            .await?;

        let request = CloseSessionRequest::new(SessionId::from(session_id));

        let connection_manager = self.connection_manager.lock().await;
        let connection = connection_manager
            .get_current_connection()
            .await
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;
        let result = connection.close_session(request).await;
        drop(connection_manager);
        result
    }
}
