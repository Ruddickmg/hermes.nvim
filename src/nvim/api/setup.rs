use nvim_oxi::conversion::FromObject;
use nvim_oxi::lua::{self, Poppable};
use nvim_oxi::{Function, Object, lua::Error};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::nvim::configuration::ClientConfigPartial;
use crate::nvim::state::PluginState;
use crate::utilities::Logger;

/// Wrapper type for setup arguments that can be nil or a config table
#[derive(Clone, Debug, Default)]
pub struct SetupArgs(pub Option<ClientConfigPartial>);

impl SetupArgs {
    pub fn into_inner(self) -> ClientConfigPartial {
        self.0.unwrap_or_default()
    }
}

impl Poppable for SetupArgs {
    unsafe fn pop(lua_state: *mut lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        // If object is nil, return None
        if obj.is_nil() {
            Ok(Self(None))
        } else {
            // Otherwise, try to parse as ClientConfigPartial
            ClientConfigPartial::from_object(obj)
                .map(|c| Self(Some(c)))
                .map_err(|e| lua::Error::RuntimeError(e.to_string()))
        }
    }
}

impl nvim_oxi::lua::Pushable for SetupArgs {
    unsafe fn push(self, lua_state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
        if let Some(config) = self.0 {
            unsafe { config.push(lua_state) }
        } else {
            // Push nil for None
            Ok(0) // Pushing nil typically returns 0 values pushed
        }
    }
}

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
