use agent_client_protocol::schema::v1::{AuthMethod, AuthenticateRequest};

use crate::{
    acp::{Result, error::Error},
    api::Api,
};

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn authenticate(&self, id: String) -> Result<()> {
        let state = self.state.lock().await;
        let agent = state.agent_info.current.clone();
        let auth_method = state.agent_info.get_auth_method(id.clone());
        drop(state);
        if let Some(method) = auth_method {
            let connection_manager = self.connection_manager.lock().await;
            let connection = connection_manager
                .get_current_connection()
                .await
                .ok_or_else(|| Error::Connection("No connection found".to_string()))?;
            let result = match method {
                AuthMethod::Terminal(terminal_method) => {
                    connection
                        .terminal_authentication(agent, terminal_method)
                        .await
                }
                _ => connection.authenticate(AuthenticateRequest::new(id)).await,
            };
            drop(connection_manager);
            result
        } else {
            Err(Error::InvalidInput(format!(
                "No auth method with id: \"{}\" found for agent: {}",
                id, agent
            )))
        }
    }
}
