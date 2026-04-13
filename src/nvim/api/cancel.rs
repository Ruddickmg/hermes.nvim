use crate::acp::{Result, error::Error};
use agent_client_protocol::CancelNotification;
use nvim_oxi::Object;
use std::{cell::RefCell, rc::Rc};
use tracing::{instrument, trace};

use crate::{
    acp::connection::ConnectionManager, api::create_api_method, nvim::requests::RequestHandler,
};

#[instrument(level = "trace", skip_all)]
pub fn cancel<R: RequestHandler + 'static>(
    connection: Rc<RefCell<ConnectionManager>>,
    request_handler: Rc<R>,
) -> Object {
    create_api_method(move |session_id: String| -> Result<()> {
        trace!("Cancel api methpd called with session_id: {}", session_id);

        let borrowed_connection = connection.try_borrow_mut()?;
        let conn = borrowed_connection
            .get_current_connection()
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        conn.cancel(CancelNotification::new(session_id.clone()))?;

        drop(borrowed_connection);

        request_handler.cancel_session_requests(session_id)
    })
}
