use agent_client_protocol::AuthenticateRequest;
use nvim_oxi::{Function, Object, lua::Error};
use std::{cell::RefCell, rc::Rc};
use tracing::{debug, instrument};

use crate::acp::connection::ConnectionManager;

#[instrument(level = "trace", skip_all)]
pub fn authenticate(connection: Rc<RefCell<ConnectionManager>>) -> Object {
    let function: Function<String, Result<(), Error>> =
        Function::from_fn(move |id: String| -> Result<(), Error> {
            debug!("Authenticate function called with: {}", id);
            let args: AuthenticateRequest = AuthenticateRequest::new(id);
            connection
                .borrow()
                .get_current_connection()
                .ok_or_else(|| {
                    Error::RuntimeError(
                        "You are not connected to an agent, call connect before \"authenticate\""
                            .to_string(),
                    )
                })?
                .authenticate(args)?;
            Ok(())
        });
    function.into()
}
