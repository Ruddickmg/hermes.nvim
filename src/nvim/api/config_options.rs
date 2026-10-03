use crate::{
    acp::{Result, error::Error},
    api::Api,
    nvim::autocommands::Commands,
};

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn config_options(&self, session_id: String) -> Result<()> {
        let state = self.state.lock().await;
        let options = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.clone()))?
            .all_config_options()
            .to_vec();
        drop(state);

        self.response_handler
            .execute_autocommand(Commands::ConfigOptions, options)
            .await
    }
}
