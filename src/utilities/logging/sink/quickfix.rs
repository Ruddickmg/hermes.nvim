//! Quickfix log sink for sending logs to Neovim quickfix list
//!
//! This module provides a `LogSink` implementation that sends log messages
//! to Neovim's quickfix list using `setqflist()`, enabling file navigation.

use std::io;

use nvim_oxi::api;
use nvim_oxi::{Array, Dictionary, Integer, Object};

use crate::utilities::logging::sink::LogSink;

/// A quickfix-based log sink
///
/// Sends log messages to Neovim's quickfix list using `setqflist()`.
/// This enables users to navigate to log source locations.
pub struct QuickfixSink {
    /// Quickfix list ID for appending updates
    qf_id: Option<u64>,
    /// Title for the quickfix list
    title: String,
}

impl QuickfixSink {
    /// Create a new quickfix sink
    pub fn new() -> Self {
        Self {
            qf_id: None,
            title: "Hermes Logs".to_string(),
        }
    }

    /// Extract log level from message and return type character
    fn get_error_type(message: &str) -> &'static str {
        if message.contains("ERROR") {
            "E"
        } else if message.contains("WARN") {
            "W"
        } else if message.contains("INFO") {
            "I"
        } else {
            "N" // Note for debug, trace, or unknown
        }
    }

    /// Parse a log message to extract file, line, and content
    ///
    /// Expected format: "[LEVEL] file.rs:123: message" or just "[LEVEL] message"
    fn parse_message(message: &str) -> (String, i64, i64, String, &'static str) {
        let error_type = Self::get_error_type(message);

        // Default values
        let mut filename = "hermes".to_string();
        let mut lnum = 1;
        let mut col = 1;
        let mut text = message.to_string();

        // Try to parse out file:line:col from the message
        // Format examples:
        // "[INFO] src/main.rs:42:10: Starting application"
        // "[ERROR] src/lib.rs:123: Something failed"

        // Find the content after the level prefix
        let content_start = if message.starts_with('[') {
            if let Some(end) = message.find("] ") {
                end + 2
            } else {
                0
            }
        } else {
            0
        };

        if content_start > 0 && content_start < message.len() {
            let content = &message[content_start..];

            // Look for file:line:col pattern
            if let Some(colon_pos) = content.find(':') {
                let potential_file = &content[..colon_pos];

                // Check if this looks like a file path (contains / or .)
                if potential_file.contains('/')
                    || potential_file.contains('.')
                    || potential_file.contains("\\")
                {
                    filename = potential_file.to_string();

                    // Try to extract line number
                    let after_file = &content[colon_pos + 1..];
                    if let Some(line_end) = after_file.find(':') {
                        if let Ok(line) = after_file[..line_end].parse::<i64>() {
                            lnum = line;

                            // Try to extract column (optional)
                            let after_line = &after_file[line_end + 1..];
                            if let Some(col_end) = after_line.find(':') {
                                if let Ok(c) = after_line[..col_end].parse::<i64>() {
                                    col = c;
                                    text = after_line[col_end + 1..].trim().to_string();
                                } else {
                                    // No column, just line
                                    text = after_line.trim().to_string();
                                }
                            } else {
                                text = after_line.trim().to_string();
                            }
                        } else {
                            // Line number not a valid int, treat rest as text
                            text = after_file.trim().to_string();
                        }
                    } else {
                        text = after_file.trim().to_string();
                    }
                } else {
                    // No file path detected, use entire content as text
                    text = content.trim().to_string();
                }
            } else {
                // No colon found, use entire content as text
                text = content.trim().to_string();
            }
        }

        (filename, lnum, col, text, error_type)
    }

    /// Create a quickfix item from parsed message data
    fn create_qf_item(
        filename: &str,
        lnum: i64,
        col: i64,
        text: &str,
        error_type: &str,
    ) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.insert("filename".to_string(), Object::from(filename.to_string()));
        dict.insert("lnum".to_string(), Object::from(lnum as Integer));
        dict.insert("col".to_string(), Object::from(col as Integer));
        dict.insert("text".to_string(), Object::from(text.to_string()));
        dict.insert("type".to_string(), Object::from(error_type.to_string()));
        dict
    }
}

impl LogSink for QuickfixSink {
    fn write_batch(&mut self, messages: &[String]) -> io::Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        // Parse all messages into quickfix items
        let mut items = Vec::new();
        for message in messages {
            let (filename, lnum, col, text, error_type) = Self::parse_message(message);
            let item = Self::create_qf_item(&filename, lnum, col, &text, error_type);
            items.push(Object::from(item));
        }

        // Build the setqflist arguments
        let items_array = Array::from_iter(items);

        // Create options dictionary
        let mut opts = Dictionary::new();
        opts.insert("title".to_string(), Object::from(self.title.clone()));

        // If we have an existing qf_id, use it to append
        if let Some(id) = self.qf_id {
            opts.insert("id".to_string(), Object::from(id as Integer));
        }

        // Call setqflist with 'a' (append) action
        // Using a tuple for the arguments (setqflist takes 3 args)
        let args = (
            Object::from(items_array),     // list of items
            Object::from("a".to_string()), // action: append
            Object::from(opts),            // options
        );

        // Call the function - ignore errors but log to stderr
        match api::call_function::<_, i64>("setqflist", args) {
            Ok(id) => {
                // Store the qf_id for future appends
                self.qf_id = Some(id as u64);
            }
            Err(e) => {
                eprintln!("Failed to update quickfix list: {}", e);
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Quickfix is updated immediately in write_batch
        Ok(())
    }
}

impl Default for QuickfixSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_error_type() {
        assert_eq!(QuickfixSink::get_error_type("[ERROR] Something"), "E");
        assert_eq!(QuickfixSink::get_error_type("[WARN] Warning"), "W");
        assert_eq!(QuickfixSink::get_error_type("[INFO] Info"), "I");
        assert_eq!(QuickfixSink::get_error_type("[DEBUG] Debug"), "N");
        assert_eq!(QuickfixSink::get_error_type("[TRACE] Trace"), "N");
        assert_eq!(QuickfixSink::get_error_type("Plain message"), "N");
    }

    #[test]
    fn test_parse_message_simple() {
        let (file, line, col, text, err_type) =
            QuickfixSink::parse_message("[INFO] Just a message");
        assert_eq!(file, "hermes");
        assert_eq!(line, 1);
        assert_eq!(col, 1);
        assert_eq!(text, "Just a message");
        assert_eq!(err_type, "I");
    }

    #[test]
    fn test_parse_message_with_file() {
        let (file, line, _col, text, err_type) =
            QuickfixSink::parse_message("[ERROR] src/main.rs:42: Something failed");
        assert_eq!(file, "src/main.rs");
        assert_eq!(line, 42);
        assert_eq!(text, "Something failed");
        assert_eq!(err_type, "E");
    }

    #[test]
    fn test_parse_message_with_file_col() {
        let (file, line, col, text, err_type) =
            QuickfixSink::parse_message("[WARN] src/lib.rs:123:5: Check this");
        assert_eq!(file, "src/lib.rs");
        assert_eq!(line, 123);
        assert_eq!(col, 5);
        assert_eq!(text, "Check this");
        assert_eq!(err_type, "W");
    }

    #[test]
    fn test_create_qf_item() {
        let _dict = QuickfixSink::create_qf_item("test.rs", 10, 5, "Error here", "E");
        // Dictionary was created successfully
    }
}
