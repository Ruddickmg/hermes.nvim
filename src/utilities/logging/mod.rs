use std::sync::Mutex as StdMutex;
use std::sync::{Arc, OnceLock};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, Registry, fmt,
    prelude::*,
    reload::{self, Handle},
};

use crate::{
    acp::error::Error,
    nvim::configuration::{LogConfig, LogFileConfig},
};

pub mod channel;
pub mod file;
pub mod sink;

use sink::{FileSink, LogSink, MessageSink, NotificationSink, QuickfixSink};

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
        if let Ok(s) = String::from_object(obj.clone()) {
            Ok(LogLevel::from(s))
        } else if let Ok(n) = i64::from_object(obj) {
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

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFormat::Pretty => write!(f, "pretty"),
            LogFormat::Compact => write!(f, "compact"),
            LogFormat::Full => write!(f, "full"),
            LogFormat::Json => write!(f, "json"),
        }
    }
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

impl From<String> for LogFormat {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl nvim_oxi::conversion::FromObject for LogFormat {
    fn from_object(obj: nvim_oxi::Object) -> Result<Self, nvim_oxi::conversion::Error> {
        if let Ok(s) = String::from_object(obj.clone()) {
            Ok(LogFormat::from(s))
        } else {
            Err(nvim_oxi::conversion::Error::Other(
                "LogFormat must be a string".to_string(),
            ))
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

/// Type aliases for different channel writers (only for blocking operations)
pub type FileChannel = channel::ChannelWriter<FileSink>;
pub type QuickfixChannel = channel::ChannelWriter<QuickfixSink>;

/// Logger that supports multiple output targets
pub struct Logger {
    filter: Handle<EnvFilter, Registry>,
    file_handle: Handle<EnvFilter, Registry>,
    file_writer: Arc<StdMutex<Option<FileChannel>>>,
    quickfix_writer: Arc<StdMutex<Option<QuickfixChannel>>>,
}

impl Logger {
    pub fn inititalize() -> &'static Self {
        // Use default format (Compact) until configure() is called
        // This avoids needing global variables - configuration happens via setup()
        let format = LogFormat::Compact;
        let log_level = EnvFilter::new("info");

        // Create channel writer holders (start empty) - only for blocking operations
        let file_writer_holder: Arc<StdMutex<Option<FileChannel>>> = Arc::new(StdMutex::new(None));
        let file_writer_clone = file_writer_holder.clone();

        let quickfix_writer_holder: Arc<StdMutex<Option<QuickfixChannel>>> =
            Arc::new(StdMutex::new(None));
        let quickfix_writer_clone = quickfix_writer_holder.clone();

        // Create sinks for notification and message (reused across format changes)
        let notification_sink = Arc::new(StdMutex::new(NotificationSink::new()));
        let notification_sink_clone = notification_sink.clone();

        let message_sink = Arc::new(StdMutex::new(MessageSink::new()));
        let message_sink_clone = message_sink.clone();

        // Build all layers in a Vec to avoid complex type composition
        // Each layer is boxed so they have uniform type
        let mut layers: Vec<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>> =
            Vec::new();

        // Add stdout filter layer (reloadable)
        let (filter_layer, filter) = reload::Layer::new(log_level);
        layers.push(Box::new(filter_layer));

        // Add file layer with its filter
        let file_off_filter: EnvFilter = LogLevel::Off.into();
        let (file_filter_layer, file_handle) = reload::Layer::new(file_off_filter);
        let file_layer = fmt::layer()
            .with_writer(move || -> ChannelWriterGuard<FileSink> {
                ChannelWriterGuard::new(file_writer_clone.clone())
            })
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_filter(file_filter_layer);
        layers.push(Box::new(file_layer));

        // Add quickfix layer with user-selected format
        let quickfix_layer: Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> =
            match format {
                LogFormat::Full => Box::new(
                    fmt::layer()
                        .with_writer(move || -> ChannelWriterGuard<QuickfixSink> {
                            ChannelWriterGuard::new(quickfix_writer_clone.clone())
                        })
                        .with_ansi(false)
                        .with_file(true)
                        .with_line_number(true),
                ),
                LogFormat::Compact => Box::new(
                    fmt::layer()
                        .with_writer(move || -> ChannelWriterGuard<QuickfixSink> {
                            ChannelWriterGuard::new(quickfix_writer_clone.clone())
                        })
                        .with_ansi(false)
                        .with_file(true)
                        .with_line_number(true)
                        .compact(),
                ),
                LogFormat::Json => Box::new(
                    fmt::layer()
                        .with_writer(move || -> ChannelWriterGuard<QuickfixSink> {
                            ChannelWriterGuard::new(quickfix_writer_clone.clone())
                        })
                        .with_ansi(false)
                        .with_file(true)
                        .with_line_number(true)
                        .json(),
                ),
                LogFormat::Pretty => Box::new(
                    fmt::layer()
                        .with_writer(move || -> ChannelWriterGuard<QuickfixSink> {
                            ChannelWriterGuard::new(quickfix_writer_clone.clone())
                        })
                        .with_ansi(false)
                        .with_file(true)
                        .with_line_number(true)
                        .pretty(),
                ),
            };
        layers.push(quickfix_layer);

        // Add notification layer with user-selected format
        let notification_layer: Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> =
            match format {
                LogFormat::Full => Box::new(
                    fmt::layer()
                        .with_writer(move || -> DirectWriterGuard<NotificationSink> {
                            DirectWriterGuard::new(notification_sink_clone.clone())
                        })
                        .with_ansi(false),
                ),
                LogFormat::Compact => Box::new(
                    fmt::layer()
                        .with_writer(move || -> DirectWriterGuard<NotificationSink> {
                            DirectWriterGuard::new(notification_sink_clone.clone())
                        })
                        .with_ansi(false)
                        .compact(),
                ),
                LogFormat::Json => Box::new(
                    fmt::layer()
                        .with_writer(move || -> DirectWriterGuard<NotificationSink> {
                            DirectWriterGuard::new(notification_sink_clone.clone())
                        })
                        .with_ansi(false)
                        .json(),
                ),
                LogFormat::Pretty => Box::new(
                    fmt::layer()
                        .with_writer(move || -> DirectWriterGuard<NotificationSink> {
                            DirectWriterGuard::new(notification_sink_clone.clone())
                        })
                        .with_ansi(false)
                        .pretty(),
                ),
            };
        layers.push(notification_layer);

        // Add message layer with user-selected format
        let message_layer: Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> = match format
        {
            LogFormat::Full => Box::new(
                fmt::layer()
                    .with_writer(move || -> DirectWriterGuard<MessageSink> {
                        DirectWriterGuard::new(message_sink_clone.clone())
                    })
                    .with_ansi(true),
            ),
            LogFormat::Compact => Box::new(
                fmt::layer()
                    .with_writer(move || -> DirectWriterGuard<MessageSink> {
                        DirectWriterGuard::new(message_sink_clone.clone())
                    })
                    .with_ansi(true)
                    .compact(),
            ),
            LogFormat::Json => Box::new(
                fmt::layer()
                    .with_writer(move || -> DirectWriterGuard<MessageSink> {
                        DirectWriterGuard::new(message_sink_clone.clone())
                    })
                    .with_ansi(true)
                    .json(),
            ),
            LogFormat::Pretty => Box::new(
                fmt::layer()
                    .with_writer(move || -> DirectWriterGuard<MessageSink> {
                        DirectWriterGuard::new(message_sink_clone.clone())
                    })
                    .with_ansi(true)
                    .pretty(),
            ),
        };
        layers.push(message_layer);

        // Build the subscriber with all layers
        let subscriber = tracing_subscriber::registry().with(layers);

        LOGGER.get_or_init(|| {
            subscriber.init();
            Self {
                filter,
                file_handle,
                file_writer: file_writer_holder,
                quickfix_writer: quickfix_writer_holder,
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
                .file_writer
                .lock()
                .map_err(|e| Error::Internal(format!("Failed to lock file writer: {}", e)))?;

            if let Some(old_writer) = writer_guard.take() {
                old_writer.shutdown();
            }
        }

        let max_size = config.max_size.unwrap_or_default();
        let max_files = config.max_files.unwrap_or_default() as usize;

        let file_sink = FileSink::new(&config.path, max_size, max_files)
            .map_err(|e| Error::Internal(format!("Failed to create file sink: {}", e)))?;

        let channel_writer = channel::ChannelWriter::new_file(file_sink)
            .map_err(|e| Error::Internal(format!("Failed to create file channel writer: {}", e)))?;

        {
            let mut writer_guard = self
                .file_writer
                .lock()
                .map_err(|e| Error::Internal(format!("Failed to lock file writer: {}", e)))?;
            *writer_guard = Some(channel_writer);
        }

        let file_filter: EnvFilter = config.level.into();
        self.file_handle
            .reload(file_filter)
            .map_err(|e| Error::Internal(format!("Failed to enable file logger: {}", e)))?;

        Ok(())
    }

    pub fn set_notification_logger(&self, _level: LogLevel) -> Result<(), Error> {
        // Notification layer is created during initialize() with global format
        // Level filtering is handled by the tracing filter layer
        Ok(())
    }

    pub fn set_message_logger(&self, _level: LogLevel) -> Result<(), Error> {
        // Message layer is created during initialize() with global format
        // Level filtering is handled by the tracing filter layer
        Ok(())
    }

    pub fn set_quickfix_logger(&self, level: LogLevel) -> Result<(), Error> {
        {
            let mut writer_guard = self
                .quickfix_writer
                .lock()
                .map_err(|e| Error::Internal(format!("Failed to lock quickfix writer: {}", e)))?;

            if let Some(old_writer) = writer_guard.take() {
                old_writer.shutdown();
            }
        }

        if level == LogLevel::Off {
            return Ok(());
        }

        let quickfix_sink = QuickfixSink::new();

        let channel_writer = channel::ChannelWriter::new_ui(quickfix_sink).map_err(|e| {
            Error::Internal(format!("Failed to create quickfix channel writer: {}", e))
        })?;

        {
            let mut writer_guard = self
                .quickfix_writer
                .lock()
                .map_err(|e| Error::Internal(format!("Failed to lock quickfix writer: {}", e)))?;
            *writer_guard = Some(channel_writer);
        }

        Ok(())
    }

    pub fn configure(&self, config: LogConfig) -> Result<(), Error> {
        // Configure file logging
        if let Some(file_config) = config.file {
            self.set_file_logger(file_config)?;
        }

        // Note: Formats are set during initialize() with default "compact"
        // Per-target format configuration would require reinitializing the logger

        // Configure levels for each target
        self.set_notification_logger(config.notification.level)?;
        self.set_message_logger(config.message.level)?;
        self.set_quickfix_logger(config.quickfix.level)?;
        self.set_log_level(config.stdio.level)?;

        Ok(())
    }
}

/// Guard struct for channel writer access
pub struct ChannelWriterGuard<S: LogSink> {
    inner: Arc<StdMutex<Option<channel::ChannelWriter<S>>>>,
}

impl<S: LogSink> ChannelWriterGuard<S> {
    pub fn new(inner: Arc<StdMutex<Option<channel::ChannelWriter<S>>>>) -> Self {
        Self { inner }
    }
}

impl<S: LogSink> std::io::Write for ChannelWriterGuard<S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::other(format!("Lock poisoned: {}", e)))?;

        match guard.as_mut() {
            Some(writer) => writer.write(buf),
            None => Ok(buf.len()),
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

/// Guard struct for direct writer access (non-blocking sinks)
pub struct DirectWriterGuard<S> {
    inner: Arc<StdMutex<S>>,
}

impl<S> DirectWriterGuard<S> {
    pub fn new(inner: Arc<StdMutex<S>>) -> Self {
        Self { inner }
    }
}

impl<S: std::io::Write> std::io::Write for DirectWriterGuard<S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::other(format!("Lock poisoned: {}", e)))?;
        guard.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::other(format!("Lock poisoned: {}", e)))?;
        guard.flush()
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
            let _ = LogLevel::from(level);
        }

        #[test]
        fn test_log_level_from_str_roundtrip(name in "[a-zA-Z0-9_]*") {
            let _ = LogLevel::from(name.as_str());
        }

        #[test]
        fn test_log_format_from_str_roundtrip(name in "[a-zA-Z0-9_]*") {
            let _ = LogFormat::from(name.as_str());
        }
    }

    #[test]
    fn test_log_level_from_i64_known_values() {
        let inputs: Vec<i64> = vec![0, 1, 2, 3, 4, 5, 99];
        let results: Vec<LogLevel> = inputs.iter().map(|&i| LogLevel::from(i)).collect();

        let expected: Vec<LogLevel> = vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Off,
            LogLevel::Off,
        ];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_log_level_from_str_known_values() {
        let inputs: Vec<&str> = vec!["trace", "debug", "info", "warn", "error", "unknown"];
        let results: Vec<LogLevel> = inputs.iter().map(|&s| LogLevel::from(s)).collect();

        let expected: Vec<LogLevel> = vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
            LogLevel::Off,
        ];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_log_level_into_level_filter() {
        let inputs: Vec<LogLevel> = vec![LogLevel::Trace, LogLevel::Off];
        let results: Vec<LevelFilter> = inputs.iter().map(|&l| l.into()).collect();

        let expected: Vec<LevelFilter> = vec![LevelFilter::TRACE, LevelFilter::OFF];

        assert_eq!(results, expected);
    }

    #[test]
    fn test_log_format_display_pretty() {
        assert_eq!(LogFormat::Pretty.to_string(), "pretty");
    }

    #[test]
    fn test_log_format_display_compact() {
        assert_eq!(LogFormat::Compact.to_string(), "compact");
    }

    #[test]
    fn test_log_format_display_full() {
        assert_eq!(LogFormat::Full.to_string(), "full");
    }

    #[test]
    fn test_log_format_display_json() {
        assert_eq!(LogFormat::Json.to_string(), "json");
    }

    #[test]
    fn test_log_format_from_string_pretty() {
        let pretty: LogFormat = "pretty".to_string().into();
        assert_eq!(pretty, LogFormat::Pretty);
    }

    #[test]
    fn test_log_format_from_string_compact() {
        let compact: LogFormat = "compact".to_string().into();
        assert_eq!(compact, LogFormat::Compact);
    }

    #[test]
    fn test_log_format_from_string_full() {
        let full: LogFormat = "full".to_string().into();
        assert_eq!(full, LogFormat::Full);
    }

    #[test]
    fn test_log_format_from_string_json() {
        let json: LogFormat = "json".to_string().into();
        assert_eq!(json, LogFormat::Json);
    }

    #[test]
    fn test_log_format_from_string_unknown_defaults_to_pretty() {
        let unknown: LogFormat = "unknown".to_string().into();
        assert_eq!(unknown, LogFormat::Pretty);
    }
}

