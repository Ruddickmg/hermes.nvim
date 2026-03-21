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
    fn test_log_target_config_default() {
        let config = LogTargetConfig::default();
        assert_eq!(config.level, LogLevel::Off);
        assert_eq!(config.format, None);
    }

    #[test]
    fn test_log_target_config_custom() {
        let config = LogTargetConfig {
            level: LogLevel::Info,
            format: Some(LogFormat::Json),
        };
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.format, Some(LogFormat::Json));
    }

    #[test]
    fn test_log_target_config_partial_apply_to() {
        let mut config = LogTargetConfig::default();
        let partial = LogTargetConfigPartial {
            level: Some(LogLevel::Debug),
            format: Some(LogFormat::Pretty),
        };
        partial.apply_to(&mut config);
        assert_eq!(config.level, LogLevel::Debug);
        assert_eq!(config.format, Some(LogFormat::Pretty));
    }

    #[test]
    fn test_log_target_config_partial_apply_partial() {
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
        assert_eq!(config.format, Some(LogFormat::Compact)); // unchanged
    }

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.file, None);
        assert_eq!(config.stdio.level, LogLevel::Info);
        assert_eq!(config.notification.level, LogLevel::Error);
        assert_eq!(config.message.level, LogLevel::Off);
        assert_eq!(config.quickfix.level, LogLevel::Off);
        assert_eq!(config.local_list.level, LogLevel::Off);
    }

    #[test]
    fn test_log_file_config_default() {
        let config = LogFileConfig::default();
        assert_eq!(config.enabled, false);
        assert_eq!(config.level, LogLevel::Warn);
        assert_eq!(config.format, None);
        assert_eq!(config.max_size, Some(10_485_760));
        assert_eq!(config.max_files, Some(5));
    }

    #[test]
    fn test_log_file_config_partial_apply_to() {
        let mut config = LogFileConfig::default();
        let partial = LogFileConfigPartial {
            enabled: Some(true),
            path: Some("/test/path".to_string()),
            level: Some(LogLevel::Debug),
            format: Some(LogFormat::Json),
            max_size: Some(2048),
            max_files: Some(10),
        };
        partial.apply_to(&mut config);
        assert!(config.enabled);
        assert_eq!(config.path, "/test/path");
        assert_eq!(config.level, LogLevel::Debug);
        assert_eq!(config.format, Some(LogFormat::Json));
        assert_eq!(config.max_size, Some(2048));
        assert_eq!(config.max_files, Some(10));
    }

    #[test]
    fn test_log_config_partial_apply_to() {
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
        assert_eq!(config.stdio.level, LogLevel::Trace);
        assert_eq!(config.stdio.format, Some(LogFormat::Full));
        // other fields should remain default
        assert_eq!(config.notification.level, LogLevel::Error);
    }
}
