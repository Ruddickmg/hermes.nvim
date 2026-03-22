use nvim_oxi::api;
use nvim_oxi::api::opts::OptionOpts;
use std::sync::Mutex as StdMutex;
use std::sync::{Arc, OnceLock};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::filter::{self, Filtered};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::reload::Layer;
use tracing_subscriber::{
    EnvFilter, Registry, fmt,
    prelude::*,
    reload::{self, Handle},
};

use crate::nvim::configuration::LogTargetConfig;
use crate::{
    acp::{Result, error::Error},
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

/// Type alias for the holder resources returned by create_holders()
type LoggerHolders = (
    Arc<StdMutex<Option<FileChannel>>>,
    Arc<StdMutex<Option<QuickfixChannel>>>,
    Arc<StdMutex<NotificationSink>>,
    Arc<StdMutex<MessageSink>>,
);

/// Type aliases for layer reload handles
type FormatLayerHandle =
    Handle<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>, Registry>;
type FileLayerHandle = Handle<FileLayer, Registry>;
type StdioLayerHandle = Handle<StdioLayer, Registry>;
type QuickfixLayerHandle = Handle<QuickfixLayer, Registry>;
type NotificationLayerHandle = Handle<NotificationLayer, Registry>;
type MessageLayerHandle = Handle<MessageLayer, Registry>;

/// Custom stdio layer that implements its own filtering
/// This bypasses the Filtered layer bug in tracing-subscriber issue #1629
#[derive(Debug)]
pub struct StdioLayer {
    level: Arc<StdMutex<LogLevel>>,
    format: Arc<StdMutex<LogFormat>>,
}

impl StdioLayer {
    pub fn new(level: LogLevel, format: LogFormat) -> Self {
        Self {
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

impl<S> tracing_subscriber::Layer<S> for StdioLayer
where
    S: tracing::Subscriber,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        let level = self.level.lock().unwrap();
        let event_level = metadata.level();

        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };

        // Only enable if event is at or above min_level (less verbose or equal)
        event_num >= min_num
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Check level again (enabled() is just a hint)
        let level = self.level.lock().unwrap();
        let event_level = event.metadata().level();
        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        if event_num < min_num {
            return; // Filter it out
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
                    r#"{{"timestamp":"","level":"{:?}","target":"{}","fields":{{"message":"{}}}}}"#,
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
                format!(
                    "{}\n  level: {}\n  target: {}\n  message: {}\n",
                    "Event:", level_str, target, message_text
                )
            }
        };

        // Write to stdout
        print!("{}", formatted);
    }
}

/// Custom quickfix layer that implements its own filtering
/// This bypasses the Filtered layer bug in tracing-subscriber issue #1629
#[derive(Debug)]
pub struct QuickfixLayer {
    writer: Arc<StdMutex<Option<QuickfixChannel>>>,
    level: Arc<StdMutex<LogLevel>>,
    format: Arc<StdMutex<LogFormat>>,
}

impl QuickfixLayer {
    pub fn new(
        writer: Arc<StdMutex<Option<QuickfixChannel>>>,
        level: LogLevel,
        format: LogFormat,
    ) -> Self {
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

impl<S> tracing_subscriber::Layer<S> for QuickfixLayer
where
    S: tracing::Subscriber,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        let level = self.level.lock().unwrap();
        let event_level = metadata.level();

        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };

        // Only enable if event is at or above min_level (less verbose or equal)
        event_num >= min_num
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Check level again (enabled() is just a hint)
        let level = self.level.lock().unwrap();
        let event_level = event.metadata().level();
        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
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
                    r#"{{"timestamp":"","level":"{:?}","target":"{}","fields":{{"message":"{}}}}}"#,
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
                format!(
                    "{}\n  level: {}\n  target: {}\n  message: {}\n",
                    "Event:", level_str, target, message_text
                )
            }
        };

        // Write to the quickfix channel
        if let Some(ref mut writer) = *writer_guard {
            let _ = std::io::Write::write_all(writer, formatted.as_bytes());
        }
    }
}

/// Custom notification layer that implements its own filtering
/// This bypasses the Filtered layer bug in tracing-subscriber issue #1629
#[derive(Debug)]
pub struct NotificationLayer {
    level: Arc<StdMutex<LogLevel>>,
    format: Arc<StdMutex<LogFormat>>,
}

impl NotificationLayer {
    pub fn new(level: LogLevel, format: LogFormat) -> Self {
        Self {
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

impl<S> tracing_subscriber::Layer<S> for NotificationLayer
where
    S: tracing::Subscriber,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        let level = self.level.lock().unwrap();
        let event_level = metadata.level();

        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };

        // Only enable if event is at or above min_level (less verbose or equal)
        event_num >= min_num
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Check level again (enabled() is just a hint)
        let level = self.level.lock().unwrap();
        let event_level = event.metadata().level();
        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        if event_num < min_num {
            return; // Filter it out
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
                    r#"{{"timestamp":"","level":"{:?}","target":"{}","fields":{{"message":"{}}}}}"#,
                    level_str, target, message_text
                )
            }
            LogFormat::Full => {
                format!("{} {}: {}", level_str, target, message_text)
            }
            LogFormat::Compact => {
                format!("[{}] {}", level_str, message_text)
            }
            LogFormat::Pretty => {
                format!(
                    "{}\n  level: {}\n  target: {}\n  message: {}",
                    "Event:", level_str, target, message_text
                )
            }
        };

        // Convert to nvim log level
        let nvim_level = match *level_str {
            tracing::Level::ERROR => nvim_oxi::api::types::LogLevel::Error,
            tracing::Level::WARN => nvim_oxi::api::types::LogLevel::Warn,
            tracing::Level::INFO => nvim_oxi::api::types::LogLevel::Info,
            tracing::Level::DEBUG => nvim_oxi::api::types::LogLevel::Debug,
            tracing::Level::TRACE => nvim_oxi::api::types::LogLevel::Trace,
        };

        // Create opts with title
        let mut opts = nvim_oxi::Dictionary::new();
        opts.insert("title".to_string(), nvim_oxi::Object::from("Hermes"));

        // Send notification - ignore errors to avoid crashing
        let _ = nvim_oxi::api::notify(&formatted, nvim_level, &opts);
    }
}

/// Custom message layer that implements its own filtering
/// This bypasses the Filtered layer bug in tracing-subscriber issue #1629
#[derive(Debug)]
pub struct MessageLayer {
    level: Arc<StdMutex<LogLevel>>,
    format: Arc<StdMutex<LogFormat>>,
}

impl MessageLayer {
    pub fn new(level: LogLevel, format: LogFormat) -> Self {
        Self {
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

impl<S> tracing_subscriber::Layer<S> for MessageLayer
where
    S: tracing::Subscriber,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        let level = self.level.lock().unwrap();
        let event_level = metadata.level();

        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };

        // Only enable if event is at or above min_level (less verbose or equal)
        event_num >= min_num
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Check level again (enabled() is just a hint)
        let level = self.level.lock().unwrap();
        let event_level = event.metadata().level();
        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        if event_num < min_num {
            return; // Filter it out
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
                    r#"{{"timestamp":"","level":"{:?}","target":"{}","fields":{{"message":"{}}}}}"#,
                    level_str, target, message_text
                )
            }
            LogFormat::Full => {
                format!("{} {}: {}", level_str, target, message_text)
            }
            LogFormat::Compact => {
                format!("[{}] {}", level_str, message_text)
            }
            LogFormat::Pretty => {
                format!(
                    "{}\n  level: {}\n  target: {}\n  message: {}",
                    "Event:", level_str, target, message_text
                )
            }
        };

        // Send to message history using out_write
        // Add newline for proper message formatting
        let msg = format!("{}\n", formatted);
        nvim_oxi::api::out_write(msg);
    }
}

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
    pub fn new(
        writer: Arc<StdMutex<Option<FileChannel>>>,
        level: LogLevel,
        format: LogFormat,
    ) -> Self {
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
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        let level = self.level.lock().unwrap();
        let event_level = metadata.level();

        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };

        // Only enable if event is at or above min_level (less verbose or equal)
        event_num >= min_num
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Check level again (enabled() is just a hint)
        let level = self.level.lock().unwrap();
        let event_level = event.metadata().level();
        use tracing::Level;
        let event_num = match *event_level {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
        };
        let min_num = match (*level).into() {
            Level::TRACE => 0,
            Level::DEBUG => 1,
            Level::INFO => 2,
            Level::WARN => 3,
            Level::ERROR => 4,
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
                format!(
                    "{}\n  level: {}\n  target: {}\n  message: {}\n",
                    "Event:", level_str, target, message_text
                )
            }
        };

        // Write to the file channel
        if let Some(ref mut writer) = *writer_guard {
            let _ = std::io::Write::write_all(writer, formatted.as_bytes());
        }
    }
}

type BoxedLayer = Box<dyn tracing_subscriber::layer::Layer<Registry> + Send + Sync + 'static>;
type CombinedLayer = Filtered<BoxedLayer, EnvFilter, Registry>;
type CombinedHandle = Handle<Filtered<BoxedLayer, EnvFilter, Registry>, Registry>;

/// Logger that supports multiple output targets
pub struct Logger {
    stdio_handle: Handle<Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync>, Registry>,
}

impl Logger {
    pub fn filter_layer(level: LogLevel) -> EnvFilter {
        let log_level: EnvFilter = level.into();
        log_level
    }

    pub fn format_layer(format: LogFormat) -> BoxedLayer {
        let base = fmt::layer()
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

    pub fn combine_layers(config: LogTargetConfig) -> BoxedLayer {
        let filter = Self::filter_layer(config.level);
        let format = Self::format_layer(config.format);
        format.with_filter(filter).boxed()
    }

    pub fn inititalize() -> &'static Self {
        let configuration = LogConfig::default();
        let stdio_combined = Self::combine_layers(configuration.stdio);
        // let notification_combined = Self::combine_layers(configuration.notification);
        let (stdio_layer, stdio_handle) = reload::Layer::new(stdio_combined);
        // let (notification_layer, _notification_handle) = reload::Layer::new(notification_combined);

        let registry = tracing_subscriber::registry().with(stdio_layer);
        // .with(notification_layer);

        LOGGER.get_or_init(|| {
            registry.init();
            Self { stdio_handle }
        })
    }

    pub fn configure(&self, config: LogConfig) -> Result<()> {
        self.stdio_handle
            .reload(Self::combine_layers(config.stdio))
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
