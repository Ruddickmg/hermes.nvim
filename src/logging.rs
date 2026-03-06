use tracing_subscriber;

pub struct Logger {
    
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NvimLogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Off = 5,
}


impl From<NvimLogLevel> for tracing::Level {
    fn from(level: NvimLogLevel) -> Self {
        match level {
            NvimLogLevel::Trace => tracing.Level::TRACE,
            NvimLogLevel::Debug => tracing.Level::DEBUG,
            NvimLogLevel::Info => tracing.Level::INFO,
            NvimLogLevel::Warn => tracing.Level::WARN,
            NvimLogLevel::Error => tracing::Level::ERROR,
            NvimLogLevel::Off => tracing::Level::ERROR,
        }
    }
}

impl Logger {
    pub fn init() -> Result<(), nvim_oxi::api::Error> {
      let log_level: i64 = api::nvim_get_option_value("verbose", &opts!())?;
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .init();
    }
}
