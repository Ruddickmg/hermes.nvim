use agent_client_protocol::PromptResponse;
use nvim_oxi::Dictionary;

pub fn prompt_response(response: PromptResponse) -> Dictionary {
    let mut data = nvim_oxi::Dictionary::new();

    data.insert("stopReason", format!("{:#?}", response.stop_reason));

    if let Some(obj) = crate::nvim::parse::convert_metadata_to_lua_object(response.meta) {
        data.insert("meta", obj);
    }

    data
}
