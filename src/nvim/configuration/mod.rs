mod permissions;
mod terminal;

use nvim_oxi::{
    conversion::{Error, FromObject},
    lua::{self, Poppable},
    Dictionary, Object,
};
pub use permissions::{Permissions, PermissionsPartial};
pub use terminal::{TerminalConfig, TerminalConfigPartial};

#[derive(Clone, Debug, Default)]
pub struct ClientConfig {
    pub permissions: Permissions,
    pub terminal: TerminalConfig,
}

/// Partial client configuration for setup function
#[derive(Clone, Debug, Default)]
pub struct ClientConfigPartial {
    pub permissions: Option<PermissionsPartial>,
    pub terminal: Option<TerminalConfigPartial>,
}

impl ClientConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut ClientConfig) {
        if let Some(permissions) = self.permissions {
            permissions.apply_to(&mut config.permissions);
        }
        if let Some(terminal) = self.terminal {
            terminal.apply_to(&mut config.terminal);
        }
    }
}

impl FromObject for ClientConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = Dictionary::from_object(obj)?;

        let permissions = dict
            .get("permissions")
            .map(|o| PermissionsPartial::from_object(o.clone()))
            .transpose()?;

        let terminal = dict
            .get("terminal")
            .map(|o| TerminalConfigPartial::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            permissions,
            terminal,
        })
    }
}

impl Poppable for ClientConfigPartial {
    unsafe fn pop(lua_state: *mut lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Self::from_object(obj).map_err(|e| lua::Error::RuntimeError(e.to_string()))
    }
}

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
