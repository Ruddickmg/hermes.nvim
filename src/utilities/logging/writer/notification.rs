use std::io::{self, Write};
use tracing_subscriber::fmt::writer::MakeWriter;

use crate::utilities::{LogLevel, NotificationMessenger};

/// A writer that sends lines to Neovim notifications
/// Uses NotificationMessenger to safely deliver notifications on the main thread
#[derive(Debug, Clone)]
pub struct NotifyWriter {
    level: LogLevel,
    messenger: NotificationMessenger,
}

impl NotifyWriter {
    pub fn new(level: LogLevel, messenger: NotificationMessenger) -> Self {
        Self { level, messenger }
    }
}

impl Write for NotifyWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Convert bytes to string (ignore invalid UTF-8)
        let s = String::from_utf8_lossy(buf);

        // Skip empty strings
        if s.trim().is_empty() {
            return Ok(buf.len());
        }

        let escaped = s.replace('"', "\\\"");

        // Send notification via messenger (thread-safe, delivers on main thread)
        self.messenger.send(escaped, self.level).ok();

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for NotifyWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_writer_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<NotifyWriter>();
    }

    #[test]
    fn test_notify_writer_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<NotifyWriter>();
    }
}
