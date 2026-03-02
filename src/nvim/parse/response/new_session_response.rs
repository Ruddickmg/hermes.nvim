use agent_client_protocol::NewSessionResponse;
use nvim_oxi::Dictionary;

use crate::nvim::parse::parse_modes;

pub fn new_session_response(response: NewSessionResponse) -> Dictionary {
    let mut data = nvim_oxi::Dictionary::new();

    data.insert("sessionId", response.session_id.0.as_ref().to_string());

    if let Some(modes) = response.modes {
        let mut modes_dict = nvim_oxi::Dictionary::new();
        modes_dict.insert(
            "currentModeId",
            modes.current_mode_id.0.as_ref().to_string(),
        );
        modes_dict.insert("availableModes", parse_modes(modes.available_modes));
        data.insert("modes", modes_dict);
    }

    if let Some(config_options) = response.config_options {
        let config_options_arr = nvim_oxi::Array::from_iter(
            config_options
                .into_iter()
                .map(super::parse_session_config_option),
        );
        data.insert("configOptions", config_options_arr);
    }

    if let Some(obj) = crate::nvim::parse::convert_metadata_to_lua_object(response.meta) {
        data.insert("meta", obj);
    }

    data
}
