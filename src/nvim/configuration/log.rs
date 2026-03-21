use crate::utilities::{LogFormat, LogLevel};
use nvim_oxi::{
    conversion::{Error, FromObject},
    Dictionary, Object,
};

#[derive(Clone, Debug, PartialEq)]
pub struct LogFileConfig {
    pub enabled: bool,
    pub path: String,
    pub level: LogLevel,
    pub max_size: Option<u64>,
    pub max_files: Option<u32>,
}

impl Default for LogFileConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            // TODO: figure out default path
            path: "".to_string(),
            level: LogLevel::Warn,
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
            max_size,
            max_files,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogConfig {
    pub file: Option<LogFileConfig>,
    pub level: LogLevel,
    pub format: LogFormat, // Global format (fallback for all targets)
    pub local_list: LogLevel,
    pub message: LogLevel,
    pub notification: LogLevel,
    pub quick_fix_list: LogLevel,
    // Per-target format overrides (None = use global format)
    pub notification_format: Option<LogFormat>,
    pub message_format: Option<LogFormat>,
    pub quickfix_format: Option<LogFormat>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            file: None,
            level: LogLevel::Off,
            format: LogFormat::default(), // Default global format
            local_list: LogLevel::Off,
            message: LogLevel::Off,
            notification: LogLevel::Error,
            quick_fix_list: LogLevel::Off,
            notification_format: None, // None = use global format
            message_format: None,
            quickfix_format: None,
        }
    }
}

/// Partial log configuration where each field is optional
#[derive(Clone, Debug, Default)]
pub struct LogConfigPartial {
    pub file: Option<LogFileConfigPartial>,
    pub level: Option<LogLevel>,
    pub format: Option<LogFormat>,
    pub local_list: Option<LogLevel>,
    pub message: Option<LogLevel>,
    pub notification: Option<LogLevel>,
    pub quick_fix_list: Option<LogLevel>,
    // Per-target format overrides
    pub notification_format: Option<LogFormat>,
    pub message_format: Option<LogFormat>,
    pub quickfix_format: Option<LogFormat>,
}

impl LogConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut LogConfig) {
        if let Some(file_partial) = self.file {
            if let Some(ref mut file_config) = config.file {
                file_partial.apply_to(file_config);
            } else {
                // Create new LogFileConfig from partial with defaults for unspecified fields
                config.file = Some(LogFileConfig {
                    enabled: file_partial.enabled.unwrap_or(false),
                    path: file_partial.path.unwrap_or_default(),
                    level: file_partial.level.unwrap_or(LogLevel::Warn),
                    max_size: file_partial.max_size,
                    max_files: file_partial.max_files,
                });
            }
        }
        if let Some(val) = self.level {
            config.level = val;
        }
        if let Some(val) = self.format {
            config.format = val;
        }
        if let Some(val) = self.local_list {
            config.local_list = val;
        }
        if let Some(val) = self.message {
            config.message = val;
        }
        if let Some(val) = self.notification {
            config.notification = val;
        }
        if let Some(val) = self.quick_fix_list {
            config.quick_fix_list = val;
        }
        // Apply per-target format overrides
        if let Some(val) = self.notification_format {
            config.notification_format = Some(val);
        }
        if let Some(val) = self.message_format {
            config.message_format = Some(val);
        }
        if let Some(val) = self.quickfix_format {
            config.quickfix_format = Some(val);
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
        let level = dict
            .get("level")
            .map(|o| LogLevel::from_object(o.clone()))
            .transpose()?;
        let format = dict
            .get("format")
            .map(|o| LogFormat::from_object(o.clone()))
            .transpose()?;
        let local_list = dict
            .get("local_list")
            .map(|o| LogLevel::from_object(o.clone()))
            .transpose()?;
        let message = dict
            .get("message")
            .map(|o| LogLevel::from_object(o.clone()))
            .transpose()?;
        let notification = dict
            .get("notification")
            .map(|o| LogLevel::from_object(o.clone()))
            .transpose()?;
        let quick_fix_list = dict
            .get("quick_fix_list")
            .map(|o| LogLevel::from_object(o.clone()))
            .transpose()?;
        // Parse per-target format overrides
        let notification_format = dict
            .get("notification_format")
            .map(|o| LogFormat::from_object(o.clone()))
            .transpose()?;
        let message_format = dict
            .get("message_format")
            .map(|o| LogFormat::from_object(o.clone()))
            .transpose()?;
        let quickfix_format = dict
            .get("quickfix_format")
            .map(|o| LogFormat::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            file,
            level,
            format,
            local_list,
            message,
            notification,
            quick_fix_list,
            notification_format,
            message_format,
            quickfix_format,
        })
    }
}
