use crate::{
    acp::{Result, error::Error},
    api::Api,
    nvim::autocommands::Commands,
};

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn thought_levels(&self, session_id: String) -> Result<()> {
        let state = self.state.lock().await;
        let details = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.clone()))?
            .clone();
        drop(state);

        if let Some(tl_details) = details.thought_levels {
            self.response_handler
                .execute_autocommand(Commands::ThoughtLevels, tl_details)
                .await
        } else {
            Err(Error::Unsupported("thought_level".to_string()))
        }
    }
}
