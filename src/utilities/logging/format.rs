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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_log_format_display_pretty() {
        assert_eq!(format!("{}", LogFormat::Pretty), "pretty");
    }

    #[test]
    fn test_log_format_display_compact() {
        assert_eq!(format!("{}", LogFormat::Compact), "compact");
    }

    #[test]
    fn test_log_format_display_full() {
        assert_eq!(format!("{}", LogFormat::Full), "full");
    }

    #[test]
    fn test_log_format_display_json() {
        assert_eq!(format!("{}", LogFormat::Json), "json");
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

    #[test]
    fn test_log_format_from_str_invalid() {
        assert_eq!(LogFormat::from("invalid"), LogFormat::Pretty);
    }

    #[test]
    fn test_log_format_from_str_empty() {
        assert_eq!(LogFormat::from(""), LogFormat::Pretty);
    }

    #[test]
    fn test_log_format_from_string_compact() {
        assert_eq!(LogFormat::from("compact".to_string()), LogFormat::Compact);
    }

    #[test]
    fn test_log_format_from_string_json() {
        assert_eq!(LogFormat::from("json".to_string()), LogFormat::Json);
    }

    #[test]
    fn test_log_format_default_is_compact() {
        assert_eq!(LogFormat::default(), LogFormat::Compact);
    }
}
