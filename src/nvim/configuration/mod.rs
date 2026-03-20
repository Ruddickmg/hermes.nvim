mod buffer;
mod log;
mod permissions;
mod terminal;

pub use buffer::{BufferConfig, BufferConfigPartial};
pub use log::{LogConfig, LogConfigPartial, LogFileConfig, LogFileConfigPartial};
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
    pub buffer: BufferConfig,
    pub log: LogConfig,
}

/// Partial client configuration for setup function
#[derive(Clone, Debug, Default)]
pub struct ClientConfigPartial {
    pub permissions: Option<PermissionsPartial>,
    pub terminal: Option<TerminalConfigPartial>,
    pub buffer: Option<BufferConfigPartial>,
    pub log: Option<LogConfigPartial>,
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
        if let Some(buffer) = self.buffer {
            buffer.apply_to(&mut config.buffer);
        }
        if let Some(log) = self.log {
            log.apply_to(&mut config.log);
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

        let buffer = dict
            .get("buffer")
            .map(|o| BufferConfigPartial::from_object(o.clone()))
            .transpose()?;

        let log = dict
            .get("log")
            .map(|o| LogConfigPartial::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            permissions,
            terminal,
            buffer,
            log,
        })
    }
}

impl nvim_oxi::lua::Pushable for ClientConfigPartial {
    unsafe fn push(self, lua_state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
        let mut dict = Dictionary::new();

        if let Some(permissions) = self.permissions {
            let mut perms_dict = Dictionary::new();
            if let Some(val) = permissions.fs_write_access {
                perms_dict.insert("fs_write_access", val);
            }
            if let Some(val) = permissions.fs_read_access {
                perms_dict.insert("fs_read_access", val);
            }
            if let Some(val) = permissions.terminal_access {
                perms_dict.insert("terminal_access", val);
            }
            if let Some(val) = permissions.can_request_permissions {
                perms_dict.insert("can_request_permissions", val);
            }
            if let Some(val) = permissions.allow_notifications {
                perms_dict.insert("allow_notifications", val);
            }
            dict.insert("permissions", perms_dict);
        }

        if let Some(terminal) = self.terminal {
            let mut term_dict = Dictionary::new();
            if let Some(val) = terminal.delete {
                term_dict.insert("delete", val);
            }
            if let Some(val) = terminal.hidden {
                term_dict.insert("hidden", val);
            }
            if let Some(val) = terminal.enabled {
                term_dict.insert("enabled", val);
            }
            if let Some(val) = terminal.buffered {
                term_dict.insert("buffered", val);
            }
            dict.insert("terminal", term_dict);
        }

        if let Some(buffer) = self.buffer {
            let mut buffer_dict = Dictionary::new();
            if let Some(val) = buffer.auto_save {
                buffer_dict.insert("auto_save", val);
            }
            dict.insert("buffer", buffer_dict);
        }

        if let Some(log) = self.log {
            let mut log_dict = Dictionary::new();
            if let Some(file) = log.file {
                let mut file_dict = Dictionary::new();
                if let Some(val) = file.enabled {
                    file_dict.insert("enabled", val);
                }
                if let Some(ref val) = file.path {
                    file_dict.insert("path", val.as_str());
                }
                if let Some(val) = file.level {
                    file_dict.insert("level", val.to_string());
                }
                if let Some(val) = file.max_size {
                    file_dict.insert("max_size", val as i64);
                }
                if let Some(val) = file.max_files {
                    file_dict.insert("max_files", val as i64);
                }
                log_dict.insert("file", file_dict);
            }
            if let Some(val) = log.level {
                log_dict.insert("level", val.to_string());
            }
            if let Some(val) = log.local_list {
                log_dict.insert("local_list", val.to_string());
            }
            if let Some(val) = log.message {
                log_dict.insert("message", val.to_string());
            }
            if let Some(val) = log.notification {
                log_dict.insert("notification", val.to_string());
            }
            if let Some(val) = log.quick_fix_list {
                log_dict.insert("quick_fix_list", val.to_string());
            }
            dict.insert("log", log_dict);
        }

        dict.push(lua_state)
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

impl nvim_oxi::lua::Pushable for SetupArgs {
    unsafe fn push(self, lua_state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
        if let Some(config) = self.0 {
            config.push(lua_state)
        } else {
            // Push nil for None
            Ok(0) // Pushing nil typically returns 0 values pushed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_partial_apply_to_updates_nested() {
        let mut config = ClientConfig::default();
        let partial = ClientConfigPartial {
            permissions: Some(PermissionsPartial {
                fs_write_access: Some(false),
                ..Default::default()
            }),
            terminal: Some(TerminalConfigPartial {
                hidden: Some(false),
                ..Default::default()
            }),
            buffer: Some(BufferConfigPartial {
                auto_save: Some(true),
            }),
            log: Some(LogConfigPartial {
                level: Some(crate::utilities::LogLevel::Debug),
                ..Default::default()
            }),
        };
        partial.apply_to(&mut config);

        // Verify changes
        assert!(!config.permissions.fs_write_access);
        assert!(!config.terminal.hidden);
        assert!(config.buffer.auto_save);
        assert_eq!(config.log.level, crate::utilities::LogLevel::Debug);

        // Verify unspecified fields preserved defaults
        assert!(config.permissions.fs_read_access); // default true
        assert!(!config.terminal.delete); // default false, unchanged
    }

    #[test]
    fn test_client_config_partial_apply_to_preserves_all_when_none() {
        let mut config = ClientConfig {
            permissions: Permissions {
                fs_write_access: false,
                fs_read_access: false,
                terminal_access: false,
                can_request_permissions: false,
                allow_notifications: false,
            },
            terminal: TerminalConfig {
                delete: true,
                hidden: false,
                enabled: false,
                buffered: false,
            },
            buffer: BufferConfig { auto_save: true },
            log: LogConfig {
                file: None,
                level: crate::utilities::LogLevel::Warn,
                local_list: crate::utilities::LogLevel::Warn,
                message: crate::utilities::LogLevel::Warn,
                notification: crate::utilities::LogLevel::Warn,
                quick_fix_list: crate::utilities::LogLevel::Warn,
            },
        };
        let partial = ClientConfigPartial::default(); // all None
        partial.apply_to(&mut config);

        // All should remain unchanged
        assert!(!config.permissions.fs_write_access);
        assert!(config.terminal.delete);
        assert!(config.buffer.auto_save);
        assert_eq!(config.log.level, crate::utilities::LogLevel::Warn);
    }

    #[test]
    fn test_client_config_partial_from_object_parses_empty_dict() {
        let dict = nvim_oxi::Dictionary::default();
        let obj = nvim_oxi::Object::from(dict);
        let partial = ClientConfigPartial::from_object(obj).expect("Should parse");

        assert!(partial.permissions.is_none());
        assert!(partial.terminal.is_none());
        assert!(partial.buffer.is_none());
        assert!(partial.log.is_none());
    }

    #[test]
    fn test_client_config_partial_from_object_parses_full_config() {
        let mut perms_dict = nvim_oxi::Dictionary::new();
        perms_dict.insert("fs_write_access", false);

        let mut term_dict = nvim_oxi::Dictionary::new();
        term_dict.insert("hidden", false);

        let mut buffer_dict = nvim_oxi::Dictionary::new();
        buffer_dict.insert("auto_save", true);

        let mut log_dict = nvim_oxi::Dictionary::new();
        log_dict.insert("level", "debug");

        let mut config_dict = nvim_oxi::Dictionary::new();
        config_dict.insert("permissions", perms_dict);
        config_dict.insert("terminal", term_dict);
        config_dict.insert("buffer", buffer_dict);
        config_dict.insert("log", log_dict);

        let obj = nvim_oxi::Object::from(config_dict);
        let partial = ClientConfigPartial::from_object(obj).expect("Should parse");

        assert!(partial.permissions.is_some());
        assert!(partial.terminal.is_some());
        assert!(partial.buffer.is_some());
        assert!(partial.log.is_some());
    }
}
