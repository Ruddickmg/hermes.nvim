use agent_client_protocol::{Client, SetSessionModeRequest};
use nvim_oxi::{Function, Object, lua::Error};
use std::rc::Rc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{acp::connection::ConnectionManager, nvim::autocommands::ResponseHandler};

use super::Api;

/// Tuple for two positional arguments: (session_id, mode_id)
pub type SetModeArgs = (String, String);

impl<H, R> Api<H, R>
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: crate::nvim::requests::RequestHandler + 'static,
{
    #[instrument(level = "trace", skip_all)]
    pub fn set_mode(&self, session_id: String, mode_id: String) -> Result<(), Error> {
        debug!(
            "SetMode function called with session_id: {}, mode_id: {}",
            session_id, mode_id
        );
        self.runtime.block_on(async {
            let request = SetSessionModeRequest::new(session_id, mode_id);
            let connections = self.connection.lock().await;
            let connection = connections.get_current_connection().await.ok_or_else(|| {
                Error::RuntimeError(
                    "No connection found, call the connect function first".to_string(),
                )
            })?;
            drop(connections);
            connection
                .set_mode(request)
                .await
                .map_err(|e| Error::RuntimeError(e.to_string()))?;

            Ok(())
        })
    }
}

#[instrument(level = "trace", skip_all)]
pub fn set_mode<H, R>(api: Rc<Api<H, R>>) -> Object
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: crate::nvim::requests::RequestHandler + 'static,
{
    let function: Function<SetModeArgs, Result<(), Error>> =
        Function::from_fn(move |(session_id, mode_id)| api.set_mode(session_id, mode_id));
    function.into()
}
