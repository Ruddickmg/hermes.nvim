use agent_client_protocol::{
    AudioContent, BlobResourceContents, ContentBlock, EmbeddedResource, EmbeddedResourceResource,
    ImageContent, PromptRequest, ResourceLink, TextContent, TextResourceContents,
};
use nvim_oxi::{
    conversion::{Error as ConversionError, FromObject},
    lua::{Error, Poppable, Pushable},
    Array, Dictionary, Object, ObjectKind,
};
use std::rc::Rc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{acp::connection::ConnectionManager, nvim::autocommands::ResponseHandler};

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

impl ContentBlockType {
    fn into_content_block(self) -> ContentBlock {
        match self {
            ContentBlockType::Text { text } => ContentBlock::Text(TextContent::new(text)),
            ContentBlockType::Link { name, uri, .. } => {
                ContentBlock::ResourceLink(ResourceLink::new(name, uri))
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
        let type_str: nvim_oxi::String = dict
            .get("type")
            .ok_or_else(|| {
                ConversionError::Other("Missing 'type' field in content block".to_string())
            })?
            .clone()
            .try_into()?;

        match type_str.to_string().as_str() {
            "text" => {
                let text: nvim_oxi::String = dict
                    .get("text")
                    .ok_or_else(|| {
                        ConversionError::Other("Missing 'text' field for text content".to_string())
                    })?
                    .clone()
                    .try_into()?;
                Ok(ContentBlockType::Text {
                    text: text.to_string(),
                })
            }
            "link" => {
                let name: nvim_oxi::String = dict
                    .get("name")
                    .ok_or_else(|| {
                        ConversionError::Other("Missing 'name' field for link content".to_string())
                    })?
                    .clone()
                    .try_into()?;
                let uri: nvim_oxi::String = dict
                    .get("uri")
                    .ok_or_else(|| {
                        ConversionError::Other("Missing 'uri' field for link content".to_string())
                    })?
                    .clone()
                    .try_into()?;
                let description: Option<nvim_oxi::String> = dict
                    .get("description")
                    .and_then(|v| v.clone().try_into().ok());
                let description = description.map(|s| s.to_string());
                let mime_type: Option<nvim_oxi::String> =
                    dict.get("mimeType").and_then(|v| v.clone().try_into().ok());
                let mime_type = mime_type.map(|s| s.to_string());
                Ok(ContentBlockType::Link {
                    name: name.to_string(),
                    uri: uri.to_string(),
                    description,
                    mime_type,
                })
            }
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

                let uri: nvim_oxi::String = resource_dict
                    .get("uri")
                    .ok_or_else(|| {
                        ConversionError::Other(
                            "Missing 'uri' field in embedded resource".to_string(),
                        )
                    })?
                    .clone()
                    .try_into()?;

                let mime_type: Option<nvim_oxi::String> = resource_dict
                    .get("mimeType")
                    .and_then(|v| v.clone().try_into().ok());
                let mime_type: Option<String> = mime_type.map(|s| s.to_string());

                let resource = if let Some(text_obj) = resource_dict.get("text") {
                    let text: nvim_oxi::String = text_obj.clone().try_into()?;
                    let trc = TextResourceContents::new(uri.to_string(), text.to_string());
                    // Apply mime_type if provided
                    let trc = if let Some(mt) = mime_type {
                        trc.mime_type(mt)
                    } else {
                        trc
                    };
                    EmbeddedResourceResource::TextResourceContents(trc)
                } else if let Some(blob_obj) = resource_dict.get("blob") {
                    let blob: nvim_oxi::String = blob_obj.clone().try_into()?;
                    let brc = BlobResourceContents::new(blob.to_string(), uri.to_string());
                    // Apply mime_type if provided
                    let brc = if let Some(mt) = mime_type {
                        brc.mime_type(mt)
                    } else {
                        brc
                    };
                    EmbeddedResourceResource::BlobResourceContents(brc)
                } else {
                    return Err(ConversionError::Other(
                        "Embedded resource must have either 'text' or 'blob' field".to_string(),
                    ));
                };

                Ok(ContentBlockType::Embedded { resource })
            }
            "image" => {
                let data: nvim_oxi::String = dict
                    .get("data")
                    .ok_or_else(|| {
                        ConversionError::Other("Missing 'data' field for image content".to_string())
                    })?
                    .clone()
                    .try_into()?;
                let mime_type: nvim_oxi::String = dict
                    .get("mimeType")
                    .ok_or_else(|| {
                        ConversionError::Other(
                            "Missing 'mimeType' field for image content".to_string(),
                        )
                    })?
                    .clone()
                    .try_into()?;
                let uri: Option<nvim_oxi::String> =
                    dict.get("uri").and_then(|v| v.clone().try_into().ok());
                let uri = uri.map(|s| s.to_string());
                Ok(ContentBlockType::Image {
                    data: data.to_string(),
                    mime_type: mime_type.to_string(),
                    uri,
                })
            }
            "audio" => {
                let data: nvim_oxi::String = dict
                    .get("data")
                    .ok_or_else(|| {
                        ConversionError::Other("Missing 'data' field for audio content".to_string())
                    })?
                    .clone()
                    .try_into()?;
                let mime_type: nvim_oxi::String = dict
                    .get("mimeType")
                    .ok_or_else(|| {
                        ConversionError::Other(
                            "Missing 'mimeType' field for audio content".to_string(),
                        )
                    })?
                    .clone()
                    .try_into()?;
                Ok(ContentBlockType::Audio {
                    data: data.to_string(),
                    mime_type: mime_type.to_string(),
                })
            }
            other => Err(ConversionError::Other(format!(
                "Unknown content type: {}. Expected one of: text, link, embedded, image, audio",
                other
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
                    _ => {}
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
pub fn prompt<H: agent_client_protocol::Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
    use nvim_oxi::Function;

    let function: Function<PromptArgs, Result<(), Error>> =
        Function::from_fn(move |(session_id, content): PromptArgs| {
            debug!("Prompt function called with session_id: {}", session_id);

            // Convert content blocks to ContentBlock
            let content_blocks: Vec<ContentBlock> = content
                .into_vec()
                .into_iter()
                .map(|c| c.into_content_block())
                .collect();

            let request = PromptRequest::new(session_id, content_blocks);

            connection
                .blocking_lock()
                .get_current_connection()
                .ok_or_else(|| {
                    Error::RuntimeError(
                        "No connection found, call the connect function first".to_string(),
                    )
                })?
                .prompt(request)
                .map_err(|e| Error::RuntimeError(e.to_string()))?;

            Ok(())
        });
    function.into()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let content_block = block.into_content_block();
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
        let content_block = block.into_content_block();
        assert!(matches!(content_block, ContentBlock::ResourceLink(_)));
    }

    #[test]
    fn test_convert_embedded_to_content_block() {
        let resource = EmbeddedResourceResource::TextResourceContents(TextResourceContents::new(
            "file:///path/to/file.txt".to_string(),
            "content".to_string(),
        ));
        let block = ContentBlockType::Embedded { resource };
        let content_block = block.into_content_block();
        assert!(matches!(content_block, ContentBlock::Resource(_)));
    }

    #[test]
    fn test_convert_image_without_uri_to_content_block() {
        let block = ContentBlockType::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
            uri: None,
        };
        let content_block = block.into_content_block();
        assert!(matches!(content_block, ContentBlock::Image(_)));
    }

    #[test]
    fn test_convert_image_with_uri_to_content_block() {
        let block = ContentBlockType::Image {
            data: "base64data".to_string(),
            mime_type: "image/png".to_string(),
            uri: Some("file:///path/to/image.png".to_string()),
        };
        let content_block = block.into_content_block();
        assert!(matches!(content_block, ContentBlock::Image(_)));
    }

    #[test]
    fn test_convert_audio_to_content_block() {
        let block = ContentBlockType::Audio {
            data: "base64data".to_string(),
            mime_type: "audio/wav".to_string(),
        };
        let content_block = block.into_content_block();
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
