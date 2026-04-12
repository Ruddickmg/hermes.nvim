use crate::utilities::logging::{LogFormat, LogLevel};
use nvim_oxi::{
    Object,
    conversion::{Error, FromObject},
};

use super::dict_from_object;

pub const LOG_FILE_NAME: &str = "hermes.log";

/// Configuration for a single log target (notification, message, quickfix, etc.)
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LogTargetConfig {
    pub level: LogLevel,
    pub format: LogFormat,
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
            config.format = val;
        }
    }
}

impl FromObject for LogTargetConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

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
    pub path: String,
    pub name: String,
    pub level: LogLevel,
    pub format: LogFormat, // None = use global format
    pub max_size: u64,
    pub max_files: u32,
}

impl LogFileConfig {
    pub fn is_enabled(&self) -> bool {
        self.level != LogLevel::Off && !self.path.is_empty()
    }
}

impl Default for LogFileConfig {
    fn default() -> Self {
        Self {
            path: "".to_string(),
            name: LOG_FILE_NAME.to_string(),
            level: LogLevel::Off,
            format: LogFormat::default(),
            max_size: 10_485_760, // 10MB default
            max_files: 5,         // Keep 5 backup files
        }
    }
}

/// Partial log file configuration where each field is optional
#[derive(Clone, Debug, Default)]
pub struct LogFileConfigPartial {
    pub path: Option<String>,
    pub level: Option<LogLevel>,
    pub format: Option<LogFormat>,
    pub max_size: Option<u64>,
    pub max_files: Option<u32>,
}

impl LogFileConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut LogFileConfig) {
        if let Some(val) = self.path {
            config.path = val;
        }
        if let Some(val) = self.level {
            config.level = val;
        }
        if let Some(val) = self.format {
            config.format = val;
        }
        if let Some(val) = self.max_size {
            config.max_size = val;
        }
        if let Some(val) = self.max_files {
            config.max_files = val;
        }
    }
}

impl FromObject for LogFileConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

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
    pub file: LogFileConfig,
    pub stdio: LogTargetConfig, // Stdout/stderr logging configuration
    pub notification: LogTargetConfig,
    pub message: LogTargetConfig,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            file: LogFileConfig::default(),
            stdio: LogTargetConfig::default(),
            message: LogTargetConfig::default(),
            notification: LogTargetConfig {
                level: LogLevel::Error,
                format: LogFormat::default(),
            },
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
}

impl LogConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut LogConfig) {
        if let Some(val) = self.file {
            val.apply_to(&mut config.file);
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
    }
}

impl FromObject for LogConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

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

        Ok(Self {
            file,
            stdio,
            notification,
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

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
        assert_eq!(config.format, LogFormat::Pretty);
    }

    #[test]
    fn test_log_target_config_partial_apply_partial_preserves_untouched() {
        let mut config = LogTargetConfig {
            level: LogLevel::Error,
            format: LogFormat::Compact,
        };
        let partial = LogTargetConfigPartial {
            level: Some(LogLevel::Warn),
            format: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.format, LogFormat::Compact);
    }

    #[test]
    fn test_log_target_config_partial_apply_partial_updates_provided() {
        let mut config = LogTargetConfig {
            level: LogLevel::Error,
            format: LogFormat::Compact,
        };
        let partial = LogTargetConfigPartial {
            level: Some(LogLevel::Warn),
            format: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.level, LogLevel::Warn);
    }

    #[test]
    fn test_log_file_config_partial_apply_to_path() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
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
            path: None,
            level: None,
            format: Some(LogFormat::Json),
            max_size: None,
            max_files: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.format, LogFormat::Json);
    }

    #[test]
    fn test_log_file_config_partial_apply_to_max_size() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
            path: None,
            level: None,
            format: None,
            max_size: Some(2048),
            max_files: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.max_size, 2048);
    }

    #[test]
    fn test_log_file_config_partial_apply_to_max_files() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
            path: None,
            level: None,
            format: None,
            max_size: None,
            max_files: Some(10),
        };
        partial.apply_to(&mut config);
        assert_eq!(config.max_files, 10);
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
            file: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.stdio.format, LogFormat::Full);
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
            file: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.notification.level, LogLevel::Error);
    }

    #[test]
    fn test_log_file_config_is_enabled_when_level_off() {
        let config = LogFileConfig {
            path: "/test.log".to_string(),
            name: LOG_FILE_NAME.to_string(),
            level: LogLevel::Off,
            format: LogFormat::default(),
            max_size: 10_485_760,
            max_files: 5,
        };
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_log_file_config_is_enabled_when_path_empty() {
        let config = LogFileConfig {
            path: "".to_string(),
            name: LOG_FILE_NAME.to_string(),
            level: LogLevel::Debug,
            format: LogFormat::default(),
            max_size: 10_485_760,
            max_files: 5,
        };
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_log_file_config_is_enabled_when_both_invalid() {
        let config = LogFileConfig {
            path: "".to_string(),
            name: LOG_FILE_NAME.to_string(),
            level: LogLevel::Off,
            format: LogFormat::default(),
            max_size: 10_485_760,
            max_files: 5,
        };
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_log_file_config_is_enabled_when_valid() {
        let config = LogFileConfig {
            path: "/test.log".to_string(),
            name: LOG_FILE_NAME.to_string(),
            level: LogLevel::Debug,
            format: LogFormat::default(),
            max_size: 10_485_760,
            max_files: 5,
        };
        assert!(config.is_enabled());
    }
}
