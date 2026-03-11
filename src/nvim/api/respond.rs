use std::rc::Rc;

use agent_client_protocol::Client;
use nvim_oxi::{Function, Object};
use tracing::{debug, instrument};
use uuid::Uuid;

use crate::nvim::autocommands::ResponseHandler;
use crate::nvim::requests::RequestHandler;

use super::Api;

impl<H, R> Api<H, R>
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: RequestHandler + 'static,
{
    #[instrument(level = "trace", skip_all)]
    pub fn respond(&self, request_id: String, response_data: nvim_oxi::Object) -> Result<(), nvim_oxi::lua::Error> {
        debug!("Respond function called with request_id: {}", request_id);
        self.runtime.block_on(async {
            let request_uuid = Uuid::parse_str(&request_id).map_err(|e| {
                nvim_oxi::lua::Error::RuntimeError(format!(
                    "Invalid request ID format '{}': {}",
                    request_id, e
                ))
            })?;

            self.request_handler
                .handle_response(&request_uuid, response_data)
                .await
                .map_err(|e| nvim_oxi::lua::Error::RuntimeError(e.to_string()))?;

            debug!("Successfully sent response for request {}", request_id);
            Ok(())
        })
    }
}

#[instrument(level = "trace", skip_all)]
pub fn respond<H, R>(api: Rc<Api<H, R>>) -> Object
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: RequestHandler + 'static,
{
    let function: Function<(String, nvim_oxi::Object), Result<(), nvim_oxi::lua::Error>> =
        Function::from_fn(
            move |(request_id, response_data): (String, nvim_oxi::Object)| {
                api.respond(request_id, response_data)
            },
        );
    function.into()
}
