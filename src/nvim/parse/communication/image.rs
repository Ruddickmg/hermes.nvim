use crate::nvim::parse::annotations::parse_annotations;
use crate::nvim::parse::convert_metadata_to_lua_object;
use agent_client_protocol::ImageContent;
use nvim_oxi::Dictionary;

pub fn image_event(image: ImageContent) -> (Dictionary, String) {
    let mut dict: Dictionary = Dictionary::new();
    dict.insert("data", image.data);
    dict.insert("mimeType", image.mime_type);
    if let Some(uri) = image.uri {
        dict.insert("uri", uri);
    }
    if let Some(annotations) = image.annotations {
        dict.insert("annotations", parse_annotations(annotations));
    }
    if let Some(obj) = convert_metadata_to_lua_object(image.meta) {
        dict.insert("meta", obj);
    }
    (dict, "Image".to_string())
}
