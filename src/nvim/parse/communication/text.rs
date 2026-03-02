use crate::nvim::parse::annotations::parse_annotations;
use agent_client_protocol::TextContent;
use nvim_oxi::Dictionary;

pub fn text_event(text: TextContent) -> (Dictionary, String) {
    let mut dict: Dictionary = Dictionary::new();
    dict.insert("text", text.text);
    if let Some(annotations) = text.annotations {
        dict.insert("annotations", parse_annotations(annotations));
    }
    if let Some(meta) = text.meta {
        dict.insert("meta", format!("{:?}", meta));
    }
    (dict, "Text".to_string())
}
