use crate::nvim::parse::{convert_metadata_to_lua_object, parse_session_config_option};
use agent_client_protocol::SetSessionConfigOptionResponse;
use nvim_oxi::Dictionary;

pub fn config_option_response(response: SetSessionConfigOptionResponse) -> Dictionary {
    let mut data = nvim_oxi::Dictionary::new();

    let config_options_arr = nvim_oxi::Array::from_iter(
        response
            .config_options
            .into_iter()
            .map(parse_session_config_option),
    );
    data.insert("configOptions", config_options_arr);

    if let Some(obj) = convert_metadata_to_lua_object(response.meta) {
        data.insert("meta", obj);
    }

    data
}
