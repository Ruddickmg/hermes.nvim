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
    fn test_log_level_from_i64_trace() {
        assert_eq!(LogLevel::from(0), LogLevel::Trace);
    }

    #[test]
    fn test_log_level_from_i64_debug() {
        assert_eq!(LogLevel::from(1), LogLevel::Debug);
    }

    #[test]
    fn test_log_level_from_i64_info() {
        assert_eq!(LogLevel::from(2), LogLevel::Info);
    }

    #[test]
    fn test_log_level_from_i64_warn() {
        assert_eq!(LogLevel::from(3), LogLevel::Warn);
    }

    #[test]
    fn test_log_level_from_i64_error() {
        assert_eq!(LogLevel::from(4), LogLevel::Error);
    }

    #[test]
    fn test_log_level_from_i64_off() {
        assert_eq!(LogLevel::from(5), LogLevel::Off);
    }

    #[test]
    fn test_log_level_from_i64_unknown() {
        assert_eq!(LogLevel::from(99), LogLevel::Off);
    }

    #[test]
    fn test_log_level_from_str_trace() {
        assert_eq!(LogLevel::from("trace"), LogLevel::Trace);
    }

    #[test]
    fn test_log_level_from_str_debug() {
        assert_eq!(LogLevel::from("debug"), LogLevel::Debug);
    }

    #[test]
    fn test_log_level_from_str_info() {
        assert_eq!(LogLevel::from("info"), LogLevel::Info);
    }

    #[test]
    fn test_log_level_from_str_warn() {
        assert_eq!(LogLevel::from("warn"), LogLevel::Warn);
    }

    #[test]
    fn test_log_level_from_str_error() {
        assert_eq!(LogLevel::from("error"), LogLevel::Error);
    }

    #[test]
    fn test_log_level_from_str_unknown() {
        assert_eq!(LogLevel::from("unknown"), LogLevel::Off);
    }

    #[test]
    fn test_log_level_into_level_filter_trace() {
        let filter: LevelFilter = LogLevel::Trace.into();
        assert_eq!(filter, LevelFilter::TRACE);
    }

    #[test]
    fn test_log_level_into_level_filter_off() {
        let filter: LevelFilter = LogLevel::Off.into();
        assert_eq!(filter, LevelFilter::OFF);
    }

    #[test]
    fn test_log_format_from_str_pretty() {
        assert_eq!(LogFormat::from("pretty"), LogFormat::Pretty);
    }

    #[test]
    fn test_log_format_from_str_compact() {
        assert_eq!(LogFormat::from("compact"), LogFormat::Compact);
    }

    #[test]
    fn test_log_format_from_str_full() {
        assert_eq!(LogFormat::from("full"), LogFormat::Full);
    }

    #[test]
    fn test_log_format_from_str_json() {
        assert_eq!(LogFormat::from("json"), LogFormat::Json);
    }

    #[test]
    fn test_log_format_from_str_unknown() {
        assert_eq!(LogFormat::from("unknown"), LogFormat::Pretty);
    }
}
