use agent_client_protocol::{Client, SetSessionModeRequest};
use nvim_oxi::{Function, Object, lua::Error};
use std::rc::Rc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{acp::connection::ConnectionManager, nvim::autocommands::ResponseHandler};

/// Tuple for two positional arguments: (session_id, mode_id)
pub type SetModeArgs = (String, String);

#[instrument(level = "trace", skip_all)]
pub fn set_mode<H: Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
    let function: Function<SetModeArgs, Result<(), Error>> = Function::from_fn(
        move |(session_id, mode_id): SetModeArgs| -> Result<(), Error> {
            debug!(
                "SetMode function called with session_id: {}, mode_id: {}",
                session_id, mode_id
            );

            let request = SetSessionModeRequest::new(session_id, mode_id);

            connection
                .blocking_lock()
                .get_current_connection()
                .ok_or_else(|| {
                    Error::RuntimeError(
                        "No connection found, call the connect function first".to_string(),
                    )
                })?
                .set_mode(request)
                .map_err(|e| Error::RuntimeError(e.to_string()))?;

            Ok(())
        },
    );
    function.into()
}
