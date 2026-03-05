use crate::nvim::parse::convert_metadata_to_lua_object;
use agent_client_protocol::AuthenticateResponse;
use nvim_oxi::Dictionary;

pub fn authenticate_response(response: AuthenticateResponse) -> Dictionary {
    let mut data = nvim_oxi::Dictionary::new();

    if let Some(obj) = convert_metadata_to_lua_object(response.meta) {
        data.insert("meta", obj);
    }

    data
}
