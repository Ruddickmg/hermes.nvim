pub mod api;
pub mod parse;
pub mod producer;
pub mod state;

use nvim_oxi::Dictionary;
use std::{rc::Rc, sync::Mutex};

#[nvim_oxi::plugin]
pub fn hermes() -> nvim_oxi::Result<Dictionary> {
    let plugin_state = Rc::new(Mutex::new(state::PluginState::new()?));

    Ok(Dictionary::from_iter([
        ("connect", api::create_lua_connect(plugin_state.clone())),
        // (
        //     "authenticate",
        //     api::create_lua_authenticate(plugin_state.clone()),
        // ),
        // ("prompt", api::create_lua_prompt(plugin_state.clone())),
    ]))
}
