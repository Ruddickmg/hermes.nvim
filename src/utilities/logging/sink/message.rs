//! Message log sink for sending logs to Neovim message history
//!
//! This module provides a `LogSink` implementation that sends log messages
//! to Neovim's `:messages` history using `nvim_echo()`.

use std::io;

use nvim_oxi::api;

use crate::utilities::logging::sink::LogSink;

/// A message-based log sink
///
/// Sends log messages to Neovim's message history using `nvim_echo()`.
/// Messages appear in `:messages` and can be viewed with `:mes`.
pub struct MessageSink;

impl MessageSink {
    /// Create a new message sink
    pub fn new() -> Self {
        Self
    }

    /// Extract log level from message prefix
    fn extract_level(message: &str) -> &str {
        if message.starts_with("[ERROR]") {
            "ERROR"
        } else if message.starts_with("[WARN]") {
            "WARN"
        } else if message.starts_with("[INFO]") {
            "INFO"
        } else if message.starts_with("[DEBUG]") {
            "DEBUG"
        } else if message.starts_with("[TRACE]") {
            "TRACE"
        } else {
            "INFO"
        }
    }

    /// Get highlight group for log level
    fn get_highlight_group(level: &str) -> &'static str {
        match level {
            "ERROR" => "ErrorMsg",
            "WARN" => "WarningMsg",
            "INFO" => "MoreMsg",
            "DEBUG" => "Comment",
            "TRACE" => "Comment",
            _ => "Normal",
        }
    }
}

impl LogSink for MessageSink {
    fn write_batch(&mut self, messages: &[String]) -> io::Result<()> {
        for message in messages {
            let level = Self::extract_level(message);
            let _hl_group = Self::get_highlight_group(level);

            // Send to message history using out_write
            // Add newline for proper message formatting
            let msg = format!("{}\n", message);

            // Use out_write for message history
            api::out_write(msg);
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Messages are sent immediately, no buffering needed
        Ok(())
    }
}

impl Default for MessageSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_level() {
        assert_eq!(
            MessageSink::extract_level("[ERROR] Something failed"),
            "ERROR"
        );
        assert_eq!(MessageSink::extract_level("[WARN] Warning message"), "WARN");
        assert_eq!(MessageSink::extract_level("[INFO] Info message"), "INFO");
        assert_eq!(MessageSink::extract_level("[DEBUG] Debug info"), "DEBUG");
        assert_eq!(MessageSink::extract_level("[TRACE] Trace details"), "TRACE");
        assert_eq!(MessageSink::extract_level("Plain message"), "INFO");
    }

    #[test]
    fn test_get_highlight_group() {
        assert_eq!(MessageSink::get_highlight_group("ERROR"), "ErrorMsg");
        assert_eq!(MessageSink::get_highlight_group("WARN"), "WarningMsg");
        assert_eq!(MessageSink::get_highlight_group("INFO"), "MoreMsg");
        assert_eq!(MessageSink::get_highlight_group("DEBUG"), "Comment");
        assert_eq!(MessageSink::get_highlight_group("TRACE"), "Comment");
        assert_eq!(MessageSink::get_highlight_group("UNKNOWN"), "Normal");
    }
}
