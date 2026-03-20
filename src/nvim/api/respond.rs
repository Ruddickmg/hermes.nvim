use nvim_oxi::{Function, Object};
use std::rc::Rc;
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::nvim::requests::RequestHandler;

pub type RespondArgs = (String, nvim_oxi::Object);

#[instrument(level = "trace", skip_all)]
pub fn respond<H: RequestHandler + 'static>(requests: Rc<H>) -> Object {
    let function: Function<RespondArgs, Result<(), nvim_oxi::lua::Error>> = Function::from_fn(
        move |(request_id, response_data): RespondArgs| -> Result<(), nvim_oxi::lua::Error> {
            debug!("Respond function called with request_id: {}", request_id);

            // Parse the request ID as UUID
            let request_uuid = Uuid::parse_str(&request_id).map_err(|e| {
                nvim_oxi::lua::Error::RuntimeError(format!(
                    "Invalid request ID format '{}': {}",
                    request_id, e
                ))
            })?;

            requests.handle_response(&request_uuid, response_data)?;

            debug!("Successfully sent response for request {}", request_id);
            Ok(())
        },
    );
    function.into()
}
