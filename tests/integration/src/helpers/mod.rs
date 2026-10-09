pub mod mock;
pub mod ui;

pub use mock::*;
pub use ui::*;

use async_lock::Mutex;
use hermes::{PluginState, acp::connection::ConnectionManager, utilities::NvimRuntime};
use std::sync::Arc;

/// Creates a smol LocalExecutor for testing
pub fn mock_runtime() -> NvimRuntime {
    NvimRuntime::new()
}

/// Creates a `ConnectionManager` bound to `state`, matching how the plugin wires it in
/// `hermes::nvim::hermes`.
pub fn mock_connection_manager(state: &Arc<Mutex<PluginState>>) -> Arc<Mutex<ConnectionManager>> {
    Arc::new(Mutex::new(ConnectionManager::new(state.clone())))
}
