pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod requests;
pub mod state;
pub mod terminal;

use crate::{
    Handler,
    acp::connection::ConnectionManager,
    utilities::{Logger, detect_project_storage_path},
};
use nvim_oxi::{
    Dictionary,
    api::opts::{CreateAugroupOpts, CreateAutocmdOpts},
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use tokio::sync::Mutex;
use tracing::error;

pub const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let storage_path = detect_project_storage_path()?;
    let logger = Logger::inititalize(&storage_path)?;
    let plugin_state = Arc::new(Mutex::new(state::PluginState::new()));
    let request_handler = Rc::new(requests::Requests::new(plugin_state.clone())?);
    let event_handler = Arc::new(Handler::new(plugin_state.clone(), request_handler.clone())?);
    let connection_manager = Rc::new(RefCell::new(ConnectionManager::new(plugin_state.clone())));
    let connection = connection_manager.clone();

    let group =
        nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default()).map_err(|e| {
            nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
                "Failed to create autogroup for the '{}' group: {}",
                GROUP, e
            )))
        })?;

    // clean up on exit
    nvim_oxi::api::create_autocmd(
        ["VimLeavePre"],
        &CreateAutocmdOpts::builder()
            .group(group)
            .callback(move |_| {
                connection
                    .borrow_mut()
                    .close_all()
                    .inspect_err(|e| error!("Error occurred while exiting neovim: {:?}", e))
                    .ok();
                true
            })
            .build(),
    )?;

    Ok(Dictionary::from_iter([
        (
            "cancel",
            api::cancel(connection_manager.clone(), request_handler.clone()),
        ),
        (
            "connect",
            api::connect(connection_manager.clone(), event_handler.clone()),
        ),
        (
            "authenticate",
            api::authenticate(connection_manager.clone()),
        ),
        ("disconnect", api::disconnect(connection_manager.clone())),
        (
            "create_session",
            api::create_session(connection_manager.clone(), plugin_state.clone()),
        ),
        (
            "load_session",
            api::load_session(connection_manager.clone(), plugin_state.clone()),
        ),
        (
            "list_sessions",
            api::list_sessions(connection_manager.clone()),
        ),
        ("prompt", api::prompt(connection_manager.clone())),
        ("set_mode", api::set_mode(connection_manager.clone())),
        ("respond", api::respond(request_handler)),
        ("setup", api::setup(plugin_state, logger)),
    ]))
}
