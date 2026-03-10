use agent_client_protocol::{Client, RequestPermissionOutcome};
use nvim_oxi::{conversion::FromObject, Dictionary, Function, Object};
use std::rc::Rc;
use tokio::sync::Mutex;
use tracing::{debug, error, instrument};
use uuid::Uuid;

use crate::{
    acp::connection::ConnectionManager,
    nvim::autocommands::{Responder, ResponseHandler},
};

#[instrument(level = "trace", skip_all)]
pub fn respond<H: Client + ResponseHandler + Send + Sync + 'static>(
    response: Rc<Mutex<AutoCommand>>,
) -> Object {
    let function: Function<(String, Dictionary), Result<(), nvim_oxi::lua::Error>> =
        Function::from_fn(
            move |(request_id, response_data): (String, Dictionary)| -> Result<(), nvim_oxi::lua::Error> {
                debug!("Respond function called with request_id: {}", request_id);

                // Parse the request ID as UUID
                let request_uuid = Uuid::parse_str(&request_id).map_err(|e| {
                    nvim_oxi::lua::Error::RuntimeError(format!(
                        "Invalid request ID format '{}': {}",
                        request_id, e
                    ))
                })?;

                let responder = response.blocking_lock();
                
                responder.respond(&request_uuid, response_data)?;

                debug!("Successfully sent response for request {}", request_id);
                Ok(())
            },
        );
    function.into()
}

/// Deserialize a Dictionary response into RequestPermissionOutcome
///
/// Supports:
/// - { cancel = true } -> Cancelled
/// - { optionId = "..." } -> Selected(SelectedPermissionOutcome::new("..."))
fn deserialize_permission_response(
    dict: &Dictionary,
) -> Result<RequestPermissionOutcome, nvim_oxi::lua::Error> {
    use agent_client_protocol::SelectedPermissionOutcome;

    // Check for cancel flag
    if let Some(cancel_obj) = dict.get("cancel") {
        let cancel = bool::from_object(cancel_obj.clone()).map_err(|e| {
            nvim_oxi::lua::Error::RuntimeError(format!(
                "Invalid 'cancel' value - expected boolean: {}",
                e
            ))
        })?;
        if cancel {
            return Ok(RequestPermissionOutcome::Cancelled);
        }
    }

    // Check for optionId
    if let Some(option_id_obj) = dict.get("optionId") {
        let option_id = String::from_object(option_id_obj.clone()).map_err(|e| {
            nvim_oxi::lua::Error::RuntimeError(format!(
                "Invalid 'optionId' value - expected string: {}",
                e
            ))
        })?;
        return Ok(RequestPermissionOutcome::Selected(
            SelectedPermissionOutcome::new(option_id),
        ));
    }

    // Neither cancel nor optionId provided
    Err(nvim_oxi::lua::Error::RuntimeError(
        "Invalid response - must provide either 'cancel' = true or 'optionId' = \"...\""
            .to_string(),
    ))
}
