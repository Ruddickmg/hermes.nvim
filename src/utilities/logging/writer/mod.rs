mod ansi_strip;
mod file;
mod message;
mod notification;
mod stdio;

pub use ansi_strip::*;
pub use file::*;
pub use message::*;
pub use notification::*;
pub use stdio::*;

use std::io::{self, Write};
use tracing::Metadata;
use tracing_subscriber::fmt::writer::MakeWriter;

use crate::utilities::LogLevel;

/// A writer that conditionally discards output based on log level
pub struct MaybeWriter<W> {
    inner: Option<W>,
}

impl<W: Write> Write for MaybeWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.inner {
            Some(w) => w.write(buf),
            None => Ok(buf.len()), // Pretend success, discard data
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.inner {
            Some(w) => w.flush(),
            None => Ok(()),
        }
    }
}

/// A wrapper that filters writes based on log level
///
/// This wraps any writer that implements Clone + Write, and implements MakeWriter
/// by cloning the inner writer for each write operation.
#[derive(Clone)]
pub struct LevelFilterWriter<W> {
    inner: W,
    level: tracing::Level,
}

impl<W> LevelFilterWriter<W> {
    pub fn new(inner: W, level: tracing::Level) -> Self {
        Self { inner, level }
    }
}

impl<W> Write for LevelFilterWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W> MakeWriter<'_> for LevelFilterWriter<W>
where
    W: Write + Clone + 'static,
{
    type Writer = MaybeWriter<Self>;

    fn make_writer_for(&self, meta: &Metadata<'_>) -> Self::Writer {
        if *meta.level() <= self.level {
            MaybeWriter {
                inner: Some(self.clone()),
            }
        } else {
            MaybeWriter { inner: None }
        }
    }

    fn make_writer(&self) -> Self::Writer {
        MaybeWriter {
            inner: Some(self.clone()),
        }
    }
}

/// Trait to add filtering capability to writers
///
/// This trait is implemented for types that implement both Write and Clone.
/// It wraps them in a LevelFilterWriter that filters based on log level.
pub trait Filtered: Clone + Write + 'static {
    /// Wrap this writer with a LevelFilterWriter
    fn filtered(self, level: LogLevel) -> LevelFilterWriter<Self> {
        LevelFilterWriter::new(self, level.into())
    }
}

// Implement Filtered for any type that is Clone + Write + 'static
impl<T> Filtered for T where T: Clone + Write + 'static {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A test writer that tracks what was written
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

        fn get_data(&self) -> Vec<u8> {
            self.data.lock().unwrap().clone()
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
    fn test_maybe_writer_discards_when_inner_is_none() {
        let mut writer = MaybeWriter::<TrackingWriter> { inner: None };

        // Write should succeed but discard data
        let result = writer.write(b"test data");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 9); // Returns length as if written

        // Flush should succeed
        assert!(writer.flush().is_ok());
    }

    #[test]
    fn test_maybe_writer_writes_when_inner_is_some() {
        let tracking = TrackingWriter::new();
        let mut writer = MaybeWriter {
            inner: Some(tracking.clone()),
        };

        // Write should succeed and store data
        assert!(writer.write(b"test data").is_ok());
        assert_eq!(tracking.get_data(), b"test data");
    }

    #[test]
    fn test_filtered_trait_creates_level_filter_writer() {
        use crate::utilities::logging::LogLevel;

        let tracking = TrackingWriter::new();
        let filtered = tracking.filtered(LogLevel::Info);

        // Verify the writer was created with correct level
        assert_eq!(filtered.level, tracing::Level::INFO);
    }

    #[test]
    fn test_make_writer_returns_valid_writer() {
        // Test that make_writer always returns a writer with inner set
        let tracking = TrackingWriter::new();
        let filter_writer = LevelFilterWriter::new(tracking.clone(), tracing::Level::INFO);

        // make_writer should always return Some inner
        let mut writer = filter_writer.make_writer();
        assert!(writer.write(b"test").is_ok());

        // Data should be written
        assert_eq!(tracking.get_data(), b"test");
    }

    #[test]
    fn test_maybe_writer_with_none_discards_all_data() {
        // This test confirms the core filtering behavior:
        // When MaybeWriter has inner=None (which happens when log level is filtered out),
        // all data is silently discarded

        // Create a MaybeWriter directly with None inner
        // This simulates what happens when make_writer_for blocks a log level
        let mut blocked_writer = MaybeWriter::<TrackingWriter> { inner: None };

        // Try to write data - should succeed but not actually write anywhere
        let data = b"this should be blocked";
        let result = blocked_writer.write(data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len()); // Pretends to write all bytes

        // Flush should also succeed but do nothing
        assert!(blocked_writer.flush().is_ok());

        // The key assertion: data was "written" (returned Ok) but actually discarded
        // This demonstrates the filtering behavior - logs are silently dropped
    }
}
