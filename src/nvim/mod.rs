pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod state;

use nvim_oxi::{Dictionary, api::opts::CreateAugroupOpts};
use std::{rc::Rc, sync::Arc};
use tokio::sync::Mutex;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::{Handler, acp::connection::ConnectionManager, nvim::autocommands::AutoCommand};

pub const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).map_err(|e| {
        nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Failed to set global subscriber: {}",
            e
        )))
    })?;

    let (handle, sender) = AutoCommand::listener()?;
    let plugin_state = Arc::new(Mutex::new(state::PluginState::new()));
    let auto_command = autocommands::AutoCommand::producer(sender, handle);
    let event_handler = Arc::new(Handler::new(plugin_state.clone(), auto_command));
    let connection_manager = Rc::new(Mutex::new(ConnectionManager::new(event_handler.clone())));

    nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default()).map_err(|e| {
        nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Failed to create autogroup for the '{}' group: {}",
            GROUP,
            e
        )))
    })?;

    Ok(Dictionary::from_iter([
        ("connect", api::connect(connection_manager.clone())),
        (
            "authenticate",
            api::authenticate(connection_manager.clone()),
        ),
        ("disconnect", api::disconnect(connection_manager.clone())),
    ]))
}
