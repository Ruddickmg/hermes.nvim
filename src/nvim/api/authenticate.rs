use agent_client_protocol::{AuthenticateRequest, Client};
use nvim_oxi::{Function, Object, lua::Error};
use std::{rc::Rc};
use tokio::sync::Mutex;

use crate::{apc::connection::ConnectionManager, nvim::autocommands::ResponseHandler};

pub fn authenticate<H: Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
    let function: Function<String, Result<(), Error>> =
        Function::from_fn(move |id: String| -> Result<(), Error> {
            let args: AuthenticateRequest = AuthenticateRequest::new(id);
            connection
                .blocking_lock()
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
