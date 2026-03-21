//! Notification log sink for sending logs to Neovim notifications
//!
//! This module provides a `LogSink` implementation that sends log messages
//! to Neovim via `vim.notify()`, displayed as notifications to the user.

use std::io;

use nvim_oxi::api::{self, types::LogLevel as NvimLogLevel};
use nvim_oxi::Dictionary;

use crate::utilities::logging::sink::LogSink;

/// A notification-based log sink
///
/// Sends log messages to Neovim's notification system using `vim.notify()`.
/// Messages are displayed as pop-up notifications in the UI.
pub struct NotificationSink;

impl NotificationSink {
    /// Create a new notification sink
    pub fn new() -> Self {
        Self
    }

    /// Convert Hermes LogLevel to nvim-oxi LogLevel
    fn convert_level(level: &str) -> NvimLogLevel {
        // Parse the level from the message prefix like "[ERROR]" or "[INFO]"
        if level.contains("ERROR") {
            NvimLogLevel::Error
        } else if level.contains("WARN") {
            NvimLogLevel::Warn
        } else if level.contains("INFO") {
            NvimLogLevel::Info
        } else if level.contains("DEBUG") {
            NvimLogLevel::Debug
        } else if level.contains("TRACE") {
            NvimLogLevel::Trace
        } else {
            NvimLogLevel::Info
        }
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

    /// Format message for notification (compact format)
    fn format_message(message: &str) -> String {
        // Remove the level prefix since notify handles that visually
        let level = Self::extract_level(message);
        let prefix = format!("[{}] ", level);

        if message.starts_with(&prefix) {
            message[prefix.len()..].to_string()
        } else {
            message.to_string()
        }
    }
}

impl LogSink for NotificationSink {
    fn write_batch(&mut self, messages: &[String]) -> io::Result<()> {
        for message in messages {
            let level_str = Self::extract_level(message);
            let nvim_level = Self::convert_level(level_str);
            let formatted = Self::format_message(message);

            // Create opts with title using Dictionary
            let mut opts = Dictionary::new();
            opts.insert("title".to_string(), nvim_oxi::Object::from("Hermes"));

            // Send notification - ignore errors to avoid crashing
            if let Err(e) = api::notify(&formatted, nvim_level, &opts) {
                eprintln!("Failed to send notification: {}", e);
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Notifications are sent immediately, no buffering needed
        Ok(())
    }
}

impl Default for NotificationSink {
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
            NotificationSink::extract_level("[ERROR] Something failed"),
            "ERROR"
        );
        assert_eq!(
            NotificationSink::extract_level("[WARN] Warning message"),
            "WARN"
        );
        assert_eq!(
            NotificationSink::extract_level("[INFO] Info message"),
            "INFO"
        );
        assert_eq!(
            NotificationSink::extract_level("[DEBUG] Debug info"),
            "DEBUG"
        );
        assert_eq!(
            NotificationSink::extract_level("[TRACE] Trace details"),
            "TRACE"
        );
        assert_eq!(NotificationSink::extract_level("Plain message"), "INFO");
    }

    #[test]
    fn test_format_message() {
        assert_eq!(
            NotificationSink::format_message("[ERROR] Something failed"),
            "Something failed"
        );
        assert_eq!(
            NotificationSink::format_message("[INFO] Test message"),
            "Test message"
        );
        assert_eq!(
            NotificationSink::format_message("Plain message"),
            "Plain message"
        );
    }

    #[test]
    fn test_convert_level() {
        assert!(matches!(
            NotificationSink::convert_level("ERROR"),
            NvimLogLevel::Error
        ));
        assert!(matches!(
            NotificationSink::convert_level("WARN"),
            NvimLogLevel::Warn
        ));
        assert!(matches!(
            NotificationSink::convert_level("INFO"),
            NvimLogLevel::Info
        ));
        assert!(matches!(
            NotificationSink::convert_level("DEBUG"),
            NvimLogLevel::Debug
        ));
        assert!(matches!(
            NotificationSink::convert_level("TRACE"),
            NvimLogLevel::Trace
        ));
    }
}
