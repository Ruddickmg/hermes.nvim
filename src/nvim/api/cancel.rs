use crate::{acp::{Result, error::Error}, api::Api};
use agent_client_protocol::CancelNotification;

use crate::{
    nvim::requests::RequestHandler,
};

impl Api {

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn cancel(&self, session_id: String) -> Result<()> {
        let connection = self
            .connection
            .get_current_connection()
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        connection.cancel(CancelNotification::new(session_id.clone()))?;

        self.request_handler.cancel_session_requests(session_id)
    }
}
