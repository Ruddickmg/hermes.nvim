use agent_client_protocol::{AuthenticateRequest, Client};
use nvim_oxi::{Function, Object, lua::Error};
use std::sync::{Arc, Mutex};

use crate::{apc::connection::ConnectionManager, nvim::producer::AutoCommands};

pub fn create_lua_authenticate<H: Client>(
    connection: Arc<Mutex<ConnectionManager<AutoCommands>>>,
) -> Object {
    let function: Function<String, ()> =
        Function::from_fn(move |id: String| -> Result<(), Error> {
            let args: AuthenticateRequest = AuthenticateRequest::new(id);
            connection
                .lock()
                .map_err(|e| Error::RuntimeError(e.to_string()))?
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
