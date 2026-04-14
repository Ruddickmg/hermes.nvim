pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod requests;
pub mod state;
pub mod terminal;

use crate::{
    Handler,
    api::{Api, DisconnectArgs},
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
    let api = Rc::new(RefCell::new(api::Api::new(
        plugin_state,
        logger,
        event_handler,
        request_handler,
    )));
    let cloned = api.clone();

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
                match cloned.try_borrow_mut() {
                    Ok(mut app) => {
                        app.disconnect(DisconnectArgs::All)
                            .inspect_err(|e| error!("Error disconnecting: {:?}", e))
                            .ok();
                    }
                    Err(e) => error!(
                        "An error occurred while disconnecting sessions on exit: {:?}",
                        e
                    ),
                };
                true
            })
            .build(),
    )?;

    Ok(Api::to_dictionary(api))
}
