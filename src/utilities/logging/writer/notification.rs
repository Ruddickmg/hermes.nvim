use nvim_oxi::{Dictionary, api};
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

        let escaped = s.replace('"', "\\\"");
        api::notify(&escaped, self.level.into(), &self.config)
            .map_err(|e| std::io::Error::other(format!("Failed to send notification: {e}")))?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
