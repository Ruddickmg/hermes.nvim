use crate::nvim::parse;
use agent_client_protocol::SetSessionModeResponse;
use nvim_oxi::Dictionary;

pub fn mode_response(response: SetSessionModeResponse) -> Option<Dictionary> {
    parse::convert_metadata_to_lua_object(response.meta).map(|obj| {
        let mut data = nvim_oxi::Dictionary::new();
        data.insert("meta", obj);
        data
    })
}
