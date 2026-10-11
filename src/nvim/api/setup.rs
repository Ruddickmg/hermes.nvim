use nvim_oxi::Object;
use nvim_oxi::conversion::FromObject;
use nvim_oxi::lua::{self, Poppable};
use tracing::{error, instrument, warn};

use crate::nvim::configuration::{ClientConfigPartial, elicitation_changed};
use crate::{acp::Result, api::Api};

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
            Ok(ClientConfigPartial::from_object(obj)
                .map(|c| Self(Some(c)))
                .inspect_err(|e| {
                    error!(
                        "Error occurred while parsing setup args, reverting to defaults: {:?}",
                        e
                    )
                })
                .unwrap_or_default())
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
    pub async fn setup(&self, args: SetupArgs) -> Result<()> {
        let config_update = args.into_inner();
        let mut state = self.state.lock().await;
        let show_cmdline_progress = state.config.progress.cmdline;
        let old_permissions = state.config.permissions.clone();
        config_update.clone().apply_to(&mut state.config);
        let cmdline = state.config.progress.cmdline;
        let log_config = state.config.log.clone();
        drop(state);
        let elicitation_changed =
            elicitation_changed(config_update.permissions.as_ref(), &old_permissions);
        if !self.connection.connected_agents().is_empty() && elicitation_changed {
            warn!("Enabling or disabling elicitation will only take effect on new connections");
        }
        if let Some(progress) = config_update.progress.clone() {
            if let Some(show_in_cmdline) = progress.cmdline
                && show_in_cmdline != show_cmdline_progress
            {
                crate::nvim::configuration::show_progress_in_cmdline(cmdline);
            }
        }

        self.logger.configure(log_config)
    }
}
