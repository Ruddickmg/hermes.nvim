//! File-based log sink for writing logs to disk
//!
//! This module provides a `LogSink` implementation that writes log messages
//! to files with rotation support.

use std::io::{self, Write};

use crate::utilities::logging::file::SizeBasedFileAppender;
use crate::utilities::logging::sink::LogSink;

/// A file-based log sink
///
/// Writes log messages to a file with size-based rotation.
pub struct FileSink {
    appender: SizeBasedFileAppender,
}

impl FileSink {
    /// Create a new file sink
    ///
    /// # Arguments
    /// * `path` - Base path for the log file
    /// * `max_size` - Maximum size in bytes before rotation
    /// * `max_files` - Maximum number of backup files to keep
    pub fn new(
        path: impl AsRef<std::path::Path>,
        max_size: u64,
        max_files: usize,
    ) -> io::Result<Self> {
        let appender = SizeBasedFileAppender::new(path, max_size, max_files)?;
        Ok(Self { appender })
    }
}

impl LogSink for FileSink {
    fn write_batch(&mut self, messages: &[String]) -> io::Result<()> {
        for message in messages {
            self.appender.write_all(message.as_bytes())?;
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.appender.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_sink_write_batch() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut sink = FileSink::new(&log_path, 1024 * 1024, 5).unwrap();

        let messages = vec!["Message 1\n".to_string(), "Message 2\n".to_string()];

        sink.write_batch(&messages).unwrap();
        sink.flush().unwrap();

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Message 1"));
        assert!(content.contains("Message 2"));
    }

    #[test]
    fn test_file_sink_empty_batch() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut sink = FileSink::new(&log_path, 1024 * 1024, 5).unwrap();

        sink.write_batch(&[]).unwrap();
        sink.flush().unwrap();

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.is_empty());
    }
}
