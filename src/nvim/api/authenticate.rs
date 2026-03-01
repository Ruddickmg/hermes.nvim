use std::{rc::Rc, sync::Mutex};

use agent_client_protocol::AuthenticateRequest;
use nvim_oxi::{Function, lua::Error};

use crate::PluginState;

pub fn create_lua_authenticate(
    plugin_state: Rc<Mutex<PluginState>>,
) -> Function<String, Result<(), Error>> {
    Function::from_fn(move |id: String| {
        let args: AuthenticateRequest = AuthenticateRequest::new(id);
        plugin_state
            .lock()
            .map_err(|e| Error::RuntimeError(e.to_string()))?
            .connection
            .get_current_connection()
            .ok_or_else(|| {
                Error::RuntimeError(
                    "You are not connected to an agent, call connect before \"authenticate\""
                        .to_string(),
                )
            })?
            .authenticate(args)?;
        Ok(())
    })
}
