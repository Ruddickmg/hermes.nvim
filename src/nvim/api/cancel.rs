use agent_client_protocol::{CancelNotification, Client};
use nvim_oxi::{Function, Object, lua::Error};
use std::{rc::Rc, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{acp::connection::ConnectionManager, nvim::{autocommands::ResponseHandler, requests::RequestHandler}};

#[instrument(level = "trace", skip_all)]
pub fn cancel<H: Client + ResponseHandler + Send + Sync + 'static, R: RequestHandler + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
    request_handler: Arc<R>,
) -> Object {
    let function: Function<String, Result<(), Error>> =
        Function::from_fn(move |session_id: String| -> Result<(), Error> {
            debug!("Cancel function called with session_id: {}", session_id);
            let notification: CancelNotification = CancelNotification::new(session_id.clone());
            connection
                .blocking_lock()
                .get_current_connection()
                .ok_or_else(|| {
                    Error::RuntimeError(
                        "No connection found, call the connect function first".to_string(),
                    )
                })?
                .cancel(notification)
                .map_err(|e| Error::RuntimeError(e.to_string()))?;
            request_handler.cancel_session_requests(session_id)?;
            Ok(())
        });
    function.into()
}
