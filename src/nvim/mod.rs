pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod requests;
pub mod state;

use nvim_oxi::{Dictionary, api::opts::CreateAugroupOpts};
use std::{rc::Rc, sync::Arc};
use tokio::sync::Mutex;

use crate::{Handler, acp::connection::ConnectionManager, api::Api, utilities::logging::Logger};

pub const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let _logger = Logger::inititalize();
    let plugin_state = Arc::new(Mutex::new(state::PluginState::new()));
    let request_handler = Arc::new(requests::Requests::new());
    let auto_command = autocommands::AutoCommand::new(request_handler.clone())?;
    let event_handler = Arc::new(Handler::new(plugin_state.clone(), auto_command));
    let connection_manager = Rc::new(Mutex::new(ConnectionManager::new(event_handler.clone())));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all() // Enables I/O and time drivers
        .build()
        .map_err(|e| {
            nvim_oxi::Error::Lua(nvim_oxi::lua::Error::RuntimeError(format!(
                "Failed to create Tokio runtime: {}",
                e
            )))
        })?;
    let api_methods = Rc::new(Api::new(
        connection_manager.clone(),
        request_handler.clone(),
        runtime,
    ));

    nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default()).map_err(|e| {
        nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Failed to create autogroup for the '{}' group: {}",
            GROUP, e
        )))
    })?;

    Ok(Dictionary::from_iter([
        ("cancel", api::cancel(api_methods.clone())),
        ("connect", api::connect(api_methods.clone())),
        ("authenticate", api::authenticate(api_methods.clone())),
        ("disconnect", api::disconnect(api_methods.clone())),
        ("createSession", api::create_session(api_methods.clone())),
        ("prompt", api::prompt(api_methods.clone())),
        ("setMode", api::set_mode(api_methods.clone())),
        ("respond", api::respond(api_methods.clone())),
    ]))
}
