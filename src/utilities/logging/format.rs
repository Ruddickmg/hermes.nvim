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



