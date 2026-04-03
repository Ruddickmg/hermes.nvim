use agent_client_protocol::{
    AudioContent, BlobResourceContents, ContentBlock, EmbeddedResource, EmbeddedResourceResource,
    ImageContent, PromptRequest, ResourceLink, TextContent, TextResourceContents,
};
use nvim_oxi::{
    Array, Dictionary, Function, Object, ObjectKind,
    conversion::{Error as ConversionError, FromObject},
    lua::{Error, Poppable, Pushable},
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, error, instrument};

use crate::{PluginState, acp::connection::ConnectionManager};

/// Extracts a required string field from a Lua dictionary.
fn required_string(dict: &Dictionary, key: &str) -> Result<String, ConversionError> {
    let value: nvim_oxi::String = dict
        .get(key)
        .ok_or_else(|| ConversionError::Other(format!("Missing '{key}' field")))?
        .clone()
        .try_into()?;
    Ok(value.to_string())
}

/// Extracts an optional string field from a Lua dictionary.
fn optional_string(dict: &Dictionary, key: &str) -> Result<Option<String>, ConversionError> {
    if let Some(result) = dict.get(key).map(|s| {
        let a: Result<nvim_oxi::String, ConversionError> = s.clone().try_into();
        a.map(|s| s.to_string())
    }) {
        result.map(Some)
    } else {
        Ok(None)
    }
}

/// Represents the content block type from Lua
#[derive(Debug, Clone)]
pub enum ContentBlockType {
    Text {
        text: String,
    },
    Link {
        name: String,
        uri: String,
        description: Option<String>,
        mime_type: Option<String>,
    },
    Embedded {
        resource: EmbeddedResourceResource,
    },
    Image {
        data: String,
        mime_type: String,
        uri: Option<String>,
    },
    Audio {
        data: String,
        mime_type: String,
    },
}

impl From<ContentBlockType> for ContentBlock {
    fn from(block: ContentBlockType) -> Self {
        match block {
            ContentBlockType::Text { text } => ContentBlock::Text(TextContent::new(text)),
            ContentBlockType::Link {
                name,
                uri,
                description,
                mime_type,
            } => {
                let mut link = ResourceLink::new(name, uri);
                link.description = description;
                link.mime_type = mime_type;
                ContentBlock::ResourceLink(link)
            }
            ContentBlockType::Embedded { resource } => {
                ContentBlock::Resource(EmbeddedResource::new(resource))
            }
            ContentBlockType::Image {
                data,
                mime_type,
                uri,
            } => {
                let image = ImageContent::new(data, mime_type);
                if let Some(u) = uri {
                    ContentBlock::Image(image.uri(u))
                } else {
                    ContentBlock::Image(image)
                }
            }
            ContentBlockType::Audio { data, mime_type } => {
                ContentBlock::Audio(AudioContent::new(data, mime_type))
            }
        }
    }
}

impl FromObject for ContentBlockType {
    fn from_object(obj: Object) -> Result<Self, ConversionError> {
        let dict: Dictionary = obj.try_into()?;
        let type_str = required_string(&dict, "type")?;

        match type_str.as_str() {
            "text" => Ok(ContentBlockType::Text {
                text: required_string(&dict, "text")?,
            }),
            "link" => Ok(ContentBlockType::Link {
                name: required_string(&dict, "name")?,
                uri: required_string(&dict, "uri")?,
                description: optional_string(&dict, "description")?,
                mime_type: optional_string(&dict, "mimeType")?,
            }),
            "embedded" => {
                let resource_dict: Dictionary = dict
                    .get("resource")
                    .ok_or_else(|| {
                        ConversionError::Other(
                            "Missing 'resource' field for embedded content".to_string(),
                        )
                    })?
                    .clone()
                    .try_into()?;

                let uri = required_string(&resource_dict, "uri")?;
                let mime_type = optional_string(&resource_dict, "mimeType")?;

                let resource = if let Some(text_obj) = resource_dict.get("text") {
                    let text: nvim_oxi::String = text_obj.clone().try_into()?;
                    let trc = TextResourceContents::new(uri, text.to_string());
                    let trc = match mime_type {
                        Some(mt) => trc.mime_type(mt),
                        None => trc,
                    };
                    EmbeddedResourceResource::TextResourceContents(trc)
                } else if let Some(blob_obj) = resource_dict.get("blob") {
                    let blob: nvim_oxi::String = blob_obj.clone().try_into()?;
                    let brc = BlobResourceContents::new(blob.to_string(), uri);
                    let brc = match mime_type {
                        Some(mt) => brc.mime_type(mt),
                        None => brc,
                    };
                    EmbeddedResourceResource::BlobResourceContents(brc)
                } else {
                    return Err(ConversionError::Other(
                        "Embedded resource must have either 'text' or 'blob' field".to_string(),
                    ));
                };

                Ok(ContentBlockType::Embedded { resource })
            }
            "image" => Ok(ContentBlockType::Image {
                data: required_string(&dict, "data")?,
                mime_type: required_string(&dict, "mimeType")?,
                uri: optional_string(&dict, "uri")?,
            }),
            "audio" => Ok(ContentBlockType::Audio {
                data: required_string(&dict, "data")?,
                mime_type: required_string(&dict, "mimeType")?,
            }),
            other => Err(ConversionError::Other(format!(
                "Unknown content type: {other}. Expected one of: text, link, embedded, image, audio"
            ))),
        }
    }
}

impl Poppable for ContentBlockType {
    unsafe fn pop(lua_state: *mut nvim_oxi::lua::ffi::State) -> Result<Self, Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Self::from_object(obj).map_err(|e| Error::RuntimeError(e.to_string()))
    }
}

impl Pushable for ContentBlockType {
    unsafe fn push(self, state: *mut nvim_oxi::lua::ffi::State) -> Result<i32, Error> {
        let dict = match self {
            ContentBlockType::Text { text } => {
                let mut d = Dictionary::new();
                d.insert("type", "text");
                d.insert("text", text);
                d
            }
            ContentBlockType::Link {
                name,
                uri,
                description,
                mime_type,
            } => {
                let mut d = Dictionary::new();
                d.insert("type", "link");
                d.insert("name", name);
                d.insert("uri", uri);
                if let Some(desc) = description {
                    d.insert("description", desc);
                }
                if let Some(mt) = mime_type {
                    d.insert("mimeType", mt);
                }
                d
            }
            ContentBlockType::Embedded { resource } => {
                let mut d = Dictionary::new();
                d.insert("type", "embedded");
                let mut res_dict = Dictionary::new();
                match resource {
                    EmbeddedResourceResource::TextResourceContents(trc) => {
                        res_dict.insert("uri", trc.uri);
                        res_dict.insert("text", trc.text);
                        if let Some(mt) = trc.mime_type {
                            res_dict.insert("mimeType", mt);
                        }
                    }
                    EmbeddedResourceResource::BlobResourceContents(brc) => {
                        res_dict.insert("uri", brc.uri);
                        res_dict.insert("blob", brc.blob);
                        if let Some(mt) = brc.mime_type {
                            res_dict.insert("mimeType", mt);
                        }
                    }
                    resource => {
                        return Err(Error::RuntimeError(format!(
                            "Unsupported embedded resource type: {:?}",
                            resource
                        )));
                    }
                }
                d.insert("resource", res_dict);
                d
            }
            ContentBlockType::Image {
                data,
                mime_type,
                uri,
            } => {
                let mut d = Dictionary::new();
                d.insert("type", "image");
                d.insert("data", data);
                d.insert("mimeType", mime_type);
                if let Some(u) = uri {
                    d.insert("uri", u);
                }
                d
            }
            ContentBlockType::Audio { data, mime_type } => {
                let mut d = Dictionary::new();
                d.insert("type", "audio");
                d.insert("data", data);
                d.insert("mimeType", mime_type);
                d
            }
        };
        unsafe { Object::from(dict).push(state) }
    }
}

/// Wraps either a single content block or an array of content blocks
#[derive(Debug, Clone)]
pub enum PromptContent {
    Single(ContentBlockType),
    Multiple(Vec<ContentBlockType>),
}

impl PromptContent {
    fn into_vec(self) -> Vec<ContentBlockType> {
        match self {
            PromptContent::Single(block) => vec![block],
            PromptContent::Multiple(blocks) => blocks,
        }
    }
}

impl FromObject for PromptContent {
    fn from_object(obj: Object) -> Result<Self, ConversionError> {
        match obj.kind() {
            ObjectKind::Array => {
                let array = unsafe { obj.into_array_unchecked() };
                let blocks = array
                    .into_iter()
                    .map(ContentBlockType::from_object)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(PromptContent::Multiple(blocks))
            }
            ObjectKind::Dictionary => {
                let block = ContentBlockType::from_object(obj)?;
                Ok(PromptContent::Single(block))
            }
            other => Err(ConversionError::FromWrongType {
                expected: "Array or Dictionary",
                actual: other.as_static(),
            }),
        }
    }
}

impl Poppable for PromptContent {
    unsafe fn pop(lua_state: *mut nvim_oxi::lua::ffi::State) -> Result<Self, Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Self::from_object(obj).map_err(|e| Error::RuntimeError(e.to_string()))
    }
}

impl Pushable for PromptContent {
    unsafe fn push(self, state: *mut nvim_oxi::lua::ffi::State) -> Result<i32, Error> {
        match self {
            PromptContent::Single(block) => unsafe { block.push(state) },
            PromptContent::Multiple(blocks) => {
                let content_array: Array = blocks
                    .into_iter()
                    .map(|c| unsafe {
                        c.push(state)?;
                        Object::pop(state).map_err(|e| Error::RuntimeError(e.to_string()))
                    })
                    .collect::<Result<Array, _>>()?;
                unsafe { Object::from(content_array).push(state) }
            }
        }
    }
}

/// Tuple for two positional arguments: (session_id, content)
pub type PromptArgs = (String, PromptContent);

#[instrument(level = "trace", skip_all)]
pub fn prompt(
    connection: Rc<RefCell<ConnectionManager>>,
    state: Arc<Mutex<PluginState>>,
) -> Object {
    let function: Function<PromptArgs, Result<(), Error>> =
        Function::from_fn(move |(session_id, content): PromptArgs| {
            debug!("Prompt function called with session_id: {}", session_id);
            let state = state.blocking_lock();
            let agent_info = state.agent_info.clone();
            drop(state);
            let content_blocks: Vec<ContentBlock> = content
                .into_vec()
                .into_iter()
                .map(Into::into)
                .filter(|content| match content {
                    ContentBlock::Image(_) => agent_info.can_send_images(),
                    ContentBlock::Audio(_) => agent_info.can_send_audio(),
                    ContentBlock::Resource(_) => agent_info.can_send_embedded_context(),
                    _ => true
                })
                .collect();

            let request = PromptRequest::new(session_id, content_blocks);

            let conn = match connection.borrow().get_current_connection() {
                Some(c) => c,
                None => {
                    error!("No connection found, call the connect function first");
                    return Ok(());
                }
            };

            if let Err(e) = conn.prompt(request) {
                error!("Error sending prompt: {:?}", e);
            }

            Ok(())
        });
    function.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // Helper function to create a text content dictionary
    fn create_text_dict(text: &str) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("type", "text");
        dict.insert("text", text);
        dict
    }

    // Helper function to create a link content dictionary
    fn create_link_dict(name: &str, uri: &str) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("type", "link");
        dict.insert("name", name);
        dict.insert("uri", uri);
        dict
    }

    // Helper function to create a link content dictionary with optional fields
    fn create_link_dict_with_optional(
        name: &str,
        uri: &str,
        description: Option<&str>,
        mime_type: Option<&str>,
    ) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("type", "link");
        dict.insert("name", name);
        dict.insert("uri", uri);
        if let Some(desc) = description {
            dict.insert("description", desc);
        }
        if let Some(mt) = mime_type {
            dict.insert("mimeType", mt);
        }
        dict
    }

    // Helper function to create an embedded text resource dictionary
    fn create_embedded_text_dict(uri: &str, text: &str, mime_type: Option<&str>) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("type", "embedded");
        let mut res_dict = Dictionary::new();
        res_dict.insert("uri", uri);
        res_dict.insert("text", text);
        if let Some(mt) = mime_type {
            res_dict.insert("mimeType", mt);
        }
        dict.insert("resource", res_dict);
        dict
    }

    // Helper function to create an embedded blob resource dictionary
    fn create_embedded_blob_dict(uri: &str, blob: &str, mime_type: Option<&str>) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("type", "embedded");
        let mut res_dict = Dictionary::new();
        res_dict.insert("uri", uri);
        res_dict.insert("blob", blob);
        if let Some(mt) = mime_type {
            res_dict.insert("mimeType", mt);
        }
        dict.insert("resource", res_dict);
        dict
    }

    // Helper function to create an image content dictionary
    fn create_image_dict(data: &str, mime_type: &str, uri: Option<&str>) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("type", "image");
        dict.insert("data", data);
        dict.insert("mimeType", mime_type);
        if let Some(u) = uri {
            dict.insert("uri", u);
        }
        dict
    }

    // Helper function to create an audio content dictionary
    fn create_audio_dict(data: &str, mime_type: &str) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("type", "audio");
        dict.insert("data", data);
        dict.insert("mimeType", mime_type);
        dict
    }

    // Helper function to create a single prompt content array
    fn create_single_content_array(text: &str) -> Array {
        Array::from_iter(vec![Object::from(create_text_dict(text))])
    }

    // Helper function to create a multiple prompt content array
    fn create_multiple_content_array(texts: &[&str]) -> Array {
        let objects: Vec<Object> = texts
            .iter()
            .map(|text| Object::from(create_text_dict(text)))
            .collect();
        Array::from_iter(objects)
    }

    #[cfg(test)]
    mod proptest_conversions {
        use super::*;
        use proptest::prelude::*;

        // Strategy for generating valid text strings
        fn arb_text_string() -> impl Strategy<Value = String> {
            "[a-zA-Z0-9.,;:!?\\s]*".prop_map(|s| s.to_string())
        }

        // Strategy for generating valid uri strings
        fn arb_uri_string() -> impl Strategy<Value = String> {
            "file:///[a-zA-Z0-9._/-]*".prop_map(|s| s.to_string())
        }

        // Strategy for generating valid name strings
        fn arb_name_string() -> impl Strategy<Value = String> {
            "[a-zA-Z][a-zA-Z0-9_-]*".prop_map(|s| s.to_string())
        }

        proptest! {
            #[test]
            fn test_parse_text_from_lua_never_panics(text in arb_text_string()) {
                // Property: Parsing text content from Lua dict never panics
                let dict = create_text_dict(&text);
                let obj = Object::from(dict);
                let _result = ContentBlockType::from_object(obj);
                // Test passes if no panic occurs
            }

            #[test]
            fn test_parse_link_from_lua_never_panics(
                name in arb_name_string(),
                uri in arb_uri_string()
            ) {
                // Property: Parsing link content from Lua dict never panics
                let dict = create_link_dict(&name, &uri);
                let obj = Object::from(dict);
                let _result = ContentBlockType::from_object(obj);
                // Test passes if no panic occurs
            }

            #[test]
            fn test_parse_single_content_from_lua_never_panics(text in arb_text_string()) {
                // Property: Parsing single prompt content from Lua array never panics
                let arr = create_single_content_array(&text);
                let obj = Object::from(arr);
                let _result = PromptContent::from_object(obj);
                // Test passes if no panic occurs
            }

            #[test]
            fn test_parse_multiple_content_from_lua_never_panics(
                texts in prop::collection::vec(arb_text_string(), 1..5)
            ) {
                // Property: Parsing multiple prompt content from Lua array never panics
                let texts_ref: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
                let arr = create_multiple_content_array(&texts_ref);
                let obj = Object::from(arr);
                let _result = PromptContent::from_object(obj);
                // Test passes if no panic occurs
            }
        }
    }

    // ContentBlockType::FromObject Tests - Text

    #[test]
    fn test_parse_text_success() {
        let dict = create_text_dict("Hello, world!");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_text_missing_type_field_error() {
        let mut dict = Dictionary::new();
        dict.insert("text", "Hello, world!");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_text_missing_text_field_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "text");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    // ContentBlockType::FromObject Tests - Link

    #[test]
    fn test_parse_link_success() {
        let dict = create_link_dict("example", "file:///path/to/file.txt");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_link_with_optional_fields() {
        let dict = create_link_dict_with_optional(
            "example",
            "file:///path/to/file.txt",
            Some("An example file"),
            Some("text/plain"),
        );
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_link_missing_name_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "link");
        dict.insert("uri", "file:///path/to/file.txt");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_link_missing_uri_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "link");
        dict.insert("name", "example");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    // ContentBlockType::FromObject Tests - Embedded Text Resource

    #[test]
    fn test_parse_embedded_text_resource_success() {
        let dict =
            create_embedded_text_dict("file:///path/to/file.txt", "file contents here", None);
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_embedded_text_with_mime_type() {
        let dict = create_embedded_text_dict(
            "file:///path/to/file.py",
            "def hello(): pass",
            Some("text/x-python"),
        );
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_embedded_missing_resource_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "embedded");
        dict.insert("uri", "file:///path/to/file.txt");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_embedded_missing_uri_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "embedded");
        let mut res_dict = Dictionary::new();
        res_dict.insert("text", "file contents");
        dict.insert("resource", res_dict);
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_embedded_missing_text_and_blob_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "embedded");
        let mut res_dict = Dictionary::new();
        res_dict.insert("uri", "file:///path/to/file.txt");
        dict.insert("resource", res_dict);
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    // ContentBlockType::FromObject Tests - Embedded Blob Resource

    #[test]
    fn test_parse_embedded_blob_resource_success() {
        let dict = create_embedded_blob_dict("file:///path/to/file.bin", "base64data", None);
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_embedded_blob_with_mime_type() {
        let dict = create_embedded_blob_dict(
            "file:///path/to/file.pdf",
            "base64data",
            Some("application/pdf"),
        );
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    // ContentBlockType::FromObject Tests - Image

    #[test]
    fn test_parse_image_success() {
        let dict = create_image_dict("base64imagedata", "image/png", None);
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_image_with_uri() {
        let dict = create_image_dict(
            "base64imagedata",
            "image/png",
            Some("file:///path/to/image.png"),
        );
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_image_missing_data_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "image");
        dict.insert("mimeType", "image/png");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_image_missing_mime_type_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "image");
        dict.insert("data", "base64imagedata");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    // ContentBlockType::FromObject Tests - Audio

    #[test]
    fn test_parse_audio_success() {
        let dict = create_audio_dict("base64audiodata", "audio/wav");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_audio_missing_data_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "audio");
        dict.insert("mimeType", "audio/wav");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_audio_missing_mime_type_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "audio");
        dict.insert("data", "base64audiodata");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    // ContentBlockType::FromObject Tests - Unknown Type

    #[test]
    fn test_parse_unknown_type_error() {
        let mut dict = Dictionary::new();
        dict.insert("type", "unknown");
        dict.insert("data", "some data");
        let obj = Object::from(dict);
        let result = ContentBlockType::from_object(obj);
        assert!(result.is_err());
    }

    // ContentBlockType Conversion Tests (into_content_block)

    #[test]
    fn test_convert_text_to_content_block() {
        let block = ContentBlockType::Text {
            text: "Hello, world!".to_string(),
        };
        let content_block = block.into();
        assert!(matches!(content_block, ContentBlock::Text(_)));
    }

    #[test]
    fn test_convert_link_to_content_block() {
        let block = ContentBlockType::Link {
            name: "example".to_string(),
            uri: "file:///path/to/file.txt".to_string(),
            description: None,
            mime_type: None,
        };
        let content_block = block.into();
        assert!(matches!(content_block, ContentBlock::ResourceLink(_)));
    }

    #[test]
    fn test_convert_embedded_to_content_block() {
        let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
            "file:///path/to/file.txt".to_string(),
            "content".to_string(),
        ));
        let block = ContentBlockType::Embedded { resource };
        let content_block = block.into();
        assert!(matches!(content_block, ContentBlock::Resource(_)));
    }

    #[test]
    fn test_convert_image_without_uri_to_content_block() {
        let block = ContentBlockType::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
            uri: None,
        };
        let content_block = block.into();
        assert!(matches!(content_block, ContentBlock::Image(_)));
    }

    #[test]
    fn test_convert_image_with_uri_to_content_block() {
        let block = ContentBlockType::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
            uri: Some("file:///path/to/image.png".to_string()),
        };
        let content_block = block.into();
        assert!(matches!(content_block, ContentBlock::Image(_)));
    }

    #[test]
    fn test_convert_audio_to_content_block() {
        let block = ContentBlockType::Audio {
            data: "base64data".to_string(),
            mime_type: "audio/wav".to_string(),
        };
        let content_block = block.into();
        assert!(matches!(content_block, ContentBlock::Audio(_)));
    }

    // PromptContent Parsing Tests (FromObject)

    #[test]
    fn test_parse_prompt_content_single() {
        let dict = create_text_dict("Hello!");
        let obj = Object::from(dict);
        let result = PromptContent::from_object(obj);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PromptContent::Single(_)));
    }

    #[test]
    fn test_parse_prompt_content_multiple() {
        let array = Array::from_iter(vec![
            Object::from(create_text_dict("Hello!")),
            Object::from(create_link_dict("file", "file:///test.txt")),
        ]);
        let obj = Object::from(array);
        let result = PromptContent::from_object(obj);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PromptContent::Multiple(_)));
    }

    #[test]
    fn test_parse_prompt_content_invalid_type_error() {
        let obj = Object::from("invalid string");
        let result = PromptContent::from_object(obj);
        assert!(result.is_err());
    }

    // PromptContent Conversion Tests (into_vec)

    #[test]
    fn test_single_content_into_vec() {
        let content = PromptContent::Single(ContentBlockType::Text {
            text: "Hello!".to_string(),
        });
        let vec = content.into_vec();
        assert_eq!(vec.len(), 1);
    }

    #[test]
    fn test_multiple_content_into_vec() {
        let content = PromptContent::Multiple(vec![
            ContentBlockType::Text {
                text: "Hello!".to_string(),
            },
            ContentBlockType::Text {
                text: "World!".to_string(),
            },
        ]);
        let vec = content.into_vec();
        assert_eq!(vec.len(), 2);
    }
}
