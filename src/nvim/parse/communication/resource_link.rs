use crate::nvim::parse::annotations::parse_annotations;
use crate::nvim::parse::convert_metadata_to_lua_object;
use agent_client_protocol::ResourceLink;
use nvim_oxi::Dictionary;

pub fn resource_link_event(block: ResourceLink) -> (Dictionary, String) {
    let mut dict: Dictionary = Dictionary::new();
    dict.insert("name", block.name);
    dict.insert("uri", block.uri);
    if let Some(description) = block.description {
        dict.insert("description", description);
    }
    if let Some(mime_type) = block.mime_type {
        dict.insert("mimeType", mime_type);
    }
    if let Some(size) = block.size {
        dict.insert("size", size);
    }
    if let Some(title) = block.title {
        dict.insert("title", title);
    }
    if let Some(annotations) = block.annotations {
        dict.insert("annotations", parse_annotations(annotations));
    }
    if let Some(obj) = convert_metadata_to_lua_object(block.meta) {
        dict.insert("meta", obj);
    }
    (dict, "ResourceLink".to_string())
}
