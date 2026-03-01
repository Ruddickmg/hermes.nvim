pub mod api;
pub mod parse;
pub mod producer;
pub mod state;

use nvim_oxi::{Dictionary, api::opts::CreateAugroupOpts};
use std::sync::{Arc, Mutex};

use crate::{Handler, apc::connection::ConnectionManager, nvim::producer::AutoCommands};

const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let plugin_state = Arc::new(Mutex::new(state::PluginState::new()));
    let auto_command_generator = AutoCommands::new(GROUP.to_string());
    let event_handler = Arc::new(Handler::new(plugin_state.clone(), auto_command_generator));
    let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(event_handler.clone())));

    nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default()).unwrap();

    Ok(Dictionary::from_iter([
        (
            "connect",
            api::create_lua_connect(connection_manager.clone()),
        ),
        // (
        //     "authenticate",
        //     api::create_lua_authenticate(plugin_state.clone()),
        // ),
        // ("prompt", api::create_lua_prompt(plugin_state.clone())),
    ]))
}
