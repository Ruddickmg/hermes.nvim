use nvim_oxi::{api, Dictionary};
use std::io::{self, Write};

use crate::utilities::LogLevel;

/// A writer that sends lines to Neovim notifications
#[derive(Debug, Clone)]
pub struct NotifyWriter {
    level: LogLevel,
    config: Dictionary,
}

// SAFETY: NotifyWriter contains Dictionary which has raw pointers, but we
// only access it through the Mutex, ensuring thread safety
unsafe impl Send for NotifyWriter {}
unsafe impl Sync for NotifyWriter {}

impl NotifyWriter {
    pub fn new(level: LogLevel) -> Self {
        let mut config = Dictionary::new();
        config.insert("title", "Hermes");
        config.insert("merge", true);
        Self { level, config }
    }
}

impl Write for NotifyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Convert bytes to string (ignore invalid UTF-8)
        let s = String::from_utf8_lossy(buf);

        // Split into lines (tracing fmt layer may send partial lines)
        for line in s.lines() {
            // Send each line to nvim notify
            let _ = api::notify(line, self.level.into(), &self.config);
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
