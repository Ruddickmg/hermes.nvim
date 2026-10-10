//! Integration tests for Assistant command construction

use crate::helpers::mock_handler;
use async_lock::Mutex;
use hermes::PluginState;
use hermes::acp::{
    connection::{Assistant, ConnectionManager},
    registry::entry::AgentEntry,
};
use hermes::nvim::configuration::DistributionsConfig;
use std::collections::HashMap;
use std::sync::Arc;

#[nvim_oxi::test]
fn assistant_command_with_no_registry_returns_error() {
    let assistant = Assistant::Registered {
        agent: AgentEntry {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::new(),
        },
        distribution: None,
        configuration: DistributionsConfig::default(),
        command: None,
        args: None,
        registry: None,
    };

    let result = smol::block_on(assistant.command());
    assert!(
        result.is_err(),
        "command() should fail when registry is None"
    );
}

#[nvim_oxi::test]
fn assistant_command_with_no_registry_error_mentions_registry() {
    let assistant = Assistant::Registered {
        agent: AgentEntry {
            id: "test-agent".to_string(),
            name: "Test Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            repository: None,
            website: None,
            authors: None,
            license: None,
            icon: None,
            distribution: HashMap::new(),
        },
        distribution: None,
        configuration: DistributionsConfig::default(),
        command: None,
        args: None,
        registry: None,
    };

    let result = smol::block_on(assistant.command());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("registry") || err.contains("Registry"),
        "Error should mention missing registry: {}",
        err
    );
}

#[nvim_oxi::test]
fn reconnect_returns_error_when_no_connection() -> nvim_oxi::Result<()> {
    let mut manager = ConnectionManager::new(Arc::new(Mutex::new(PluginState::new())));
    let handler = Arc::new(mock_handler());

    let result = smol::block_on(manager.reconnect(handler, &Assistant::Opencode));

    assert_eq!(
        result.unwrap_err().to_string(),
        "Connection error: No connection found for assistant opencode"
    );
    Ok(())
}
