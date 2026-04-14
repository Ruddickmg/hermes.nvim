use nvim_oxi::Object;
use nvim_oxi::conversion::FromObject;
use nvim_oxi::lua::{self, Poppable};
use tracing::instrument;

use crate::{acp::Result, api::Api};
use crate::nvim::configuration::ClientConfigPartial;

/// Wrapper type for setup arguments that can be nil or a config table
#[derive(Clone, Debug, Default)]
pub struct SetupArgs(pub Option<ClientConfigPartial>);

impl SetupArgs {
    pub fn into_inner(self) -> ClientConfigPartial {
        self.0.unwrap_or_default()
    }
}

impl Poppable for SetupArgs {
    unsafe fn pop(lua_state: *mut lua::ffi::State) -> std::result::Result<Self, lua::Error> {
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
    unsafe fn push(self, lua_state: *mut lua::ffi::State) -> std::result::Result<i32, lua::Error> {
        if let Some(config) = self.0 {
            unsafe { config.push(lua_state) }
        } else {
            // Push nil for None
            Ok(0) // Pushing nil typically returns 0 values pushed
        }
    }
}

impl Api {
    #[instrument(level = "trace", skip_all)]
    pub fn setup(&self, args: SetupArgs) -> Result<()> {
        let config_update = args.into_inner();
        let mut state = self.state.blocking_lock();
        config_update.apply_to(&mut state.config);
        let log_config = state.config.log.clone();
        drop(state);
        self.logger.configure(log_config)
    }
}
