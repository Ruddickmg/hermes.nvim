use crate::nvim::parse::annotations::parse_annotations;
use crate::nvim::parse::convert_metadata_to_lua_object;
use agent_client_protocol::TextContent;
use nvim_oxi::Dictionary;

pub fn text_event(content: TextContent) -> (Dictionary, String) {
    let mut dict: Dictionary = Dictionary::new();
    dict.insert("text", content.text);
    if let Some(annotations) = content.annotations {
        dict.insert("annotations", parse_annotations(annotations));
    }
    if let Some(obj) = convert_metadata_to_lua_object(content.meta) {
        dict.insert("meta", obj);
    }
    (dict, "Text".to_string())
}
