pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod state;

use nvim_oxi::{Dictionary, api::opts::CreateAugroupOpts};
use std::{
    rc::Rc,
    sync::{Arc},
};
use tokio::sync::Mutex;

use crate::{Handler, apc::connection::ConnectionManager};

const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let plugin_state = Arc::new(Mutex::new(state::PluginState::new()));
    let auto_command_generator = autocommands::AutoCommands::new(GROUP.to_string());
    let event_handler = Arc::new(Handler::new(plugin_state.clone(), auto_command_generator));
    let connection_manager = Rc::new(Mutex::new(ConnectionManager::new(event_handler.clone())));

    nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default()).unwrap();

    Ok(Dictionary::from_iter([
        ("connect", api::connect(connection_manager.clone())),
        (
            "authenticate",
            api::authenticate(connection_manager.clone()),
        ),
        ("disconnect", api::disconnect(connection_manager.clone())),
    ]))
}
