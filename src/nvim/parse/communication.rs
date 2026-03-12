use agent_client_protocol::{ContentBlock, Error as AcpError, Result};

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
    use agent_client_protocol::{ContentBlock, TextContent};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_communication_text() {
        // Assuming TextContent has new or similar constructor
        let text_content = TextContent::new("hello");
        let content = ContentBlock::Text(text_content);
        assert_eq!(communication(content).unwrap(), "Text");
    }
}
