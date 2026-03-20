use crate::utilities::LogLevel;

#[derive(Clone, Debug)]
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
            max_size: None,
            max_files: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogConfig {
    pub file: Option<LogFileConfig>,
    pub level: LogLevel,
    pub local_list: LogLevel,
    pub message: LogLevel,
    pub notification: LogLevel,
    pub quick_fix_list: LogLevel,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            file: None,
            level: LogLevel::Info,
            local_list: LogLevel::Off,
            message: LogLevel::Off,
            notification: LogLevel::Off,
            quick_fix_list: LogLevel::Off,
        }
    }
}
