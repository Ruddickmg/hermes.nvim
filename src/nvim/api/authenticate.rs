use agent_client_protocol::AuthenticateRequest;
use nvim_oxi::{Function, Object, lua::Error};
use std::{cell::RefCell, rc::Rc};
use tracing::{debug, error, instrument};

use crate::acp::connection::ConnectionManager;

#[instrument(level = "trace", skip_all)]
pub fn authenticate(connection: Rc<RefCell<ConnectionManager>>) -> Object {
    let function: Function<String, Result<(), Error>> =
        Function::from_fn(move |id: String| -> Result<(), Error> {
            debug!("Authenticate function called with: {}", id);
            let args: AuthenticateRequest = AuthenticateRequest::new(id);
            let conn = match connection.borrow().get_current_connection() {
                Some(c) => c,
                None => {
                    error!(
                        "You are not connected to an agent, call connect before \"authenticate\""
                    );
                    return Ok(());
                }
            };
            if let Err(e) = conn.authenticate(args) {
                error!("Error during authenticate: {:?}", e);
            }
            Ok(())
        });
    function.into()
}
