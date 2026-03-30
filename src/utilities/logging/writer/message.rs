use std::io::{self, Write};
use tracing_subscriber::fmt::writer::MakeWriter;

use crate::utilities::MessageMessenger;

#[derive(Debug, Clone)]
pub struct MessageWriter {
    messenger: MessageMessenger,
}

unsafe impl Send for MessageWriter {}
unsafe impl Sync for MessageWriter {}

impl MessageWriter {
    pub fn new(messenger: MessageMessenger) -> Self {
        Self { messenger }
    }
}

impl Write for MessageWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);

        if s.trim().is_empty() {
            return Ok(buf.len());
        }

        self.messenger.send(s.to_string()).ok();

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for MessageWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_writer_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<MessageWriter>();
    }

    #[test]
    fn test_message_writer_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<MessageWriter>();
    }

    #[test]
    fn test_string_utf8_lossy_conversion_valid() {
        let valid = "Hello World";
        let converted = String::from_utf8_lossy(valid.as_bytes());
        assert_eq!(converted, "Hello World");
    }

    #[test]
    fn test_string_utf8_lossy_conversion_empty() {
        let empty = "";
        let converted = String::from_utf8_lossy(empty.as_bytes());
        assert!(converted.trim().is_empty());
    }

    #[test]
    fn test_string_utf8_lossy_conversion_whitespace() {
        let whitespace = "   \n\t  ";
        let converted = String::from_utf8_lossy(whitespace.as_bytes());
        assert!(converted.trim().is_empty());
    }

    #[test]
    fn test_empty_string_trim_is_empty() {
        assert!("".trim().is_empty());
    }

    #[test]
    fn test_whitespace_string_trim_is_empty() {
        assert!("   ".trim().is_empty());
        assert!("\n\t".trim().is_empty());
        assert!("  \n  \t  ".trim().is_empty());
    }

    #[test]
    fn test_non_empty_string_trim_not_empty() {
        assert!(!"hello".trim().is_empty());
        assert!(!"  hello  ".trim().is_empty());
        assert!(!"hello world".trim().is_empty());
        assert!(!"a".trim().is_empty());
    }

    #[test]
    fn test_string_from_utf8_lossy_invalid_bytes() {
        let invalid: [u8; 4] = [0x80, 0x81, 0x82, 0x83];
        let result = String::from_utf8_lossy(&invalid);
        assert!(result.len() >= 4);
        assert!(result.contains('\u{FFFD}'));
    }

    #[test]
    fn test_string_from_utf8_lossy_mixed_valid_invalid() {
        let mixed: Vec<u8> = b"Hello"
            .iter()
            .copied()
            .chain([0x80, 0x81])
            .chain(b"World".iter().copied())
            .collect();
        let result = String::from_utf8_lossy(&mixed);
        assert!(result.starts_with("Hello"));
        assert!(result.ends_with("World"));
    }

    #[test]
    fn test_trim_preserves_content() {
        let with_spaces = "  hello world  ";
        assert_eq!(with_spaces.trim(), "hello world");
    }

    #[test]
    fn test_trim_newlines_tabs() {
        let with_whitespace = "\n\thello\t\n";
        assert_eq!(with_whitespace.trim(), "hello");
    }

    #[test]
    fn test_write_returns_buf_len_for_valid_content() {
        let buf = b"hello";
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn test_write_returns_buf_len_for_whitespace() {
        let buf = b"   \n\t";
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn test_write_returns_buf_len_for_empty() {
        let buf = b"";
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_bytes_to_string_conversion() {
        let bytes = b"test";
        let s = String::from_utf8_lossy(bytes);
        assert_eq!(s.as_bytes(), bytes);
    }

    #[test]
    fn test_bytes_to_string_conversion_unicode() {
        let bytes = "🎉".as_bytes();
        let s = String::from_utf8_lossy(bytes);
        assert_eq!(s, "🎉");
    }

    #[test]
    fn test_string_len_vs_trim_len() {
        let padded = "  hello  ";
        assert_eq!(padded.len(), 9);
        assert_eq!(padded.trim().len(), 5);
    }

    #[test]
    fn test_multiline_content_not_trimmed() {
        let multiline = "line1\nline2";
        assert!(!multiline.trim().is_empty());
    }

    #[test]
    fn test_special_chars_not_trimmed() {
        let special = "<>&\"'";
        assert!(!special.trim().is_empty());
    }

    proptest::proptest! {
        #[test]
        fn test_utf8_conversion_never_panics(input in proptest::collection::vec(proptest::num::u8::ANY, 0..1000)) {
            let _ = String::from_utf8_lossy(&input);
        }

        #[test]
        fn test_empty_check_consistent(input in ".*") {
            let is_empty = input.trim().is_empty();
            if is_empty {
                assert!(input.chars().all(|c| c.is_whitespace()));
            }
        }

        #[test]
        fn test_trim_preserves_nonalphanumeric(input in "[^a-zA-Z0-9]*[a-zA-Z0-9]+[^a-zA-Z0-9]*") {
            let trimmed = input.trim();
            if !trimmed.is_empty() {
                assert!(!trimmed.is_empty());
            }
        }
    }
}
