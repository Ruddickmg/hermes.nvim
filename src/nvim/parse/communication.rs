use agent_client_protocol::{Error as AcpError, Result, v1::schema::ContentBlock};

pub fn communication(content: ContentBlock) -> Result<String> {
    match content {
        ContentBlock::Resource(_) => Ok("Resource".to_string()),
        ContentBlock::ResourceLink(_) => Ok("ResourceLink".to_string()),
        ContentBlock::Image(_) => Ok("Image".to_string()),
        ContentBlock::Text(_) => Ok("Text".to_string()),
        ContentBlock::Audio(_) => Err(AcpError::method_not_found()),
        _ => Err(AcpError::method_not_found()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::v1::schema::{
        AudioContent, ContentBlock, EmbeddedResource, EmbeddedResourceResource, ImageContent,
        ResourceLink, TextContent, TextResourceContents,
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn communication_resource_returns_resource_string() {
        let trc = TextResourceContents::new("uri", "hello");
        let resource = EmbeddedResource::new(EmbeddedResourceResource::TextResourceContents(trc));
        let content = ContentBlock::Resource(resource);
        assert_eq!(communication(content).unwrap(), "Resource");
    }

    #[test]
    fn communication_resource_link_returns_resource_link_string() {
        let link = ResourceLink::new("https://example.com", "text/plain");
        let content = ContentBlock::ResourceLink(link);
        assert_eq!(communication(content).unwrap(), "ResourceLink");
    }

    #[test]
    fn communication_image_returns_image_string() {
        let image = ImageContent::new("data".to_string(), "image/png".to_string());
        let content = ContentBlock::Image(image);
        assert_eq!(communication(content).unwrap(), "Image");
    }

    #[test]
    fn communication_text_returns_text_string() {
        let text_content = TextContent::new("hello");
        let content = ContentBlock::Text(text_content);
        assert_eq!(communication(content).unwrap(), "Text");
    }

    #[test]
    fn communication_audio_returns_method_not_found() {
        let audio = AudioContent::new("data".to_string(), "audio/wav".to_string());
        let content = ContentBlock::Audio(audio);
        assert_eq!(
            communication(content).unwrap_err(),
            AcpError::method_not_found()
        );
    }
}
