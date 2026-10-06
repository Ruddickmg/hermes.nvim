use agent_client_protocol::schema::v1::{AuthMethod, AuthMethodTerminal, AuthenticateRequest};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    acp::{Result, connection::Assistant, error::Error},
    api::Api,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TerminalAuthenticateRequest {
    request_id: String,
    method: AuthMethodTerminal,
    agent: Assistant,
}

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn authenticate(&self, id: String) -> Result<()> {
        let state = self.state.lock().await;
        let agent = state.agent_info.current.clone();
        let auth_method = state.agent_info.get_auth_method(id.clone());
        drop(state);
        if let Some(method) = auth_method {
            let connection = self
                .connection
                .get_current_connection()
                .await
                .ok_or_else(|| Error::Connection("No connection found".to_string()))?;
            match method {
                AuthMethod::Terminal(terminal_method) => {
                    connection
                        .terminal_authentication(TerminalAuthenticateRequest {
                            request_id: Uuid::new_v4().to_string(),
                            method: terminal_method,
                            agent,
                        })
                        .await
                }
                _ => connection.authenticate(AuthenticateRequest::new(id)).await,
            }
        } else {
            Err(Error::InvalidInput(format!(
                "No auth method with id: \"{}\" found for agent: {}",
                id, agent
            )))
        }
    }
}
