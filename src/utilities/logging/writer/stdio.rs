use std::io::{self, Write};

#[derive(Clone)]
pub struct StdoutWriter;

impl StdoutWriter {
    pub fn new() -> Self {
        Self
    }
}

impl Write for StdoutWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::stdout().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

impl Default for StdoutWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::logging::writer::Filtered;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_stdout_writer_write_returns_success() {
        let mut writer = StdoutWriter::new();
        // Test that write returns success with correct byte count
        let result = writer.write(b"test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdout_writer_write_returns_correct_count() {
        let mut writer = StdoutWriter::new();
        let result = writer.write(b"test").unwrap();
        assert_eq!(result, 4);
    }

    #[test]
    fn test_stdout_writer_flush() {
        let mut writer = StdoutWriter::new();
        // Test that flush returns success
        let result = writer.flush();
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdout_writer_empty_write_returns_success() {
        let mut writer = StdoutWriter::new();
        // Test that empty write returns success
        let result = writer.write(b"");
        assert!(result.is_ok());
    }

    #[test]
    fn test_stdout_writer_empty_write_returns_zero() {
        let mut writer = StdoutWriter::new();
        let result = writer.write(b"").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_stdout_writer_implements_filtered() {
        use crate::utilities::logging::LogLevel;
        use crate::utilities::logging::writer::LevelFilterWriter;

        let writer = StdoutWriter::new();
        let mut filtered_writer = writer.filtered(LogLevel::Debug);

        // Verify that we can write through the filtered writer successfully.
        let result = write!(filtered_writer, "test through filtered writer");
        assert!(result.is_ok());
    }
}
