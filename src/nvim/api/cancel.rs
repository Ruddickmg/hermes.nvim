use crate::nvim::{autocommands::ResponseHandler, requests::RequestHandler};
use agent_client_protocol::{CancelNotification, Client};
use nvim_oxi::{Function, Object, lua::Error};
use std::rc::Rc;
use tracing::{debug, instrument};

use super::Api;

impl<H, R> Api<H, R>
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: RequestHandler + 'static,
{
    #[instrument(level = "trace", skip_all)]
    pub fn cancel(&self, session_id: String) -> Result<(), Error> {
        debug!("Cancel function called with session_id: {}", session_id);
        self.runtime.block_on(async {
            let notification: CancelNotification = CancelNotification::new(session_id.clone());
            let connections = self.connection.lock().await;
            let connection = connections.get_current_connection().await.ok_or_else(|| {
                Error::RuntimeError(
                    "No connection found, call the connect function first".to_string(),
                )
            })?;
            drop(connections);
            connection
                .cancel(notification)
                .await
                .map_err(|e| Error::RuntimeError(e.to_string()))?;
            self.request_handler
                .cancel_session_requests(session_id)
                .await?;
            Ok(())
        })
    }
}

#[instrument(level = "trace", skip_all)]
pub fn cancel<H, R>(api: Rc<Api<H, R>>) -> Object
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: RequestHandler + 'static,
{
    let function: Function<String, Result<(), Error>> =
        Function::from_fn(move |session_id: String| api.cancel(session_id));
    function.into()
}
