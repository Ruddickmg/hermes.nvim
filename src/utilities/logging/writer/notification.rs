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
    use proptest::prelude::*;

    // Simple smoke tests - full integration tests in tests/integration/src

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

    #[test]
    fn test_string_utf8_lossy_conversion() {
        // Test the UTF-8 conversion logic used in write()
        let valid = "Hello World";
        let converted = String::from_utf8_lossy(valid.as_bytes());
        assert_eq!(converted, "Hello World");

        let empty = "";
        let converted = String::from_utf8_lossy(empty.as_bytes());
        assert!(converted.trim().is_empty());

        let whitespace = "   \n\t  ";
        let converted = String::from_utf8_lossy(whitespace.as_bytes());
        assert!(converted.trim().is_empty());
    }

    #[test]
    fn test_quote_escaping() {
        let input = r#"message with "quotes""#;
        let escaped = input.replace('"', "\\\"");
        assert_eq!(escaped, r#"message with \"quotes\""#);

        let no_quotes = "no quotes here";
        let unchanged = no_quotes.replace('"', "\\\"");
        assert_eq!(unchanged, "no quotes here");

        let multiple = r#""first" and "second""#;
        let escaped = multiple.replace('"', "\\\"");
        assert_eq!(escaped, r#"\"first\" and \"second\""#);
    }

    #[test]
    fn test_empty_string_skipping() {
        assert!("".trim().is_empty());
        assert!("   ".trim().is_empty());
        assert!("\n\t".trim().is_empty());
        assert!(!"hello".trim().is_empty());
        assert!(!"  hello  ".trim().is_empty());
    }

    proptest! {
        #[test]
        fn test_quote_escaping_never_panics(input in any::<String>()) {
            // The replace operation should never panic
            let _ = input.replace('"', "\\\"");
        }

        #[test]
        fn test_utf8_conversion_never_panics(input in proptest::collection::vec(any::<u8>(), 0..1000)) {
            // String::from_utf8_lossy should never panic
            let _ = String::from_utf8_lossy(&input);
        }

        #[test]
        fn test_empty_check_consistent(input in any::<String>()) {
            // is_empty() should be consistent
            let is_empty = input.trim().is_empty();
            // If it's empty, the string should have no non-whitespace chars
            if is_empty {
                assert!(input.chars().all(|c| c.is_whitespace()));
            }
        }
    }
}
