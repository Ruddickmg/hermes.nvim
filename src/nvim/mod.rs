pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod requests;
pub mod state;
pub mod terminal;

use crate::{
    Handler, api::DisconnectArgs, utilities::{Logger, detect_project_storage_path}
};
use nvim_oxi::{
    Dictionary, api::opts::{CreateAugroupOpts, CreateAutocmdOpts}
};
use tracing::{debug, error};
use std::{rc::Rc, sync::Arc};
use tokio::sync::Mutex;

pub const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let storage_path = detect_project_storage_path()?;
    let logger = Logger::inititalize(&storage_path)?;
    let plugin_state = Arc::new(Mutex::new(state::PluginState::new()));
    let request_handler = Rc::new(requests::Requests::new(plugin_state.clone())?);
    let event_handler = Arc::new(Handler::new(plugin_state.clone(), request_handler.clone())?);
    let api = api::Api::new(
        plugin_state,
        logger,
        event_handler,
        request_handler,
    );

    let group =
        nvim_oxi::api::create_augroup(GROUP, &CreateAugroupOpts::default()).map_err(|e| {
            nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
                "Failed to create autogroup for the '{}' group: {}",
                GROUP, e
            )))
        })?;

    let app: Dictionary = api.into();
    
    // clean up on exit
    nvim_oxi::api::create_autocmd(
        ["VimLeavePre"],
        &CreateAutocmdOpts::builder()
            .group(group)
            .callback(move |_| {
                if let Err(e) = app.get("disconnect").unwrap().call(DisconnectArgs::All) {
                    error!("An error occurred while disconnecting sessions on exit: {:?}", e);
                } else {
                    debug!("Successfully disconnected all sessions on exit");
                }
                true
            })
            .build(),
    )?;
    
    Ok(api.into())
}
