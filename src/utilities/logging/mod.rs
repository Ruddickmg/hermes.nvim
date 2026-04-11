use std::io;
use std::sync::OnceLock;
use tracing_subscriber::{
    Registry, fmt,
    prelude::*,
    reload::{self},
};

use crate::utilities::logging::writer::{AnsiStrip, FileWriter, Filtered, StdoutWriter};
use crate::utilities::message_messenger::MessageMessenger;
use crate::utilities::notification_messenger::NotificationMessenger;
use crate::utilities::writer::MessageWriter;
use crate::{
    acp::{Result, error::Error},
    nvim::configuration::LogConfig,
};
use crate::{
    nvim::configuration::{LogFileConfig, LogTargetConfig},
    utilities::writer::NotifyWriter,
};

pub mod channel;
pub mod file;
pub mod format;
pub mod level;
pub mod sink;
pub mod writer;
pub use format::*;
pub use level::*;

use sink::FileSink;

static LOGGER: OnceLock<Logger> = OnceLock::new();

const LOG_FILE_NAME: &str = "hermes.log";

pub type FileChannel = channel::ChannelWriter<FileSink>;
type BoxedLayer = Box<dyn tracing_subscriber::layer::Layer<Registry> + Send + Sync + 'static>;

/// Logger that supports multiple output targets
pub struct Logger {
    handle: reload::Handle<Vec<BoxedLayer>, Registry>,
    storage_path: String,
    pub nvim_messages_messenger: MessageMessenger,
    pub nvim_notifications_messenger: NotificationMessenger,
}

impl Logger {
    fn base_layer<W>(
        layer: fmt::Layer<Registry, fmt::format::DefaultFields, fmt::format::Format, W>,
        format: LogFormat,
    ) -> BoxedLayer
    where
        W: for<'a> fmt::writer::MakeWriter<'a> + Send + Sync + 'static,
    {
        let base = layer
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true);
        match format {
            LogFormat::Full => base.boxed(),
            LogFormat::Compact => base.compact().boxed(),
            LogFormat::Json => base.json().boxed(),
            LogFormat::Pretty => base.pretty().boxed(),
        }
    }

    fn stdio_layer(config: LogTargetConfig) -> BoxedLayer {
        let writer = StdoutWriter::new().filtered(config.level);
        Self::base_layer(
            fmt::layer().with_writer(writer).with_ansi(true),
            config.format,
        )
    }

    fn notification_layer(
        config: LogTargetConfig,
        messenger: NotificationMessenger,
    ) -> Result<BoxedLayer> {
        let writer = NotifyWriter::new(config.level, messenger).filtered(config.level);
        Ok(Self::base_layer(
            fmt::layer().with_writer(writer).with_ansi(true),
            config.format,
        ))
    }

    fn message_layer(config: LogTargetConfig, messenger: MessageMessenger) -> Result<BoxedLayer> {
        let writer = AnsiStrip::new(MessageWriter::new(messenger)).filtered(config.level);
        Ok(Self::base_layer(
            fmt::layer().with_writer(writer.clone()).with_ansi(false),
            config.format,
        ))
    }

    fn file_layer(config: LogFileConfig) -> io::Result<Option<BoxedLayer>> {
        let log_file_path = std::path::Path::new(&config.path).join(LOG_FILE_NAME);
        let writer = AnsiStrip::new(FileWriter::new(
            &log_file_path,
            config.max_size,
            config.max_files as usize,
        )?)
        .filtered(config.level);

        Ok(Some(Self::base_layer(
            fmt::layer().with_writer(writer).with_ansi(false),
            config.format,
        )))
    }

    fn all_layers(
        LogConfig {
            stdio,
            message,
            notification,
            file,
        }: LogConfig,
        notifications_messenger: NotificationMessenger,
        messages_messenger: MessageMessenger,
    ) -> Result<Vec<BoxedLayer>> {
        let mut layers: Vec<BoxedLayer> = vec![];

        if stdio.level != LogLevel::Off {
            layers.push(Self::stdio_layer(stdio));
        }

        if message.level != LogLevel::Off {
            layers.push(Self::message_layer(message, messages_messenger)?);
        }

        if notification.level != LogLevel::Off {
            layers.push(Self::notification_layer(
                notification,
                notifications_messenger,
            )?);
        }

        if file.is_enabled()
            && let Some(file_layer) =
                Self::file_layer(file).map_err(|e| Error::Internal(e.to_string()))?
        {
            layers.push(file_layer);
        }

        Ok(layers)
    }

    pub fn default_config(storage_path: &str) -> LogConfig {
        let file_config = LogFileConfig {
            path: storage_path.to_string(),
            ..Default::default()
        };
        LogConfig {
            file: file_config,
            ..Default::default()
        }
    }

    pub fn inititalize(storage_path: &str) -> Result<&'static Self> {
        let config = Self::default_config(storage_path);
        // Check if global subscriber already exists (reload scenario)
        if LOGGER.get().is_some() {
            // Reload: Get cached logger and rebuild layers with the cached messengers
            let logger = LOGGER
                .get()
                .ok_or_else(|| Error::Internal("Logger cached but not found".into()))?;

            // Reuse the cached messengers so future reconfiguration stays consistent
            let layers = Self::all_layers(
                config,
                logger.nvim_notifications_messenger.clone(),
                logger.nvim_messages_messenger.clone(),
            )?;
            // Reload the layers in the global subscriber
            logger
                .handle
                .reload(layers)
                .map_err(|e| Error::Internal(e.to_string()))?;

            return Ok(logger);
        }

        // First initialization: Create new global subscriber
        let nvim_notifications_messenger = NotificationMessenger::initialize()?;
        let nvim_messages_messenger = MessageMessenger::initialize()?;
        let layers: Vec<BoxedLayer> = Self::all_layers(
            config,
            nvim_notifications_messenger.clone(),
            nvim_messages_messenger.clone(),
        )?;
        let (reload_layer, handle) = reload::Layer::new(layers);

        let subscriber = tracing_subscriber::registry().with(reload_layer);

        Ok(LOGGER.get_or_init(|| {
            // Use try_init to avoid panicking if a global subscriber is already set.
            // This can happen when the binary is reloaded (e.g., in tests).
            if tracing::subscriber::set_global_default(subscriber).is_err() {
                // Global subscriber already set, that's fine - we'll reuse it
            }
            Self {
                handle,
                storage_path: storage_path.to_string(),
                nvim_messages_messenger,
                nvim_notifications_messenger,
            }
        }))
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn configure(&self, mut configuration: LogConfig) -> Result<()> {
        if configuration.file.path.is_empty() {
            configuration.file.path = self.storage_path.clone();
        }
        let layers = Self::all_layers(
            configuration,
            self.nvim_notifications_messenger.clone(),
            self.nvim_messages_messenger.clone(),
        )?;
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
    fn test_file_layer_succeeds_with_valid_path() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = LogFileConfig {
            path: temp_dir.path().to_string_lossy().to_string(),
            level: LogLevel::Debug,
            format: LogFormat::Full,
            max_size: 10_485_760,
            max_files: 5,
        };

        let result = Logger::file_layer(config);

        assert!(result.is_ok(), "file_layer should succeed with valid path");
    }

    #[test]
    fn test_file_layer_creates_log_file_at_valid_path() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config = LogFileConfig {
            path: temp_dir.path().to_string_lossy().to_string(),
            level: LogLevel::Debug,
            format: LogFormat::Full,
            max_size: 10_485_760,
            max_files: 5,
        };

        let layer = Logger::file_layer(config).unwrap();
        drop(layer);

        let expected_path = temp_dir.path().join("hermes.log");
        assert!(
            expected_path.exists(),
            "hermes.log should be created in temp directory"
        );
    }

    #[test]
    fn test_file_layer_creates_nested_directories() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("hermes/nested/logs");

        let config = LogFileConfig {
            path: nested_path.to_string_lossy().to_string(),
            level: LogLevel::Debug,
            format: LogFormat::Full,
            max_size: 10_485_760,
            max_files: 5,
        };

        let layer = Logger::file_layer(config).unwrap();
        drop(layer);

        assert!(
            nested_path.exists(),
            "nested directory structure should be created"
        );
    }

    #[test]
    fn test_file_layer_creates_log_file_in_nested_directory() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("hermes/nested/logs");

        let config = LogFileConfig {
            path: nested_path.to_string_lossy().to_string(),
            level: LogLevel::Debug,
            format: LogFormat::Full,
            max_size: 10_485_760,
            max_files: 5,
        };

        let layer = Logger::file_layer(config).unwrap();
        drop(layer);

        let expected_path = nested_path.join("hermes.log");
        assert!(
            expected_path.exists(),
            "hermes.log should be created in nested directory"
        );
    }
}
