use nvim_oxi::api::{self, opts::OptionOpts};
use std::sync::OnceLock;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter, Registry, fmt,
    prelude::*,
    reload::{self, Handle},
};

use crate::acp::error::Error;
static LOGGER: OnceLock<Logger> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Off = 5,
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

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Compact,
    Full,
    Json,
}

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Pretty
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
        let (filter_layer, filter) = reload::Layer::new(log_level);
        let base = fmt::layer()
            .with_ansi(true)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true);
        let registry = tracing_subscriber::registry().with(filter_layer);

        LOGGER.get_or_init(|| {
            match format {
                LogFormat::Full => registry.with(base).init(),
                LogFormat::Compact => registry.with(base.compact()).init(),
                LogFormat::Json => registry.with(base.json()).init(),
                _ => registry.with(base.pretty()).init(),
            }
            Self { filter }
        })
    }

    pub fn set_log_level(&self, level: LogLevel) -> Result<(), Error> {
        let filter: EnvFilter = level.into();
        self.filter
            .reload(filter)
            .map_err(|e| Error::Internal(e.to_string()))
    }
}
