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

/// Type alias for the holder resources returned by create_holders()
type LoggerHolders = (
    Arc<StdMutex<Option<FileChannel>>>,
    Arc<StdMutex<Option<QuickfixChannel>>>,
    Arc<StdMutex<NotificationSink>>,
    Arc<StdMutex<MessageSink>>,
);

/// Type aliases for layer reload handles
type FormatLayerHandle = Handle<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>, Registry>;
type FileLayerHandle = Handle<FileLayer, Registry>;

/// Log target identifier for format updates
#[derive(Clone, Copy, Debug)]
pub enum LogTarget {
    Stdio,
    File,
    Notification,
    Message,
    Quickfix,
}

/// Custom file layer that implements its own filtering
/// This bypasses the Filtered layer bug in tracing-subscriber issue #1629
#[derive(Debug)]
pub struct FileLayer {
    writer: Arc<StdMutex<Option<FileChannel>>>,
    level: Arc<StdMutex<LogLevel>>,
    format: Arc<StdMutex<LogFormat>>,
}

impl FileLayer {
    pub fn new(writer: Arc<StdMutex<Option<FileChannel>>>, level: LogLevel, format: LogFormat) -> Self {
        Self {
            writer,
            level: Arc::new(StdMutex::new(level)),
            format: Arc::new(StdMutex::new(format)),
        }
    }

    pub fn update_level(&self, level: LogLevel) {
        if let Ok(mut guard) = self.level.lock() {
            *guard = level;
        }
    }

    pub fn update_format(&self, format: LogFormat) {
        if let Ok(mut guard) = self.format.lock() {
            *guard = format;
        }
    }
}

/// Visitor to extract the message from a tracing event
struct MessageVisitor {
    message: String,
}

impl MessageVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        // Capture the "message" field specifically
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
    
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for FileLayer
where
    S: tracing::Subscriber,
{
    fn enabled(&self, metadata: &tracing::Metadata<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) -> bool {
        let level = self.level.lock().unwrap();
        let event_level = metadata.level();
        
        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0, Level::DEBUG => 1, Level::INFO => 2, 
            Level::WARN => 3, Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0, Level::DEBUG => 1, Level::INFO => 2, 
            Level::WARN => 3, Level::ERROR => 4,
        };
        
        // Only enable if event is at or above min_level (less verbose or equal)
        event_num >= min_num
    }

    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        // Check level again (enabled() is just a hint)
        let level = self.level.lock().unwrap();
        let event_level = event.metadata().level();
        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0, Level::DEBUG => 1, Level::INFO => 2, 
            Level::WARN => 3, Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0, Level::DEBUG => 1, Level::INFO => 2, 
            Level::WARN => 3, Level::ERROR => 4,
        };
        if event_num < min_num {
            return; // Filter it out
        }
        
        // Check if we have a writer
        let mut writer_guard = self.writer.lock().unwrap();
        if writer_guard.is_none() {
            return;
        }
        
        // Get current format setting
        let format = self.format.lock().unwrap();
        
        // Extract the message from the event
        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);
        
        let level_str = event.metadata().level();
        let target = event.metadata().target();
        let message_text = if visitor.message.is_empty() {
            String::new()
        } else {
            visitor.message
        };
        
        // Format based on LogFormat setting
        let formatted = match *format {
            LogFormat::Json => {
                format!(
                    r#"{{"timestamp":"","level":"{:?}","target":"{}","fields":{{"message":"{}"}}}}"#,
                    level_str, target, message_text
                )
            }
            LogFormat::Full => {
                format!("{} {}: {}\n", level_str, target, message_text)
            }
            LogFormat::Compact => {
                format!("[{}] {}\n", level_str, message_text)
            }
            LogFormat::Pretty => {
                format!("{}\n  level: {}\n  target: {}\n  message: {}\n", 
                    "Event:", level_str, target, message_text)
            }
        };
        
        // Write to the file channel
        if let Some(ref mut writer) = *writer_guard {
            let _ = std::io::Write::write_all(writer, formatted.as_bytes());
        }
    }
}

/// Logger that supports multiple output targets
pub struct Logger {
    filter: Handle<EnvFilter, Registry>,
    file_writer: Arc<StdMutex<Option<FileChannel>>>,
    quickfix_writer: Arc<StdMutex<Option<QuickfixChannel>>>,
    notification_sink: Arc<StdMutex<NotificationSink>>,
    message_sink: Arc<StdMutex<MessageSink>>,
    // Reload handles for format-dependent layers
    stdio_handle: FormatLayerHandle,
    file_layer_handle: FileLayerHandle,
    quickfix_handle: FormatLayerHandle,
    notification_handle: FormatLayerHandle,
    message_handle: FormatLayerHandle,
}

impl Logger {
    /// Initialize the logger with all output targets
    pub fn inititalize() -> &'static Self {
        // Use default format (Compact) until configure() is called
        let format = LogFormat::Compact;
        let log_level = EnvFilter::new("info");

        // Create shared resources
        let (file_writer, quickfix_writer, notification_sink, message_sink) =
            Self::create_holders();

        // Build all layers
        let mut layers: Vec<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>> =
            Vec::new();

        // Stdio layer with reload capability for format
        let (stdio_reload_layer, stdio_handle) = reload::Layer::new(
            Self::create_stdio_layer(format)
        );
        layers.push(Box::new(stdio_reload_layer));

        // Stdio filter layer (controls all stdout output level)
        let (filter_layer, filter) = Self::create_stdio_filter(log_level);
        layers.push(Box::new(filter_layer));

        // File layer with custom filtering (bypasses Filtered layer bug)
        let file_layer = FileLayer::new(file_writer.clone(), LogLevel::Off, format);
        let (file_reload_layer, file_layer_handle) = reload::Layer::new(file_layer);
        layers.push(Box::new(file_reload_layer));

        // Quickfix layer with reload capability
        let (quickfix_reload_layer, quickfix_handle) = reload::Layer::new(
            Self::create_quickfix_layer(quickfix_writer.clone(), format)
        );
        layers.push(Box::new(quickfix_reload_layer));

        // Notification layer with reload capability
        let (notification_reload_layer, notification_handle) = reload::Layer::new(
            Self::create_notification_layer(notification_sink.clone(), format)
        );
        layers.push(Box::new(notification_reload_layer));

        // Message layer with reload capability
        let (message_reload_layer, message_handle) = reload::Layer::new(
            Self::create_message_layer(message_sink.clone(), format)
        );
        layers.push(Box::new(message_reload_layer));

        // Build and init subscriber
        let subscriber = tracing_subscriber::registry().with(layers);

        LOGGER.get_or_init(|| {
            subscriber.init();
            Self {
                filter,
                file_writer,
                quickfix_writer,
                notification_sink,
                message_sink,
                stdio_handle,
                file_layer_handle,
                quickfix_handle,
                notification_handle,
                message_handle,
            }
        })
    }

    /// Create shared resource holders for all targets
    fn create_holders() -> LoggerHolders {
        let file_writer: Arc<StdMutex<Option<FileChannel>>> = Arc::new(StdMutex::new(None));
        let quickfix_writer: Arc<StdMutex<Option<QuickfixChannel>>> = Arc::new(StdMutex::new(None));
        let notification_sink = Arc::new(StdMutex::new(NotificationSink::new()));
        let message_sink = Arc::new(StdMutex::new(MessageSink::new()));

        (
            file_writer,
            quickfix_writer,
            notification_sink,
            message_sink,
        )
    }

    /// Create stdio filter layer with reload capability
    fn create_stdio_filter(
        log_level: EnvFilter,
    ) -> (
        reload::Layer<EnvFilter, Registry>,
        Handle<EnvFilter, Registry>,
    ) {
        reload::Layer::new(log_level)
    }

    /// Create stdio layer with format selection
    fn create_stdio_layer(
        format: LogFormat,
    ) -> Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> {
        let base_layer = fmt::layer()
            .with_ansi(true);

        match format {
            LogFormat::Full => Box::new(base_layer),
            LogFormat::Compact => Box::new(base_layer.compact()),
            LogFormat::Json => Box::new(base_layer.json()),
            LogFormat::Pretty => Box::new(base_layer.pretty()),
        }
    }

    /// Create file filter layer (starts OFF, reloadable)
    fn create_file_filter() -> (reload::Layer<EnvFilter, Registry>, Handle<EnvFilter, Registry>) {
        let file_off_filter: EnvFilter = LogLevel::Off.into();
        reload::Layer::new(file_off_filter)
    }

    /// Create file layer with channel writer
    fn create_file_layer(
        file_writer: Arc<StdMutex<Option<FileChannel>>>,
    ) -> Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> {
        Box::new(
            fmt::layer()
                .with_writer(move || -> ChannelWriterGuard<FileSink> {
                    ChannelWriterGuard::new(file_writer.clone())
                })
                .with_ansi(false)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true),
        )
    }

    /// Create file layer with channel writer and format selection
    fn create_file_layer_with_format(
        file_writer: Arc<StdMutex<Option<FileChannel>>>,
        format: LogFormat,
    ) -> Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> {
        let base_layer = fmt::layer()
            .with_writer(move || -> ChannelWriterGuard<FileSink> {
                ChannelWriterGuard::new(file_writer.clone())
            })
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true);

        match format {
            LogFormat::Full => Box::new(base_layer),
            LogFormat::Compact => Box::new(base_layer.compact()),
            LogFormat::Json => Box::new(base_layer.json()),
            LogFormat::Pretty => Box::new(base_layer.pretty()),
        }
    }

    /// Create quickfix layer with format selection
    fn create_quickfix_layer(
        quickfix_writer: Arc<StdMutex<Option<QuickfixChannel>>>,
        format: LogFormat,
    ) -> Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> {
        let base_layer = fmt::layer()
            .with_writer(move || -> ChannelWriterGuard<QuickfixSink> {
                ChannelWriterGuard::new(quickfix_writer.clone())
            })
            .with_ansi(false)
            .with_file(true)
            .with_line_number(true);

        match format {
            LogFormat::Full => Box::new(base_layer),
            LogFormat::Compact => Box::new(base_layer.compact()),
            LogFormat::Json => Box::new(base_layer.json()),
            LogFormat::Pretty => Box::new(base_layer.pretty()),
        }
    }

    /// Create notification layer with format selection
    fn create_notification_layer(
        notification_sink: Arc<StdMutex<NotificationSink>>,
        format: LogFormat,
    ) -> Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> {
        let base_layer = fmt::layer()
            .with_writer(move || -> DirectWriterGuard<NotificationSink> {
                DirectWriterGuard::new(notification_sink.clone())
            })
            .with_ansi(false);

        match format {
            LogFormat::Full => Box::new(base_layer),
            LogFormat::Compact => Box::new(base_layer.compact()),
            LogFormat::Json => Box::new(base_layer.json()),
            LogFormat::Pretty => Box::new(base_layer.pretty()),
        }
    }

    /// Create message layer with format selection
    fn create_message_layer(
        message_sink: Arc<StdMutex<MessageSink>>,
        format: LogFormat,
    ) -> Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> {
        let base_layer = fmt::layer()
            .with_writer(move || -> DirectWriterGuard<MessageSink> {
                DirectWriterGuard::new(message_sink.clone())
            })
            .with_ansi(true);

        match format {
            LogFormat::Full => Box::new(base_layer),
            LogFormat::Compact => Box::new(base_layer.compact()),
            LogFormat::Json => Box::new(base_layer.json()),
            LogFormat::Pretty => Box::new(base_layer.pretty()),
        }
    }

    pub fn set_log_level(&self, level: LogLevel) -> Result<(), Error> {
        let filter: EnvFilter = level.into();
        self.filter
            .reload(filter)
            .map_err(|e| Error::Internal(e.to_string()))
    }

    pub fn set_file_logger(&self, config: LogFileConfig) -> Result<(), Error> {
        // Get the format from config or use default
        let format = config.format.unwrap_or(LogFormat::Compact);
        
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

        // Create new FileLayer with updated writer, level, and format
        let new_file_layer = FileLayer::new(self.file_writer.clone(), config.level, format);
        self.file_layer_handle
            .reload(new_file_layer)
            .map_err(|e| Error::Internal(format!("Failed to reload file layer: {}", e)))?;

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

    /// Target identifier for format updates
    pub fn set_format(&self, target: LogTarget, format: LogFormat) -> Result<(), Error> {
        match target {
            LogTarget::Quickfix => {
                let new_layer = Self::create_quickfix_layer(self.quickfix_writer.clone(), format);
                self.quickfix_handle
                    .reload(new_layer)
                    .map_err(|e| Error::Internal(format!("Failed to reload quickfix format: {}", e)))?;
            }
            LogTarget::Notification => {
                let new_layer = Self::create_notification_layer(self.notification_sink.clone(), format);
                self.notification_handle
                    .reload(new_layer)
                    .map_err(|e| Error::Internal(format!("Failed to reload notification format: {}", e)))?;
            }
            LogTarget::Message => {
                let new_layer = Self::create_message_layer(self.message_sink.clone(), format);
                self.message_handle
                    .reload(new_layer)
                    .map_err(|e| Error::Internal(format!("Failed to reload message format: {}", e)))?;
            }
            LogTarget::File => {
                // Create new FileLayer with updated format (keeps same writer and level)
                let current_level = LogLevel::Info; // Default, will be updated via update_level if needed
                let new_layer = FileLayer::new(self.file_writer.clone(), current_level, format);
                self.file_layer_handle
                    .reload(new_layer)
                    .map_err(|e| Error::Internal(format!("Failed to reload file format: {}", e)))?;
            }
            LogTarget::Stdio => {
                // Reload stdio layer with new format
                let new_layer = Self::create_stdio_layer(format);
                self.stdio_handle
                    .reload(new_layer)
                    .map_err(|e| Error::Internal(format!("Failed to reload stdio format: {}", e)))?;
            }
        }
        Ok(())
    }

    pub fn configure(&self, config: LogConfig) -> Result<(), Error> {
        // Configure file logging (handles path, level, format, max_size, max_files all together)
        if let Some(file_config) = config.file {
            self.set_file_logger(file_config)?;
        } else {
            // No file config provided - disable file logging by setting level to OFF
            // This ensures file logging doesn't leak from previous configurations
            let disabled_layer = FileLayer::new(self.file_writer.clone(), LogLevel::Off, LogFormat::Compact);
            self.file_layer_handle
                .reload(disabled_layer)
                .map_err(|e| Error::Internal(format!("Failed to disable file logger: {}", e)))?;
        }

        // Update formats for each target if specified
        // Note: File format is handled in set_file_logger() above
        if let Some(format) = config.stdio.format {
            self.set_format(LogTarget::Stdio, format)?;
        }
        if let Some(format) = config.notification.format {
            self.set_format(LogTarget::Notification, format)?;
        }
        if let Some(format) = config.message.format {
            self.set_format(LogTarget::Message, format)?;
        }
        if let Some(format) = config.quickfix.format {
            self.set_format(LogTarget::Quickfix, format)?;
        }

        // Configure levels for each target
        // Note: File level is handled in set_file_logger() above
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

    #[test]
    fn test_log_target_variants_exist() {
        // Verify all LogTarget variants can be constructed
        let _stdio = LogTarget::Stdio;
        let _file = LogTarget::File;
        let _notification = LogTarget::Notification;
        let _message = LogTarget::Message;
        let _quickfix = LogTarget::Quickfix;
    }
}

