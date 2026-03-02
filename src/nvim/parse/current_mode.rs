use crate::nvim::parse::convert_metadata_to_lua_object;
use agent_client_protocol::CurrentModeUpdate;
use nvim_oxi::Dictionary;

pub fn current_mode_event(update: CurrentModeUpdate) -> Dictionary {
    let mut data: nvim_oxi::Dictionary = nvim_oxi::Dictionary::new();
    data.insert("id", update.current_mode_id.to_string());
    if let Some(obj) = convert_metadata_to_lua_object(update.meta) {
        data.insert("meta", obj);
    }
    data
}
