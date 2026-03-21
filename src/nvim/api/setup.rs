use nvim_oxi::{Function, Object, lua::Error};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::nvim::configuration::SetupArgs;
use crate::nvim::state::PluginState;
use crate::utilities::Logger;

/// Setup function for configuring the plugin
///
/// Accepts partial configuration where each field is optional.
/// Only provided values (Some()) update the config; missing values preserve existing defaults.
/// Can be called with no arguments or an empty table to keep all defaults.
#[instrument(level = "trace", skip_all)]
pub fn setup(plugin_state: Arc<Mutex<PluginState>>, logger: &'static Logger) -> Object {
    let function: Function<SetupArgs, Result<(), Error>> =
        Function::from_fn(move |args: SetupArgs| -> Result<(), Error> {
            debug!("Setup function called");

            let config_update = args.into_inner();
            let mut state = plugin_state.blocking_lock();
            let config = state.config.clone();
            config_update.apply_to(&mut state.config);
            drop(state);
            logger.configure(config.log.clone())?;

            Ok(())
        });
    function.into()
}
