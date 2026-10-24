use crate::{
    acp::{Result, error::Error},
    api::Api,
    nvim::autocommands::Commands,
};

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn models(&self, session_id: String) -> Result<()> {
        let state = self.state.lock().await;
        let details = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.clone()))?
            .clone();
        drop(state);

        if let Some(model_details) = details.models {
            self.response_handler
                .execute_autocommand(Commands::Models, model_details)
                .await
        } else {
            Err(Error::Unsupported("model".to_string()))
        }
    }
}
