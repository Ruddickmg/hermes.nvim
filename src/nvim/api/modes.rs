use crate::{
    acp::{Result, error::Error},
    api::Api,
    nvim::autocommands::Commands,
};

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn modes(&self, session_id: String) -> Result<()> {
        let state = self.state.lock().await;
        let details = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.clone()))?
            .clone();
        drop(state);

        if let Some(mode_details) = details.modes {
            self.response_handler
                .execute_autocommand(Commands::Modes, mode_details)
                .await
        } else {
            Err(Error::Unsupported("mode".to_string()))
        }
    }
}
