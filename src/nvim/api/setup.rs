use nvim_oxi::{lua::Error, Function, Object};
use std::{cell::RefCell, rc::Rc};
use tracing::{debug, instrument};

use crate::acp::connection::ConnectionManager;
use crate::nvim::configuration::SetupArgs;

/// Setup function for configuring the plugin
///
/// Accepts partial configuration where each field is optional.
/// Only provided values (Some()) update the config; missing values preserve existing defaults.
/// Can be called with no arguments or an empty table to keep all defaults.
#[instrument(level = "trace", skip_all)]
pub fn setup(connection: Rc<RefCell<ConnectionManager>>) -> Object {
    let function: Function<SetupArgs, Result<(), Error>> =
        Function::from_fn(move |args: SetupArgs| -> Result<(), Error> {
            debug!("Setup function called");

            let config = args.into_inner();

            connection
                .try_borrow_mut()
                .map_err(|e| {
                    Error::RuntimeError(format!("Failed to borrow connection manager: {}", e))
                })?
                .update_config(config);

            Ok(())
        });
    function.into()
}
