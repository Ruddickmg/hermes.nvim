use agent_client_protocol::SetSessionModelResponse;
use nvim_oxi::Dictionary;

pub fn session_model_response(response: SetSessionModelResponse) -> Dictionary {
    let mut data = nvim_oxi::Dictionary::new();

    if let Some(obj) = crate::nvim::parse::convert_metadata_to_lua_object(response.meta) {
        data.insert("meta", obj);
    }

    data
}
