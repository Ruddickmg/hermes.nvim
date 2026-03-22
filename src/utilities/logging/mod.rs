use nvim_oxi::api::types::LogLevel as NvimLogLevel;
use std::sync::Mutex as StdMutex;
use std::sync::{Arc, OnceLock};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter::Filtered;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{
    EnvFilter, Registry, fmt,
    prelude::*,
    reload::{self},
};

use crate::nvim::configuration::LogTargetConfig;
use crate::utilities::logging::writer::NotifyWriter;
use crate::{
    acp::{Result, error::Error},
    nvim::configuration::LogConfig,
};

mod writer;

pub mod channel;
pub mod file;
pub mod sink;

use sink::{FileSink, LogSink, QuickfixSink};

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

impl From<LogLevel> for NvimLogLevel {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => NvimLogLevel::Trace,
            LogLevel::Debug => NvimLogLevel::Debug,
            LogLevel::Info => NvimLogLevel::Info,
            LogLevel::Warn => NvimLogLevel::Warn,
            LogLevel::Error => NvimLogLevel::Error,
            LogLevel::Off => NvimLogLevel::Off,
        }
    }
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

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Off => tracing::Level::ERROR, // Off maps to most restrictive
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
    fn from_object(
        obj: nvim_oxi::Object,
    ) -> std::result::Result<Self, nvim_oxi::conversion::Error> {
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
    Pretty,
    #[default]
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
    fn from_object(
        obj: nvim_oxi::Object,
    ) -> std::result::Result<Self, nvim_oxi::conversion::Error> {
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

/// Type alias for boxed layer
type BoxedLayer = Box<dyn tracing_subscriber::layer::Layer<Registry> + Send + Sync + 'static>;
type BoxedLayers = Filtered<BoxedLayer, EnvFilter, Registry>;

/// Logger that supports multiple output targets
pub struct Logger {
    handle: reload::Handle<Vec<BoxedLayers>, Registry>,
}

impl Logger {
    fn filter_layer(level: LogLevel) -> EnvFilter {
        let log_level: EnvFilter = level.into();
        log_level
    }

    fn extend_layer(layer: fmt::Layer<Registry>, format: LogFormat) -> BoxedLayer {
        let base = layer
            .with_ansi(true)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true);
        match format {
            LogFormat::Full => base.boxed(),
            LogFormat::Compact => base.compact().boxed(),
            LogFormat::Json => base.json().boxed(),
            _ => base.pretty().boxed(),
        }
    }

    fn format_layer(format: LogFormat) -> BoxedLayer {
        Self::extend_layer(fmt::Layer::new(), format)
    }

    fn combine_layers(config: LogTargetConfig) -> BoxedLayers {
        Self::format_layer(config.format).with_filter(Self::filter_layer(config.level))
    }

    pub fn notification_layer(config: LogTargetConfig) -> BoxedLayers {
        let writer = NotifyWriter::new(config.level);
        Self::extend_layer(fmt::layer().with_writer(move || writer), config.format)
            .with_filter(Self::filter_layer(config.level))
    }

    pub fn all_layers(
        LogConfig {
            stdio,
            notification,
            quickfix,
            message,
            ..
        }: LogConfig,
    ) -> Vec<BoxedLayers> {
        vec![stdio, notification, message, quickfix]
            .into_iter()
            .map(Self::combine_layers)
            .collect()
    }

    pub fn inititalize() -> &'static Self {
        let layers: Vec<BoxedLayers> = Self::all_layers(Default::default());
        let (reload_layer, handle) = reload::Layer::new(layers);

        let subscriber = tracing_subscriber::registry().with(reload_layer);

        LOGGER.get_or_init(|| {
            subscriber.init();
            Self { handle }
        })
    }

    pub fn configure(&self, config: LogConfig) -> Result<()> {
        let layers = Self::all_layers(config);
        self.handle
            .reload(layers)
            .map_err(|e| Error::Internal(e.to_string()))
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
