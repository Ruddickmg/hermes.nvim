use agent_client_protocol::CancelNotification;
use nvim_oxi::{Function, Object, lua::Error};
use std::{cell::RefCell, rc::Rc};
use tracing::{debug, error, instrument};

use crate::{acp::connection::ConnectionManager, nvim::requests::RequestHandler};

#[instrument(level = "trace", skip_all)]
pub fn cancel<R: RequestHandler + 'static>(
    connection: Rc<RefCell<ConnectionManager>>,
    request_handler: Rc<R>,
) -> Object {
    let function: Function<String, Result<(), Error>> =
        Function::from_fn(move |session_id: String| -> Result<(), Error> {
            debug!("Cancel function called with session_id: {}", session_id);

            let conn = match connection.borrow().get_current_connection() {
                Some(c) => c,
                None => {
                    error!("No connection found for cancel, call the connect function first");
                    return Ok(());
                }
            };

            let notification: CancelNotification = CancelNotification::new(session_id.clone());
            if let Err(e) = conn.cancel(notification) {
                error!("Error cancelling session {}: {:?}", session_id, e);
            }

            if let Err(e) = request_handler.cancel_session_requests(session_id) {
                error!("Error cancelling session requests: {:?}", e);
            }

            Ok(())
        });
    function.into()
}
