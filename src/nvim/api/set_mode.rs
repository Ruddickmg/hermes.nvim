use agent_client_protocol::SetSessionModeRequest;
use nvim_oxi::{Function, Object, lua::Error};
use std::{cell::RefCell, rc::Rc};
use tracing::{debug, error, instrument};

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

            let conn = match connection.borrow().get_current_connection() {
                Some(c) => c,
                None => {
                    error!("No connection found, call the connect function first");
                    return Ok(());
                }
            };

            if let Err(e) = conn.set_mode(request) {
                error!("Error setting mode: {:?}", e);
            }

            Ok(())
        },
    );
    function.into()
}
