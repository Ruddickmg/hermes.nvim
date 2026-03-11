use std::rc::Rc;

use agent_client_protocol::{AuthenticateRequest, Client};
use nvim_oxi::{Function, Object, lua::Error};
use tracing::{debug, instrument};
use crate::nvim::autocommands::ResponseHandler;

use super::Api;

impl<H, R> Api<H, R>
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: crate::nvim::requests::RequestHandler + 'static,
{
    #[instrument(level = "trace", skip_all)]
    pub fn authenticate(&self, id: String) -> Result<(), Error> {
        debug!("Authenticate function called with: {}", id);
        self.runtime.block_on(async {
            let args: AuthenticateRequest = AuthenticateRequest::new(id);
            let connections = self.connection.lock().await;
            let connection = connections.get_current_connection().await.ok_or_else(|| {
                Error::RuntimeError(
                    "You are not connected to an agent, call connect before \"authenticate\""
                        .to_string(),
                )
            })?;
            drop(connections);
            connection.authenticate(args).await?;
            Ok(())
        })
    }
}

#[instrument(level = "trace", skip_all)]
pub fn authenticate<H, R>(api: Rc<Api<H, R>>) -> Object
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: crate::nvim::requests::RequestHandler + 'static,
{
    let function: Function<String, Result<(), Error>> =
        Function::from_fn(move |id| api.authenticate(id));
    function.into()
}
