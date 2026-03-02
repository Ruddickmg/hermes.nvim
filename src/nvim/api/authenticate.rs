use agent_client_protocol::{AuthenticateRequest, Client};
use nvim_oxi::{Function, Object, lua::Error};
use std::{rc::Rc, sync::Mutex};

use crate::{apc::connection::ConnectionManager, nvim::autocommands::AutoCommands};

pub fn create_lua_authenticate<H: Client>(
    connection: Rc<Mutex<ConnectionManager<AutoCommands>>>,
) -> Object {
    let function: Function<String, Result<(), Error>> =
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
