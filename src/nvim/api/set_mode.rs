use agent_client_protocol::SetSessionModeRequest;
use nvim_oxi::{Function, Object, lua::Error};
use std::{cell::RefCell, rc::Rc};
use tracing::{debug, instrument};

use crate::acp::connection::ConnectionManager;

/// Tuple for two positional arguments: (session_id, mode_id)
pub type SetModeArgs = (String, String);

#[instrument(level = "trace", skip_all)]
pub fn set_mode(connection: Rc<RefCell<ConnectionManager>>) -> Object {
    let function: Function<SetModeArgs, Result<(), Error>> = Function::from_fn(
        move |(session_id, mode_id): SetModeArgs| -> Result<(), Error> {
            debug!(
                "SetMode function called with session_id: {}, mode_id: {}",
                session_id, mode_id
            );

            let request = SetSessionModeRequest::new(session_id, mode_id);

            connection
                .borrow()
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
