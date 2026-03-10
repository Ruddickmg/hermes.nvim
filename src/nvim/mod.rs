pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod state;

use nvim_oxi::{Dictionary, api::opts::CreateAugroupOpts};
use std::{rc::Rc, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    Handler, acp::connection::ConnectionManager,
    utilities::logging::Logger,
};

pub const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let _logger = Logger::inititalize();
    let plugin_state = Arc::new(Mutex::new(state::PluginState::new()));
    let auto_command = autocommands::AutoCommand::new()?;
    let event_handler = Arc::new(Handler::new(plugin_state.clone(), auto_command));
    let connection_manager = Rc::new(Mutex::new(ConnectionManager::new(event_handler.clone())));

    nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default()).map_err(|e| {
        nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Failed to create autogroup for the '{}' group: {}",
            GROUP, e
        )))
    })?;

    Ok(Dictionary::from_iter([
        ("cancel", api::cancel(connection_manager.clone())),
        ("connect", api::connect(connection_manager.clone())),
        (
            "authenticate",
            api::authenticate(connection_manager.clone()),
        ),
        ("disconnect", api::disconnect(connection_manager.clone())),
        (
            "createSession",
            api::create_session(connection_manager.clone()),
        ),
        ("prompt", api::prompt(connection_manager.clone())),
        ("setMode", api::set_mode(connection_manager.clone())),
    ]))
}
