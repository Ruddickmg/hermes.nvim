use tracing::instrument;
use uuid::Uuid;

use crate::{
    acp::{Result, error::Error},
    api::Api,
};

pub type RespondArgs = (String, nvim_oxi::Object);

impl Api {
    #[instrument(level = "trace", skip_all)]
    pub async fn respond(&self, (request_id, response_data): RespondArgs) -> Result<()> {
        let request_uuid = Uuid::parse_str(&request_id)
            .map_err(|e| Error::InvalidInput(format!("Invalid response id: {:?}", e)))?;
        crate::nvim::requests::RequestHandler::handle_response(
            &*self.request_handler,
            &request_uuid,
            response_data,
        )
        .await
    }
}
