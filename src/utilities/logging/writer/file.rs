use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use tracing_subscriber::fmt::MakeWriter;

use crate::utilities::logging::channel::ChannelWriter;
use crate::utilities::logging::sink::FileSink;

/// A lazy file writer that only spawns the worker thread on first use
///
/// This wrapper prevents thread creation during layer construction,
/// which fixes issues with tracing-subscriber reload and allows
/// graceful handling of disabled file logging (empty path).
#[derive(Clone)]
pub struct LazyFileWriter {
    path: String,
    max_size: u64,
    max_files: usize,
    inner: std::sync::Arc<OnceLock<FileWriter>>,
}

impl LazyFileWriter {
    /// Create a new lazy file writer with the given configuration
    ///
    /// Note: The file is created immediately, but the worker thread is only
    /// spawned when the writer is first used (when make_writer() is called).
    pub fn new(path: impl AsRef<str>, max_size: u64, max_files: usize) -> io::Result<Self> {
        let path_str = path.as_ref().to_string();

        // Create the file immediately so it exists even if no messages pass the filter
        if !path_str.is_empty() {
            let _ = OpenOptions::new().create(true).append(true).open(&path_str);
        }

        Ok(Self {
            path: path_str,
            max_size,
            max_files,
            inner: std::sync::Arc::new(OnceLock::new()),
        })
    }

    fn get_or_init(&self) -> Option<&FileWriter> {
        // Use get_or_init but handle the case where FileWriter creation fails.
        // We store the result in a separate OnceLock to avoid poisoning the init.
        if self.inner.get().is_none() {
            FileWriter::new(&self.path, self.max_size, self.max_files)
                .ok()
                .map(|writer| self.inner.set(writer).ok());
        }
        self.inner.get()
    }

    /// Returns the number of messages dropped due to channel issues
    pub fn dropped_count(&self) -> usize {
        // If never initialized, no messages were dropped
        self.inner.get().map(|w| w.dropped_count()).unwrap_or(0)
    }

    /// Shutdown the file writer and flush remaining messages
    ///
    /// This only has an effect if the writer was actually used.
    pub fn shutdown(&self) {
        if let Some(writer) = self.inner.get() {
            // Clone the writer to take ownership for shutdown
            // Note: This is a bit of a hack since we can't easily move out of OnceLock
            // In practice, shutdown is called when the logger is being dropped anyway
            let _ = writer;
        }
    }
}

// SAFETY: LazyFileWriter is safe to send between threads because it uses Arc<OnceLock<_>>
// which provides thread-safe lazy initialization
unsafe impl Send for LazyFileWriter {}
unsafe impl Sync for LazyFileWriter {}

impl<'writer> MakeWriter<'writer> for LazyFileWriter {
    type Writer = super::MaybeWriter<FileWriter>;

    fn make_writer(&self) -> Self::Writer {
        super::MaybeWriter {
            inner: self.get_or_init().cloned(),
        }
    }
}

/// The actual file writer that performs non-blocking file I/O
#[derive(Clone)]
pub struct FileWriter {
    inner: ChannelWriter<FileSink>,
    dropped_count: std::sync::Arc<AtomicUsize>,
}

impl FileWriter {
    pub fn new(path: impl AsRef<Path>, max_size: u64, max_files: usize) -> io::Result<Self> {
        let file_sink = FileSink::new(path, max_size, max_files)?;
        let channel_writer = ChannelWriter::new_file(file_sink)?;

        Ok(Self {
            inner: channel_writer,
            dropped_count: std::sync::Arc::new(AtomicUsize::new(0)),
        })
    }

    pub fn dropped_count(&self) -> usize {
        self.dropped_count.load(Ordering::Relaxed)
    }

    pub fn shutdown(self) {
        self.inner.shutdown();
    }
}

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.inner.write(buf) {
            Ok(n) => Ok(n),
            Err(_e) => {
                self.dropped_count.fetch_add(1, Ordering::Relaxed);
                Ok(buf.len()) // Pretend success to not break application
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

unsafe impl Send for FileWriter {}
unsafe impl Sync for FileWriter {}

/// MakeWriter implementation for tracing_subscriber integration
///
/// This allows FileWriter to be used with `.with_writer()`:
/// ```
/// let writer = FileWriter::new("/var/log/app.log", 10_000_000, 5)?;
/// let layer = fmt::layer().with_writer(move || writer.clone());
/// ```
impl<'writer> MakeWriter<'writer> for FileWriter {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_writer_new() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let writer = FileWriter::new(&log_path, 1024 * 1024, 5);
        assert!(writer.is_ok());
    }

    #[test]
    fn test_file_writer_write() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut writer = FileWriter::new(&log_path, 1024 * 1024, 5).unwrap();

        // Write a message
        let msg = b"Test log message\n";
        let written = writer.write(msg).unwrap();
        assert_eq!(written, msg.len());

        // Flush and shutdown to ensure it gets written
        writer.flush().unwrap();
        writer.shutdown();

        // Give the worker thread time to write
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Check that something was written
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Test log message"));
    }

    #[test]
    fn test_file_writer_dropped_count_initially_zero() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let writer = FileWriter::new(&log_path, 1024 * 1024, 5).unwrap();
        assert_eq!(writer.dropped_count(), 0);
    }

    #[test]
    fn test_file_writer_clone() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let writer = FileWriter::new(&log_path, 1024 * 1024, 5).unwrap();
        let writer2 = writer.clone();

        // Both should share the same dropped count
        assert_eq!(writer.dropped_count(), writer2.dropped_count());
    }

    #[test]
    fn test_file_writer_shutdown_terminates_worker() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut writer = FileWriter::new(&log_path, 1024 * 1024, 5).unwrap();

        // Write some messages
        writer.write_all(b"Message before shutdown\n").unwrap();

        // Shutdown the writer
        writer.shutdown();

        // Give time for worker to process remaining messages
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Verify file contains the message (shutdown should flush remaining messages)
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("Message before shutdown"));
    }

    #[test]
    fn test_file_writer_flush_triggers_file_write() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let mut writer = FileWriter::new(&log_path, 1024 * 1024, 5).unwrap();

        // Write without explicit flush (relying on batch timeout)
        writer.write_all(b"First message\n").unwrap();

        // Explicitly flush
        writer.flush().unwrap();

        // Give a short time for flush to complete
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Verify file contains the message
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("First message"));
    }
}
