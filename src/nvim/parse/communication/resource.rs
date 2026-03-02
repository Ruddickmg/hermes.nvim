use crate::nvim::parse::annotations::parse_annotations;
use agent_client_protocol::{EmbeddedResource, EmbeddedResourceResource};
use nvim_oxi::Dictionary;

pub fn resource_event(block: EmbeddedResource) -> (Dictionary, String) {
    let mut dict: Dictionary = Dictionary::new();

    let resource_dict = match block.resource {
        EmbeddedResourceResource::TextResourceContents(contents) => {
            let mut inner = Dictionary::new();
            inner.insert("text", contents.text);
            inner.insert("uri", contents.uri);
            if let Some(mime_type) = contents.mime_type {
                inner.insert("mimeType", mime_type);
            }
            inner
        }
        EmbeddedResourceResource::BlobResourceContents(contents) => {
            let mut inner = Dictionary::new();
            inner.insert("blob", contents.blob);
            inner.insert("uri", contents.uri);
            if let Some(mime_type) = contents.mime_type {
                inner.insert("mimeType", mime_type);
            }
            inner
        }
        _ => Dictionary::new(),
    };
    dict.insert("resource", resource_dict);

    if let Some(annotations) = block.annotations {
        dict.insert("annotations", parse_annotations(annotations));
    }
    if let Some(meta) = block.meta {
        dict.insert("meta", format!("{:?}", meta));
    }
    (dict, "Resource".to_string())
}
