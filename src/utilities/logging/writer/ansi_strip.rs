//! ANSI escape code stripping writer wrapper.
//!
//! Provides a generic, reusable writer wrapper that strips ANSI escape codes
//! before writing to the underlying destination.
//!
//! ## Why This Is Needed
//!
//! There is a known upstream bug in `tracing-subscriber` where span fields
//! from `#[instrument]` macros are formatted with ANSI escape codes regardless
//! of the `with_ansi(false)` setting on the layer. This affects structured
//! field formatting (e.g. `agent=Opencode` gets rendered with italic/dim codes).
//!
//! Since we want ANSI colors in terminal and notification output but clean text
//! in file and message logs, we strip ANSI codes at the writer level for
//! destinations that should not contain them.
//!
//! See: <https://github.com/tokio-rs/tracing/issues/1310>

use std::io::{self, Write};

/// A writer wrapper that strips ANSI escape codes before forwarding to the
/// inner writer. Wraps any type that implements `Write + Clone`.
///
/// # Example
/// ```ignore
/// let file_writer = FileWriter::new("app.log", 10_000_000, 5)?;
/// let clean_writer = AnsiStrip::new(file_writer);
/// // All writes through clean_writer will have ANSI codes removed
/// ```
#[derive(Clone)]
pub struct AnsiStrip<W: Write + Clone> {
    inner: W,
}

impl<W: Write + Clone> AnsiStrip<W> {
    /// Create a new ANSI-stripping writer that wraps `inner`.
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: Write + Clone> Write for AnsiStrip<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let stripped = strip_ansi_escapes::strip(buf);
        self.inner.write_all(&stripped)?;
        // Return original buf length so callers see the correct byte count
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct TrackingWriter {
        data: Arc<Mutex<Vec<u8>>>,
    }

    impl TrackingWriter {
        fn new() -> Self {
            Self {
                data: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn get_string(&self) -> String {
            String::from_utf8(self.data.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for TrackingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.data.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn strips_ansi_escape_codes_from_output() {
        let tracking = TrackingWriter::new();
        let mut writer = AnsiStrip::new(tracking.clone());

        // Write text containing ANSI codes (italic, dim, reset)
        let input = b"\x1b[3magent\x1b[0m\x1b[2m=\x1b[0mOpencode";
        writer.write_all(input).unwrap();

        assert_eq!(tracking.get_string(), "agent=Opencode");
    }

    #[test]
    fn passes_through_plain_text_unchanged() {
        let tracking = TrackingWriter::new();
        let mut writer = AnsiStrip::new(tracking.clone());

        writer.write_all(b"plain text without ansi").unwrap();

        assert_eq!(tracking.get_string(), "plain text without ansi");
    }

    #[test]
    fn returns_original_buffer_length() {
        let tracking = TrackingWriter::new();
        let mut writer = AnsiStrip::new(tracking);

        let input = b"\x1b[31mred text\x1b[0m";
        let result = writer.write(input).unwrap();

        // Should return original length, not stripped length
        assert_eq!(result, input.len());
    }

    #[test]
    fn handles_empty_input() {
        let tracking = TrackingWriter::new();
        let mut writer = AnsiStrip::new(tracking.clone());

        writer.write_all(b"").unwrap();

        assert_eq!(tracking.get_string(), "");
    }

    #[test]
    fn strips_multiple_ansi_sequences() {
        let tracking = TrackingWriter::new();
        let mut writer = AnsiStrip::new(tracking.clone());

        // Multiple color codes: red, green, bold, reset
        let input = b"\x1b[31mERROR\x1b[0m \x1b[32mOK\x1b[0m \x1b[1mbold\x1b[0m";
        writer.write_all(input).unwrap();

        assert_eq!(tracking.get_string(), "ERROR OK bold");
    }
}
