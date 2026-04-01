use nvim_oxi::api::types::LogLevel as NvimLogLevel;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Default)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    #[default]
    Off = 5,
}

impl From<LogLevel> for EnvFilter {
    fn from(level: LogLevel) -> Self {
        let filter: LevelFilter = level.into();
        // Use only the default directive, don't read from environment
        // to ensure tests work consistently
        EnvFilter::new(filter.to_string())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // Display trait tests
    #[test]
    fn test_log_level_display_trace() {
        assert_eq!(format!("{}", LogLevel::Trace), "trace");
    }

    #[test]
    fn test_log_level_display_debug() {
        assert_eq!(format!("{}", LogLevel::Debug), "debug");
    }

    #[test]
    fn test_log_level_display_info() {
        assert_eq!(format!("{}", LogLevel::Info), "info");
    }

    #[test]
    fn test_log_level_display_warn() {
        assert_eq!(format!("{}", LogLevel::Warn), "warn");
    }

    #[test]
    fn test_log_level_display_error() {
        assert_eq!(format!("{}", LogLevel::Error), "error");
    }

    #[test]
    fn test_log_level_display_off() {
        assert_eq!(format!("{}", LogLevel::Off), "off");
    }

    // From<i64> tests
    #[test]
    fn test_log_level_from_i64_trace() {
        assert_eq!(LogLevel::from(0i64), LogLevel::Trace);
    }

    #[test]
    fn test_log_level_from_i64_debug() {
        assert_eq!(LogLevel::from(1i64), LogLevel::Debug);
    }

    #[test]
    fn test_log_level_from_i64_info() {
        assert_eq!(LogLevel::from(2i64), LogLevel::Info);
    }

    #[test]
    fn test_log_level_from_i64_warn() {
        assert_eq!(LogLevel::from(3i64), LogLevel::Warn);
    }

    #[test]
    fn test_log_level_from_i64_error() {
        assert_eq!(LogLevel::from(4i64), LogLevel::Error);
    }

    #[test]
    fn test_log_level_from_i64_off() {
        assert_eq!(LogLevel::from(5i64), LogLevel::Off);
    }

    #[test]
    fn test_log_level_from_i64_negative_is_off() {
        assert_eq!(LogLevel::from(-1i64), LogLevel::Off);
    }

    #[test]
    fn test_log_level_from_i64_large_is_off() {
        assert_eq!(LogLevel::from(100i64), LogLevel::Off);
    }

    // From<&str> tests
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
    fn test_log_level_from_str_off() {
        assert_eq!(LogLevel::from("off"), LogLevel::Off);
    }

    #[test]
    fn test_log_level_from_str_unknown() {
        assert_eq!(LogLevel::from("unknown"), LogLevel::Off);
    }

    #[test]
    fn test_log_level_from_str_invalid() {
        assert_eq!(LogLevel::from("invalid"), LogLevel::Off);
    }

    #[test]
    fn test_log_level_from_str_empty() {
        assert_eq!(LogLevel::from(""), LogLevel::Off);
    }

    // From<String> tests
    #[test]
    fn test_log_level_from_string_info() {
        assert_eq!(LogLevel::from("info".to_string()), LogLevel::Info);
    }

    #[test]
    fn test_log_level_from_string_error() {
        assert_eq!(LogLevel::from("error".to_string()), LogLevel::Error);
    }

    // Default test
    #[test]
    fn test_log_level_default_is_off() {
        assert_eq!(LogLevel::default(), LogLevel::Off);
    }

    // From<LogLevel> for LevelFilter tests
    #[test]
    fn test_log_level_into_level_filter_trace() {
        let filter: LevelFilter = LogLevel::Trace.into();
        assert_eq!(filter, LevelFilter::TRACE);
    }

    #[test]
    fn test_log_level_into_level_filter_debug() {
        let filter: LevelFilter = LogLevel::Debug.into();
        assert_eq!(filter, LevelFilter::DEBUG);
    }

    #[test]
    fn test_log_level_into_level_filter_info() {
        let filter: LevelFilter = LogLevel::Info.into();
        assert_eq!(filter, LevelFilter::INFO);
    }

    #[test]
    fn test_log_level_into_level_filter_warn() {
        let filter: LevelFilter = LogLevel::Warn.into();
        assert_eq!(filter, LevelFilter::WARN);
    }

    #[test]
    fn test_log_level_into_level_filter_error() {
        let filter: LevelFilter = LogLevel::Error.into();
        assert_eq!(filter, LevelFilter::ERROR);
    }

    #[test]
    fn test_log_level_into_level_filter_off() {
        let filter: LevelFilter = LogLevel::Off.into();
        assert_eq!(filter, LevelFilter::OFF);
    }

    // From<LogLevel> for tracing::Level tests
    #[test]
    fn test_log_level_into_tracing_level_trace() {
        let level: tracing::Level = LogLevel::Trace.into();
        assert_eq!(level, tracing::Level::TRACE);
    }

    #[test]
    fn test_log_level_into_tracing_level_debug() {
        let level: tracing::Level = LogLevel::Debug.into();
        assert_eq!(level, tracing::Level::DEBUG);
    }

    #[test]
    fn test_log_level_into_tracing_level_info() {
        let level: tracing::Level = LogLevel::Info.into();
        assert_eq!(level, tracing::Level::INFO);
    }

    #[test]
    fn test_log_level_into_tracing_level_warn() {
        let level: tracing::Level = LogLevel::Warn.into();
        assert_eq!(level, tracing::Level::WARN);
    }

    #[test]
    fn test_log_level_into_tracing_level_error() {
        let level: tracing::Level = LogLevel::Error.into();
        assert_eq!(level, tracing::Level::ERROR);
    }

    #[test]
    fn test_log_level_into_tracing_level_off() {
        let level: tracing::Level = LogLevel::Off.into();
        assert_eq!(level, tracing::Level::ERROR);
    }

    // From<LogLevel> for EnvFilter tests
    #[test]
    fn test_log_level_into_env_filter_trace() {
        let filter: EnvFilter = LogLevel::Trace.into();
        let filter_str = filter.to_string();
        assert!(filter_str.contains("trace"));
    }

    #[test]
    fn test_log_level_into_env_filter_info() {
        let filter: EnvFilter = LogLevel::Info.into();
        let filter_str = filter.to_string();
        assert!(filter_str.contains("info"));
    }

    #[test]
    fn test_log_level_into_env_filter_off() {
        let filter: EnvFilter = LogLevel::Off.into();
        let filter_str = filter.to_string();
        assert!(filter_str.contains("off"));
    }

    // From<LogLevel> for NvimLogLevel tests
    #[test]
    fn test_log_level_into_nvim_log_level_trace() {
        let level: NvimLogLevel = LogLevel::Trace.into();
        assert_eq!(level, NvimLogLevel::Trace);
    }

    #[test]
    fn test_log_level_into_nvim_log_level_debug() {
        let level: NvimLogLevel = LogLevel::Debug.into();
        assert_eq!(level, NvimLogLevel::Debug);
    }

    #[test]
    fn test_log_level_into_nvim_log_level_info() {
        let level: NvimLogLevel = LogLevel::Info.into();
        assert_eq!(level, NvimLogLevel::Info);
    }

    #[test]
    fn test_log_level_into_nvim_log_level_warn() {
        let level: NvimLogLevel = LogLevel::Warn.into();
        assert_eq!(level, NvimLogLevel::Warn);
    }

    #[test]
    fn test_log_level_into_nvim_log_level_error() {
        let level: NvimLogLevel = LogLevel::Error.into();
        assert_eq!(level, NvimLogLevel::Error);
    }

    #[test]
    fn test_log_level_into_nvim_log_level_off() {
        let level: NvimLogLevel = LogLevel::Off.into();
        assert_eq!(level, NvimLogLevel::Off);
    }
}
