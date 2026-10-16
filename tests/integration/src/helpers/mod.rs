pub mod mock;
pub mod ui;

pub use mock::*;
pub use ui::*;

use async_lock::Mutex;
use hermes::{
    PluginState, acp::connection::ConnectionManager, acp::handler::Handler, utilities::NvimRuntime,
};
use std::rc::Rc;
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

/// Creates a `Handler` wired to a fresh plugin state, a mock connection manager,
/// runtime, and request handler. Shared by tests that only need a constructible
/// handler and don't care about its behavior.
pub fn mock_handler() -> Handler {
    let state = Arc::new(Mutex::new(PluginState::default()));
    Handler::new(
        state.clone(),
        mock_connection_manager(&state),
        mock_runtime(),
        Rc::new(MockRequestHandler::new()),
    )
    .expect("Handler creation should succeed")
}
