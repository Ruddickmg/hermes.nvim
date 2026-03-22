use std::sync::OnceLock;
use tracing_subscriber::filter::Filtered;
use tracing_subscriber::{
    EnvFilter, Registry, fmt,
    prelude::*,
    reload::{self},
};

use crate::nvim::configuration::{LogFileConfig, LogTargetConfig};
use crate::utilities::logging::writer::NotifyWriter;
use crate::utilities::writer::MessageWriter;
use crate::{
    acp::{Result, error::Error},
    nvim::configuration::LogConfig,
};

pub mod format;
pub mod level;
pub mod writer;
pub use format::*;
pub use level::*;
pub mod channel;

pub mod file;
pub mod sink;

use sink::FileSink;

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub type FileChannel = channel::ChannelWriter<FileSink>;
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

    fn extend_layer<W>(
        layer: fmt::Layer<Registry, fmt::format::DefaultFields, fmt::format::Format, W>,
        format: LogFormat,
    ) -> BoxedLayer
    where
        W: for<'a> fmt::writer::MakeWriter<'a> + Send + Sync + 'static,
    {
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

    fn stdio_layer(config: LogTargetConfig) -> BoxedLayers {
        Self::format_layer(config.format).with_filter(Self::filter_layer(config.level))
    }

    fn notification_layer(config: LogTargetConfig) -> BoxedLayers {
        let writer = NotifyWriter::new(config.level);
        Self::extend_layer(
            fmt::layer().with_writer(move || writer.clone()),
            config.format,
        )
        .with_filter(Self::filter_layer(config.level))
    }

    fn message_layer(config: LogTargetConfig) -> BoxedLayers {
        Self::extend_layer(
            fmt::layer().with_writer(MessageWriter::default),
            config.format,
        )
        .with_filter(Self::filter_layer(config.level))
    }

    fn file_layer(config: LogFileConfig) -> BoxedLayers {
        let writer = NotifyWriter::new(config.level);
        Self::extend_layer(
            fmt::layer().with_writer(move || writer.clone()),
            config.format,
        )
        .with_filter(Self::filter_layer(config.level))
    }

    pub fn all_layers(
        LogConfig {
            stdio,
            message,
            notification,
            file,
        }: LogConfig,
    ) -> Vec<BoxedLayers> {
        vec![
            Self::stdio_layer(stdio),
            Self::message_layer(message),
            Self::notification_layer(notification),
            Self::file_layer(file),
        ]
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
