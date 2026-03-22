use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing_subscriber::fmt::MakeWriter;

use crate::utilities::logging::channel::ChannelWriter;
use crate::utilities::logging::sink::FileSink;

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
}
