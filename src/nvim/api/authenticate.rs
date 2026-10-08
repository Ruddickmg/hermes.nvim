use agent_client_protocol::schema::v1::{AuthMethod, AuthMethodTerminal, AuthenticateRequest};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    acp::{Result, connection::Assistant, error::Error},
    api::Api,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TerminalAuthenticateRequest {
    pub request_id: String,
    pub method: AuthMethodTerminal,
    pub agent: Assistant,
}

impl TerminalAuthenticateRequest {
    pub fn new(agent: Assistant, method: AuthMethodTerminal) -> Self {
        TerminalAuthenticateRequest {
            agent,
            method,
            request_id: Uuid::new_v4().to_string(),
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_terminal_authenticate_request_serializes_expected_shape() {
        let request = TerminalAuthenticateRequest::new(
            Assistant::Opencode,
            AuthMethodTerminal::new("tui-auth", "Terminal Auth")
                .args(vec!["--device-code".to_string()]),
        );
        let value = serde_json::to_value(&request).expect("serialization should succeed");

        assert_eq!(
            value,
            serde_json::json!({
                "request_id": request.request_id,
                "method": {
                    "id": "tui-auth",
                    "name": "Terminal Auth",
                    "args": ["--device-code"]
                },
                "agent": "Opencode"
            })
        );
    }
}
