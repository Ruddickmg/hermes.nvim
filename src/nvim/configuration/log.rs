use crate::utilities::{LogFormat, LogLevel};
use nvim_oxi::{
    conversion::{Error, FromObject},
    Dictionary, Object,
};

/// Configuration for a single log target (notification, message, quickfix, etc.)
#[derive(Clone, Debug, PartialEq)]
pub struct LogTargetConfig {
    pub level: LogLevel,
    pub format: Option<LogFormat>, // None = use global format
}

impl Default for LogTargetConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Off,
            format: None,
        }
    }
}

/// Partial configuration for a log target
#[derive(Clone, Debug, Default)]
pub struct LogTargetConfigPartial {
    pub level: Option<LogLevel>,
    pub format: Option<LogFormat>,
}

impl LogTargetConfigPartial {
    pub fn apply_to(self, config: &mut LogTargetConfig) {
        if let Some(val) = self.level {
            config.level = val;
        }
        if let Some(val) = self.format {
            config.format = Some(val);
        }
    }
}

impl FromObject for LogTargetConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = Dictionary::from_object(obj)?;

        let level = dict
            .get("level")
            .map(|o| LogLevel::from_object(o.clone()))
            .transpose()?;
        let format = dict
            .get("format")
            .map(|o| LogFormat::from_object(o.clone()))
            .transpose()?;

        Ok(Self { level, format })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogFileConfig {
    pub enabled: bool,
    pub path: String,
    pub level: LogLevel,
    pub format: Option<LogFormat>, // None = use global format
    pub max_size: Option<u64>,
    pub max_files: Option<u32>,
}

impl Default for LogFileConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: "".to_string(),
            level: LogLevel::Warn,
            format: None,
            max_size: Some(10_485_760), // 10MB default
            max_files: Some(5),         // Keep 5 backup files
        }
    }
}

/// Partial log file configuration where each field is optional
#[derive(Clone, Debug, Default)]
pub struct LogFileConfigPartial {
    pub enabled: Option<bool>,
    pub path: Option<String>,
    pub level: Option<LogLevel>,
    pub format: Option<LogFormat>,
    pub max_size: Option<u64>,
    pub max_files: Option<u32>,
}

impl LogFileConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut LogFileConfig) {
        if let Some(val) = self.enabled {
            config.enabled = val;
        }
        if let Some(val) = self.path {
            config.path = val;
        }
        if let Some(val) = self.level {
            config.level = val;
        }
        if let Some(val) = self.format {
            config.format = Some(val);
        }
        if let Some(val) = self.max_size {
            config.max_size = Some(val);
        }
        if let Some(val) = self.max_files {
            config.max_files = Some(val);
        }
    }
}

impl FromObject for LogFileConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = Dictionary::from_object(obj)?;

        let enabled = dict
            .get("enabled")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let path = dict
            .get("path")
            .map(|o| String::from_object(o.clone()))
            .transpose()?;
        let level = dict
            .get("level")
            .map(|o| LogLevel::from_object(o.clone()))
            .transpose()?;
        let format = dict
            .get("format")
            .map(|o| LogFormat::from_object(o.clone()))
            .transpose()?;
        let max_size = dict
            .get("max_size")
            .map(|o| u64::from_object(o.clone()))
            .transpose()?;
        let max_files = dict
            .get("max_files")
            .map(|o| u32::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            enabled,
            path,
            level,
            format,
            max_size,
            max_files,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogConfig {
    pub file: Option<LogFileConfig>,
    pub stdio: LogTargetConfig, // Stdout/stderr logging configuration
    pub notification: LogTargetConfig,
    pub message: LogTargetConfig,
    pub quickfix: LogTargetConfig,
    pub local_list: LogTargetConfig,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            file: None,
            stdio: LogTargetConfig {
                level: LogLevel::Info,
                format: None,
            },
            notification: LogTargetConfig {
                level: LogLevel::Error,
                format: None,
            },
            message: LogTargetConfig::default(),
            quickfix: LogTargetConfig::default(),
            local_list: LogTargetConfig::default(),
        }
    }
}

/// Partial log configuration where each field is optional
#[derive(Clone, Debug, Default)]
pub struct LogConfigPartial {
    pub file: Option<LogFileConfigPartial>,
    pub stdio: Option<LogTargetConfigPartial>,
    pub notification: Option<LogTargetConfigPartial>,
    pub message: Option<LogTargetConfigPartial>,
    pub quickfix: Option<LogTargetConfigPartial>,
    pub local_list: Option<LogTargetConfigPartial>,
}

impl LogConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut LogConfig) {
        if let Some(file_partial) = self.file {
            if let Some(ref mut file_config) = config.file {
                file_partial.apply_to(file_config);
            } else {
                config.file = Some(LogFileConfig {
                    enabled: file_partial.enabled.unwrap_or(false),
                    path: file_partial.path.unwrap_or_default(),
                    level: file_partial.level.unwrap_or(LogLevel::Warn),
                    format: file_partial.format,
                    max_size: file_partial.max_size,
                    max_files: file_partial.max_files,
                });
            }
        }
        if let Some(val) = self.stdio {
            val.apply_to(&mut config.stdio);
        }
        if let Some(val) = self.notification {
            val.apply_to(&mut config.notification);
        }
        if let Some(val) = self.message {
            val.apply_to(&mut config.message);
        }
        if let Some(val) = self.quickfix {
            val.apply_to(&mut config.quickfix);
        }
        if let Some(val) = self.local_list {
            val.apply_to(&mut config.local_list);
        }
    }
}

impl FromObject for LogConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = Dictionary::from_object(obj)?;

        let file = dict
            .get("file")
            .map(|o| LogFileConfigPartial::from_object(o.clone()))
            .transpose()?;
        let stdio = dict
            .get("stdio")
            .map(|o| LogTargetConfigPartial::from_object(o.clone()))
            .transpose()?;
        let notification = dict
            .get("notification")
            .map(|o| LogTargetConfigPartial::from_object(o.clone()))
            .transpose()?;
        let message = dict
            .get("message")
            .map(|o| LogTargetConfigPartial::from_object(o.clone()))
            .transpose()?;
        let quickfix = dict
            .get("quickfix")
            .map(|o| LogTargetConfigPartial::from_object(o.clone()))
            .transpose()?;
        let local_list = dict
            .get("local_list")
            .map(|o| LogTargetConfigPartial::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            file,
            stdio,
            notification,
            message,
            quickfix,
            local_list,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_log_target_config_default_level_is_off() {
        let config = LogTargetConfig::default();
        assert_eq!(config.level, LogLevel::Off);
    }

    #[test]
    fn test_log_target_config_default_format_is_none() {
        let config = LogTargetConfig::default();
        assert_eq!(config.format, None);
    }

    #[test]
    fn test_log_target_config_custom_level() {
        let config = LogTargetConfig {
            level: LogLevel::Info,
            format: None,
        };
        assert_eq!(config.level, LogLevel::Info);
    }

    #[test]
    fn test_log_target_config_custom_format() {
        let config = LogTargetConfig {
            level: LogLevel::Info,
            format: Some(LogFormat::Json),
        };
        assert_eq!(config.format, Some(LogFormat::Json));
    }

    #[test]
    fn test_log_target_config_partial_apply_to_level() {
        let mut config = LogTargetConfig::default();
        let partial = LogTargetConfigPartial {
            level: Some(LogLevel::Debug),
            format: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.level, LogLevel::Debug);
    }

    #[test]
    fn test_log_target_config_partial_apply_to_format() {
        let mut config = LogTargetConfig::default();
        let partial = LogTargetConfigPartial {
            level: None,
            format: Some(LogFormat::Pretty),
        };
        partial.apply_to(&mut config);
        assert_eq!(config.format, Some(LogFormat::Pretty));
    }

    #[test]
    fn test_log_target_config_partial_apply_partial_preserves_untouched() {
        let mut config = LogTargetConfig {
            level: LogLevel::Error,
            format: Some(LogFormat::Compact),
        };
        let partial = LogTargetConfigPartial {
            level: Some(LogLevel::Warn),
            format: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.format, Some(LogFormat::Compact));
    }

    #[test]
    fn test_log_target_config_partial_apply_partial_updates_provided() {
        let mut config = LogTargetConfig {
            level: LogLevel::Error,
            format: Some(LogFormat::Compact),
        };
        let partial = LogTargetConfigPartial {
            level: Some(LogLevel::Warn),
            format: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.level, LogLevel::Warn);
    }

    #[test]
    fn test_log_config_default_stdio_level() {
        let config = LogConfig::default();
        assert_eq!(config.stdio.level, LogLevel::Info);
    }

    #[test]
    fn test_log_config_default_notification_level() {
        let config = LogConfig::default();
        assert_eq!(config.notification.level, LogLevel::Error);
    }

    #[test]
    fn test_log_config_default_message_level() {
        let config = LogConfig::default();
        assert_eq!(config.message.level, LogLevel::Off);
    }

    #[test]
    fn test_log_config_default_quickfix_level() {
        let config = LogConfig::default();
        assert_eq!(config.quickfix.level, LogLevel::Off);
    }

    #[test]
    fn test_log_config_default_local_list_level() {
        let config = LogConfig::default();
        assert_eq!(config.local_list.level, LogLevel::Off);
    }

    #[test]
    fn test_log_config_default_file_is_none() {
        let config = LogConfig::default();
        assert_eq!(config.file, None);
    }

    #[test]
    fn test_log_file_config_default_enabled() {
        let config = LogFileConfig::default();
        assert!(!config.enabled);
    }

    #[test]
    fn test_log_file_config_default_level() {
        let config = LogFileConfig::default();
        assert_eq!(config.level, LogLevel::Warn);
    }

    #[test]
    fn test_log_file_config_default_format() {
        let config = LogFileConfig::default();
        assert_eq!(config.format, None);
    }

    #[test]
    fn test_log_file_config_default_max_size() {
        let config = LogFileConfig::default();
        assert_eq!(config.max_size, Some(10_485_760));
    }

    #[test]
    fn test_log_file_config_default_max_files() {
        let config = LogFileConfig::default();
        assert_eq!(config.max_files, Some(5));
    }

    #[test]
    fn test_log_file_config_partial_apply_to_enabled() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
            enabled: Some(true),
            path: None,
            level: None,
            format: None,
            max_size: None,
            max_files: None,
        };
        partial.apply_to(&mut config);
        assert!(config.enabled);
    }

    #[test]
    fn test_log_file_config_partial_apply_to_path() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
            enabled: None,
            path: Some("/test/path".to_string()),
            level: None,
            format: None,
            max_size: None,
            max_files: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.path, "/test/path");
    }

    #[test]
    fn test_log_file_config_partial_apply_to_level() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
            enabled: None,
            path: None,
            level: Some(LogLevel::Debug),
            format: None,
            max_size: None,
            max_files: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.level, LogLevel::Debug);
    }

    #[test]
    fn test_log_file_config_partial_apply_to_format() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
            enabled: None,
            path: None,
            level: None,
            format: Some(LogFormat::Json),
            max_size: None,
            max_files: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.format, Some(LogFormat::Json));
    }

    #[test]
    fn test_log_file_config_partial_apply_to_max_size() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
            enabled: None,
            path: None,
            level: None,
            format: None,
            max_size: Some(2048),
            max_files: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.max_size, Some(2048));
    }

    #[test]
    fn test_log_file_config_partial_apply_to_max_files() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
            enabled: None,
            path: None,
            level: None,
            format: None,
            max_size: None,
            max_files: Some(10),
        };
        partial.apply_to(&mut config);
        assert_eq!(config.max_files, Some(10));
    }

    #[test]
    fn test_log_config_partial_apply_to_stdio_level() {
        let mut config = LogConfig::default();
        let partial = LogConfigPartial {
            stdio: Some(LogTargetConfigPartial {
                level: Some(LogLevel::Trace),
                format: None,
            }),
            notification: None,
            message: None,
            quickfix: None,
            local_list: None,
            file: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.stdio.level, LogLevel::Trace);
    }

    #[test]
    fn test_log_config_partial_apply_to_stdio_format() {
        let mut config = LogConfig::default();
        let partial = LogConfigPartial {
            stdio: Some(LogTargetConfigPartial {
                level: None,
                format: Some(LogFormat::Full),
            }),
            notification: None,
            message: None,
            quickfix: None,
            local_list: None,
            file: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.stdio.format, Some(LogFormat::Full));
    }

    #[test]
    fn test_log_config_partial_apply_to_preserves_untouched() {
        let mut config = LogConfig::default();
        let partial = LogConfigPartial {
            stdio: Some(LogTargetConfigPartial {
                level: Some(LogLevel::Trace),
                format: Some(LogFormat::Full),
            }),
            notification: None,
            message: None,
            quickfix: None,
            local_list: None,
            file: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.notification.level, LogLevel::Error);
    }
}
