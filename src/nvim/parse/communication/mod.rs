mod image;
mod resource;
mod resource_link;
mod text;

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
