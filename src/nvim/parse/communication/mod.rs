mod image;
mod resource;
mod resource_link;
mod text;

pub use image::image_event;
pub use resource::resource_event;
pub use resource_link::resource_link_event;
pub use text::text_event;

use agent_client_protocol::{ContentBlock, Error as AcpError, Result};
use nvim_oxi::Dictionary;

pub fn communication(content: ContentBlock) -> Result<(Dictionary, String)> {
    match content {
        ContentBlock::Resource(block) => Ok(resource::resource_event(block)),
        ContentBlock::ResourceLink(block) => Ok(resource_link::resource_link_event(block)),
        ContentBlock::Image(image) => Ok(image::image_event(image)),
        ContentBlock::Text(text) => Ok(text::text_event(text)),
        ContentBlock::Audio(_) => Err(AcpError::method_not_found()),
        _ => Err(AcpError::method_not_found()),
    }
}
