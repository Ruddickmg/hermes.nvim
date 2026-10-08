pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod requests;
pub mod state;
pub mod terminal;

use crate::{
    Handler,
    acp::{connection::ConnectionManager, error::Error, registry::Registry},
    api::{DisconnectArgs, Hermes},
    utilities::{
        Downloader, Logger, NvimRuntime, create_augroup, create_autocmd,
        detect_project_storage_path,
    },
};
use async_lock::Mutex;
use nvim_oxi::Dictionary;
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    // Apply default progress configuration: suppress cmdline progress
    // This runs at module load even without setup(), ensuring sensible defaults
    configuration::show_progress_in_cmdline(false);

    let storage_path = detect_project_storage_path()?;
    let logger = Logger::inititalize(&storage_path)?;
    let downloader = Downloader::new(
        logger.nvim_notifications_messenger.clone(),
        configuration::ProgressConfig::default(),
    );
    let registry = Registry::new(downloader);
    let nvim_runtime = NvimRuntime::new();
    let plugin_state = Arc::new(Mutex::new(
        state::PluginState::new()
            .with_storage_path(std::path::PathBuf::from(&storage_path))
            .with_registry(registry),
    ));
    let connection_manager = Arc::new(Mutex::new(ConnectionManager::new(plugin_state.clone())));
    let request_handler = Rc::new(requests::Requests::new(
        nvim_runtime.clone(),
        plugin_state.clone(),
    )?);
    let event_handler = Arc::new(Handler::new(
        plugin_state.clone(),
        connection_manager.clone(),
        nvim_runtime.clone(),
        request_handler.clone(),
    )?);
    let api = Rc::new(RefCell::new(api::Api::new(
        plugin_state,
        logger,
        event_handler,
        request_handler,
        connection_manager,
    )));
    let cloned = api.clone();
    let shutdown_runtime = nvim_runtime.clone();
    let hermes = Hermes::new(nvim_runtime, api)?;

    let group = create_augroup(GROUP, true)?;

    create_autocmd(group, "VimLeavePre", move || {
        let mut app = cloned
            .try_borrow_mut()
            .map_err(|e| Error::Internal(format!("Failed to borrow API on VimLeavePre: {}", e)))?;
        shutdown_runtime
            .block_on_primary(async move { app.disconnect(DisconnectArgs::All).await })?;
        Ok(true)
    })?;

    Ok(hermes.into())
}
