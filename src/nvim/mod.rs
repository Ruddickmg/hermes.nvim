pub mod api;
pub mod autocommands;
pub mod configuration;
pub mod parse;
pub mod state;

use nvim_oxi::{
    Dictionary, Object,
    api::opts::{CreateAugroupOpts, ExecAutocmdsOpts},
};
use std::{rc::Rc, sync::Arc};
use tokio::{sync::Mutex, sync::mpsc::channel};

use crate::{Handler, apc::connection::ConnectionManager, nvim::autocommands::Commands};

const GROUP: &str = "hermes";

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let (sender, mut receiver) = channel::<(Commands, serde_json::Value)>(100);
    let handle = nvim_oxi::libuv::AsyncHandle::new(move || {
        while let Ok((command, data)) = receiver.try_recv() {
            let obj: Object =
                serde_json::from_value(data).expect("Failed to convert data to Object");
            let opts = ExecAutocmdsOpts::builder()
                .patterns(command.to_string())
                .data(obj)
                .group(GROUP)
                .build();
            if let Err(err) = nvim_oxi::api::exec_autocmds(["User"], &opts) {
                eprintln!("Error executing autocommand '{}': {:?}", command, err);
            }
        }
    })?;
    let plugin_state = Arc::new(Mutex::new(state::PluginState::new()));
    let auto_command_generator = autocommands::AutoCommands::new(GROUP.to_string(), sender, handle);
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

pub fn example() {}
