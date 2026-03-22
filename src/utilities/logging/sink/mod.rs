//! Log sink implementations for different output destinations
//!
//! This module provides various `LogSink` implementations for sending
//! logs to different destinations: files, notifications, messages, and quickfix.

use std::io;

pub mod file;

/// Trait for log output destinations
///
/// Implementors of this trait can be used with `ChannelWriter` to send
/// log messages to various output destinations (files, UI, etc.).
pub trait LogSink: Send + 'static {
    /// Write a batch of log messages
    ///
    /// # Arguments
    /// * `messages` - Slice of log message strings to write
    fn write_batch(&mut self, messages: &[String]) -> io::Result<()>;

    /// Flush any pending writes
    fn flush(&mut self) -> io::Result<()>;
}

/// A sink that discards all log messages
///
/// Useful as a placeholder or for testing.
pub struct NullSink;

impl LogSink for NullSink {
    fn write_batch(&mut self, _messages: &[String]) -> io::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub use file::FileSink;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_sink() {
        let mut sink = NullSink;
        assert!(sink.write_batch(&["test".to_string()]).is_ok());
        assert!(sink.flush().is_ok());
    }
}
