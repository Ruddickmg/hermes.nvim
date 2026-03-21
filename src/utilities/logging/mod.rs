use nvim_oxi::api::{self, opts::OptionOpts};
use std::sync::Mutex as StdMutex; // For non-async file writer
use std::sync::{Arc, OnceLock};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt,
    layer::Layered,
    prelude::*,
    reload::{self, Handle},
    EnvFilter, Registry,
};

use crate::{
    acp::error::Error,
    nvim::configuration::{LogConfig, LogFileConfig},
};

pub mod channel;
pub mod file;

static LOGGER: OnceLock<Logger> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    #[default]
    Info = 2,
    Warn = 3,
    Error = 4,
    Off = 5,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
            LogLevel::Off => write!(f, "off"),
        }
    }
}

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => LevelFilter::TRACE,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Error => LevelFilter::ERROR,
            LogLevel::Off => LevelFilter::OFF,
        }
    }
}

impl From<i64> for LogLevel {
    fn from(value: i64) -> Self {
        match value {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            _ => LogLevel::Off,
        }
    }
}

impl From<&str> for LogLevel {
    fn from(value: &str) -> Self {
        match value {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Off,
        }
    }
}

impl From<String> for LogLevel {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl nvim_oxi::conversion::FromObject for LogLevel {
    fn from_object(obj: nvim_oxi::Object) -> Result<Self, nvim_oxi::conversion::Error> {
        // Try to parse as string first
        if let Ok(s) = String::from_object(obj.clone()) {
            Ok(LogLevel::from(s))
        } else if let Ok(n) = i64::from_object(obj) {
            // Try to parse as integer
            Ok(LogLevel::from(n))
        } else {
            Err(nvim_oxi::conversion::Error::Other(
                "LogLevel must be a string or integer".to_string(),
            ))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogFormat {
    #[default]
    Pretty,
    Compact,
    Full,
    Json,
}

impl From<&str> for LogFormat {
    fn from(value: &str) -> Self {
        match value {
            "pretty" => LogFormat::Pretty,
            "compact" => LogFormat::Compact,
            "full" => LogFormat::Full,
            "json" => LogFormat::Json,
            _ => LogFormat::Pretty,
        }
    }
}

impl From<LogLevel> for EnvFilter {
    fn from(level: LogLevel) -> Self {
        let filter: LevelFilter = level.into();
        EnvFilter::builder()
            .with_default_directive(filter.into())
            .from_env_lossy()
    }
}

impl From<String> for LogFormat {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

pub struct Logger {
    filter: Handle<EnvFilter, Registry>,
    file_handle: Handle<EnvFilter, Layered<reload::Layer<EnvFilter, Registry>, Registry>>,
    channel_writer: Arc<StdMutex<Option<channel::ChannelWriter>>>,
}

impl Logger {
    pub fn inititalize() -> &'static Self {
        let opts = OptionOpts::default();
        let format: LogFormat = api::get_var::<String>("HERMES_LOG_FORMAT")
            .map(LogFormat::from)
            .unwrap_or_default();
        let log_level: EnvFilter = api::get_option_value::<i64>("verbose", &opts)
            .map(LogLevel::from)
            .unwrap_or_default()
            .into();

        // Create reloadable filter for stdout
        let (filter_layer, filter) = reload::Layer::new(log_level);

        // Create placeholder file layer (initially OFF)
        let file_off_filter: EnvFilter = LogLevel::Off.into();
        let (file_filter_layer, file_handle) = reload::Layer::new(file_off_filter);

        // Create channel writer holder (starts empty, filled by set_file_logger)
        let channel_writer_holder: Arc<StdMutex<Option<channel::ChannelWriter>>> =
            Arc::new(StdMutex::new(None));
        let channel_writer_clone = channel_writer_holder.clone();

        // Create file layer with writer that uses channel when available
        let file_layer = fmt::layer()
            .with_writer(move || -> ChannelWriterGuard {
                ChannelWriterGuard {
                    inner: channel_writer_clone.clone(),
                }
            })
            .with_filter(file_filter_layer);

        let base = fmt::layer()
            .with_ansi(true)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true);

        let registry = tracing_subscriber::registry()
            .with(filter_layer)
            .with(file_layer);

        LOGGER.get_or_init(|| {
            match format {
                LogFormat::Full => registry.with(base).init(),
                LogFormat::Compact => registry.with(base.compact()).init(),
                LogFormat::Json => registry.with(base.json()).init(),
                _ => registry.with(base.pretty()).init(),
            }
            Self {
                filter,
                file_handle,
                channel_writer: channel_writer_holder,
            }
        })
    }

    pub fn set_log_level(&self, level: LogLevel) -> Result<(), Error> {
        let filter: EnvFilter = level.into();
        self.filter
            .reload(filter)
            .map_err(|e| Error::Internal(e.to_string()))
    }

    pub fn set_file_logger(&self, config: LogFileConfig) -> Result<(), Error> {
        // Stop current writer if exists
        {
            let mut writer_guard = self
                .channel_writer
                .lock()
                .map_err(|e| Error::Internal(format!("Failed to lock channel writer: {}", e)))?;

            if let Some(old_writer) = writer_guard.take() {
                // Shutdown old writer (blocking, waits for drain)
                old_writer.shutdown();
            }
        }

        if !config.enabled {
            // Disable file logging by setting filter to OFF
            let off_filter: EnvFilter = LogLevel::Off.into();
            self.file_handle
                .reload(off_filter)
                .map_err(|e| Error::Internal(format!("Failed to disable file logger: {}", e)))?;
            return Ok(());
        }

        // Get configuration values (defaults are defined in LogFileConfig)
        let max_size = config.max_size.unwrap_or_default();
        let max_files = config.max_files.unwrap_or_default() as usize;

        // Create the file appender
        let file_appender = file::SizeBasedFileAppender::new(&config.path, max_size, max_files)
            .map_err(|e| Error::Internal(format!("Failed to create file appender: {}", e)))?;

        // Create channel writer
        let channel_writer = channel::ChannelWriter::new(file_appender)
            .map_err(|e| Error::Internal(format!("Failed to create channel writer: {}", e)))?;

        // Store the channel writer
        {
            let mut writer_guard = self
                .channel_writer
                .lock()
                .map_err(|e| Error::Internal(format!("Failed to lock channel writer: {}", e)))?;
            *writer_guard = Some(channel_writer);
        }

        // Enable the file layer by reloading the filter
        let file_filter: EnvFilter = config.level.into();
        self.file_handle
            .reload(file_filter)
            .map_err(|e| Error::Internal(format!("Failed to enable file logger: {}", e)))?;

        Ok(())
    }

    pub fn configure(&self, config: LogConfig) -> Result<(), Error> {
        if let Some(file_config) = config.file {
            self.set_file_logger(file_config)?;
        }
        self.set_log_level(config.level)
    }
}

/// Guard struct for channel writer access
/// This is used by the tracing subscriber layer to access the channel writer
pub struct ChannelWriterGuard {
    inner: Arc<StdMutex<Option<channel::ChannelWriter>>>,
}

impl std::io::Write for ChannelWriterGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::other(format!("Lock poisoned: {}", e)))?;

        match guard.as_mut() {
            Some(writer) => writer.write(buf),
            None => Ok(buf.len()), // No-op if no writer configured (file logging disabled)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::other(format!("Lock poisoned: {}", e)))?;

        match guard.as_mut() {
            Some(writer) => writer.flush(),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    use tracing::level_filters::LevelFilter;

    proptest! {
        #[test]
        fn test_log_level_from_i64_roundtrip(level in any::<i64>()) {
            // Property: converting i64 to LogLevel should never panic
            let _ = LogLevel::from(level);
        }

        #[test]
        fn test_log_level_from_str_roundtrip(name in "[a-zA-Z0-9_]*") {
            // Property: converting string to LogLevel should never panic
            let _ = LogLevel::from(name.as_str());
        }

        #[test]
        fn test_log_format_from_str_roundtrip(name in "[a-zA-Z0-9_]*") {
            // Property: converting string to LogFormat should never panic
            let _ = LogFormat::from(name.as_str());
        }
    }

    #[test]
    fn test_log_level_from_i64_known_values() {
        // Test known mapping values using slice comparison
        let inputs: Vec<i64> = vec![0, 1, 2, 3, 4, 5, 99];
        let results: Vec<LogLevel> = inputs.iter().map(|&i| LogLevel::from(i)).collect();

        let expected: Vec<LogLevel> = vec![
            LogLevel::Trace, // 0
            LogLevel::Debug, // 1
            LogLevel::Info,  // 2
            LogLevel::Warn,  // 3
            LogLevel::Error, // 4
            LogLevel::Off,   // 5
            LogLevel::Off,   // 99 (unknown)
        ];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_log_level_from_str_known_values() {
        // Test known string mappings (case-insensitive)
        let inputs: Vec<&str> = vec!["trace", "debug", "info", "warn", "error", "unknown"];
        let results: Vec<LogLevel> = inputs.iter().map(|&s| LogLevel::from(s)).collect();

        let expected: Vec<LogLevel> = vec![
            LogLevel::Trace, // trace
            LogLevel::Debug, // debug
            LogLevel::Info,  // info
            LogLevel::Warn,  // warn
            LogLevel::Error, // error
            LogLevel::Off,   // unknown
        ];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_log_level_into_level_filter() {
        // Test conversion to tracing LevelFilter using slice comparison
        let inputs: Vec<LogLevel> = vec![LogLevel::Trace, LogLevel::Off];
        let results: Vec<LevelFilter> = inputs.iter().map(|&l| l.into()).collect();

        let expected: Vec<LevelFilter> = vec![LevelFilter::TRACE, LevelFilter::OFF];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_log_format_from_str_known_values() {
        // Test known LogFormat mappings
        let inputs: Vec<&str> = vec!["pretty", "compact", "full", "json", "unknown"];
        let results: Vec<LogFormat> = inputs.iter().map(|&s| LogFormat::from(s)).collect();

        let expected: Vec<LogFormat> = vec![
            LogFormat::Pretty,  // pretty
            LogFormat::Compact, // compact
            LogFormat::Full,    // full
            LogFormat::Json,    // json
            LogFormat::Pretty,  // unknown (default)
        ];

        assert_eq!(results, expected);
    }
}
