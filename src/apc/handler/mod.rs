pub mod client;
pub mod response;

use std::sync::{Arc, Mutex};

use agent_client_protocol::Client;

use crate::PluginState;

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub fs_write_access: bool,
    pub fs_read_access: bool,
    pub terminal_access: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            fs_write_access: true,
            fs_read_access: true,
            terminal_access: true,
        }
    }
}

#[derive(Clone)]
pub struct Handler<H: Client> {
    pub state: Arc<Mutex<PluginState>>,
    handler: H,
}

impl<H: Client> Handler<H> {
    pub fn new(state: Arc<Mutex<PluginState>>, handler: H) -> Self {
        Self { state, handler }
    }

    pub fn can_write(&self) -> bool {
        self.state.lock().unwrap().config.fs_write_access
    }

    pub fn can_read(&self) -> bool {
        self.state.lock().unwrap().config.fs_read_access
    }

    pub fn can_access_terminal(&self) -> bool {
        self.state.lock().unwrap().config.terminal_access
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_new_client() {
//         let config = ClientConfig::default();
//         let client = ApcClient::new(config);
//         assert_eq!(client.config().name, "hermes");
//     }
//
//     #[test]
//     fn test_custom_config() {
//         let config = ClientConfig {
//             name: "test-client".to_string(),
//             version: "0.1.0".to_string(),
//             fs_read_access: true,
//             fs_write_access: false,
//             terminal_access: true,
//         };
//
//         let client = ApcClient::new(config.clone());
//         assert_eq!(client.config().name, "test-client");
//         assert_eq!(client.config().version, "0.1.0");
//         assert!(!client.config().fs_write_access);
//         assert!(client.config().terminal_access);
//     }
//
//     #[test]
//     fn test_default_config() {
//         let config = ClientConfig::default();
//         assert_eq!(config.name, "hermes");
//         assert!(config.fs_write_access);
//         assert!(config.terminal_access);
//     }
//
//     #[tokio::test]
//     async fn test_session_notification() {
//         use agent_client_protocol::{
//             ContentBlock, ContentChunk, SessionId, SessionUpdate, TextContent,
//         };
//
//         let client = ApcClient::new(ClientConfig::default());
//         let notification = SessionNotification::new(
//             SessionId::new("test-session"),
//             SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
//                 TextContent::new("Hello"),
//             ))),
//         );
//
//         let result = client.session_notification(notification).await;
//         assert!(result.is_ok());
//     }
// }
