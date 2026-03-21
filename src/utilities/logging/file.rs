//! Size-based file rotation for logging
//!
//! This module provides a file appender that rotates log files based on size,
//! with no external dependencies beyond std. It checks file size before each
//! write and rotates inline when the limit is exceeded.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

/// A file appender that rotates based on file size
///
/// Before each write, checks if the write would exceed max_size.
/// If so, rotates the file (shifting backups, deleting old ones).
/// Uses only std::fs and std::io - zero chrono dependencies.
pub struct SizeBasedFileAppender {
    /// Base path for the log file (e.g., "/path/to/app.log")
    path: PathBuf,
    /// Maximum size in bytes before rotation
    max_size: u64,
    /// Maximum number of backup files to keep (0 = no backups)
    max_files: usize,
    /// Current size of the active log file
    current_size: u64,
    /// The active file writer (None when closed/rotating)
    writer: Option<File>,
}

impl SizeBasedFileAppender {
    /// Create a new size-based file appender
    ///
    /// # Arguments
    /// * `path` - Base path for the log file
    /// * `max_size` - Maximum size in bytes before rotation
    /// * `max_files` - Maximum number of backup files to keep
    pub fn new(path: impl AsRef<Path>, max_size: u64, max_files: usize) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Open or create the file
        let file = OpenOptions::new().create(true).append(true).open(&path)?;

        // Get current file size
        let metadata = file.metadata()?;
        let current_size = metadata.len();

        Ok(Self {
            path,
            max_size,
            max_files,
            current_size,
            writer: Some(file),
        })
    }

    /// Check if we need to rotate before writing `bytes_to_write`
    fn rotate_if_needed(&mut self, bytes_to_write: usize) -> io::Result<()> {
        let new_size = self.current_size + bytes_to_write as u64;

        if new_size > self.max_size {
            self.perform_rotation()?;
        }

        Ok(())
    }

    /// Perform the file rotation
    ///
    /// 1. Close current file
    /// 2. Shift backup files: .2 -> .3, .1 -> .2, etc.
    /// 3. Rename current file to .1
    /// 4. Delete files exceeding max_files
    /// 5. Open new file
    fn perform_rotation(&mut self) -> io::Result<()> {
        // Close current file
        self.writer = None;

        // Delete oldest file if it exists (file.N where N = max_files)
        if self.max_files > 0 {
            let oldest = self.path.with_extension(format!("{}", self.max_files));
            let _ = fs::remove_file(&oldest); // Ignore if doesn't exist
        }

        // Shift backup files down: .(N-1) -> .N, ..., .1 -> .2
        for i in (1..self.max_files).rev() {
            let from = self.path.with_extension(format!("{}", i));
            let to = self.path.with_extension(format!("{}", i + 1));

            // Try to rename, ignore errors (file might not exist)
            let _ = fs::rename(&from, &to);
        }

        // Rename current file to .1 (if max_files > 0)
        if self.max_files > 0 && self.path.exists() {
            let backup = self.path.with_extension("1");
            fs::rename(&self.path, &backup)?;
        } else if self.max_files == 0 {
            // No backups, just truncate current file
            let _ = fs::remove_file(&self.path);
        }

        // Open new file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        self.writer = Some(file);
        self.current_size = 0;

        Ok(())
    }

    /// Get the current file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the current file size
    pub fn current_size(&self) -> u64 {
        self.current_size
    }
}

impl Write for SizeBasedFileAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Check if we need to rotate before writing
        self.rotate_if_needed(buf.len())?;

        // Write to file
        let written = match self.writer {
            Some(ref mut file) => file.write(buf)?,
            None => {
                return Err(io::Error::other(
                    "File writer not available",
                ))
            }
        };

        // Update current size
        self.current_size += written as u64;

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.writer {
            Some(ref mut file) => file.flush(),
            None => Ok(()),
        }
    }
}

/// Make the appender usable with tracing_subscriber
impl tracing_subscriber::fmt::MakeWriter<'_> for SizeBasedFileAppender {
    type Writer = Self;

    fn make_writer(&self) -> Self::Writer {
        // Clone is expensive for files, so we need to handle this differently
        // For now, return a reference to self (this requires Arc<Mutex<>> wrapper)
        unimplemented!("Use Arc<Mutex<SizeBasedFileAppender>> for thread-safe access")
    }
}

/// Factory function to create a size-based appender
pub fn size_based(
    path: impl AsRef<Path>,
    max_size: u64,
    max_files: usize,
) -> io::Result<SizeBasedFileAppender> {
    SizeBasedFileAppender::new(path, max_size, max_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_size_based_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        // Create appender with 100 byte limit, keep 2 backups
        let mut appender = SizeBasedFileAppender::new(&log_path, 100, 2).unwrap();

        // Write 50 bytes
        appender
            .write_all(b"01234567890123456789012345678901234567890123456789")
            .unwrap();
        assert_eq!(appender.current_size(), 50);

        // Write another 50 bytes (total 100, still within limit)
        appender
            .write_all(b"01234567890123456789012345678901234567890123456789")
            .unwrap();
        assert_eq!(appender.current_size(), 100);

        // Write 10 more bytes - should trigger rotation
        appender.write_all(b"0123456789").unwrap();

        // Current file should now have 10 bytes
        assert_eq!(appender.current_size(), 10);

        // Check that backup file was created
        let backup_path = log_path.with_extension("1");
        assert!(backup_path.exists());

        // Verify backup contains 100 bytes
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content.len(), 100);
    }

    #[test]
    fn test_max_files_limit() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        // Create appender with 50 byte limit, keep 1 backup
        let mut appender = SizeBasedFileAppender::new(&log_path, 50, 1).unwrap();

        // Trigger multiple rotations
        for i in 0..5 {
            appender
                .write_all(
                    format!("Iteration {} with enough data to trigger rotation", i).as_bytes(),
                )
                .unwrap();
        }

        // Should only have 1 backup file (.1)
        assert!(log_path.with_extension("1").exists());
        assert!(!log_path.with_extension("2").exists()); // Should be deleted
    }

    #[test]
    fn test_zero_max_files() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        // Create appender with 50 byte limit, no backups
        let mut appender = SizeBasedFileAppender::new(&log_path, 50, 0).unwrap();

        // Write data
        appender.write_all(b"First content").unwrap();

        // Trigger rotation
        appender
            .write_all(b"More data to trigger rotation of the file")
            .unwrap();

        // Should NOT have backup file
        assert!(!log_path.with_extension("1").exists());

        // Current file should only have the second write
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("More data"));
        assert!(!content.contains("First content"));
    }
}
