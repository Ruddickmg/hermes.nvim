use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use super::file::SizeBasedFileAppender;

const CHANNEL_CAPACITY: usize = 10_000;
const FLUSH_INTERVAL: usize = 100;

#[derive(Debug)]
enum LogMessage {
    Data(Vec<u8>),
    Flush,
    Shutdown,
}

/// Channel-based writer that sends logs to a dedicated thread
pub struct ChannelWriter {
    sender: Sender<LogMessage>,
    // We keep a reference to the worker for shutdown
    _worker: Arc<Mutex<Option<Worker>>>,
}

impl std::fmt::Debug for ChannelWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelWriter")
            .field("sender", &"Sender<LogMessage>")
            .finish_non_exhaustive()
    }
}

impl ChannelWriter {
    /// Create a new channel writer with the given file appender
    ///
    /// Spawns a dedicated logging thread that will consume messages
    /// and write them to the file.
    pub fn new(file_appender: SizeBasedFileAppender) -> io::Result<Self> {
        let (sender, receiver) = bounded(CHANNEL_CAPACITY);

        // Spawn the logging worker thread
        let worker = Worker::spawn(file_appender, receiver);

        Ok(Self {
            sender,
            _worker: Arc::new(Mutex::new(Some(worker))),
        })
    }

    /// Signal the worker thread to shutdown and wait for it to complete
    ///
    /// This is blocking - it will wait until all queued messages are processed.
    pub fn shutdown(self) {
        // Signal shutdown
        let _ = self.sender.send(LogMessage::Shutdown);

        // Take ownership of worker and wait for it to finish
        if let Ok(mut worker_guard) = self._worker.lock()
            && let Some(worker) = worker_guard.take() {
                let _ = worker.join();
            }
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Send to channel - non-blocking, ~50ns
        // If channel is full, this will block until space available
        // (bounded channel behavior)
        match self.sender.send(LogMessage::Data(buf.to_vec())) {
            Ok(_) => Ok(buf.len()),
            Err(_) => {
                // Channel disconnected (worker died)
                // Return success to not break application, but log is lost
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.sender.send(LogMessage::Flush) {
            Ok(_) => Ok(()),
            Err(_) => {
                // Channel disconnected
                Ok(())
            }
        }
    }
}

/// Worker thread that consumes from channel and writes to file
struct Worker {
    handle: JoinHandle<()>,
}

impl Worker {
    fn spawn(mut file_appender: SizeBasedFileAppender, receiver: Receiver<LogMessage>) -> Self {
        let handle = thread::spawn(move || {
            let mut message_count = 0;
            let mut shutdown_requested = false;

            loop {
                // Try to receive a message
                let msg = if shutdown_requested {
                    // In shutdown mode, drain remaining messages without blocking
                    match receiver.try_recv() {
                        Ok(msg) => msg,
                        Err(TryRecvError::Empty) => {
                            // Channel empty and shutdown requested, we're done
                            break;
                        }
                        Err(TryRecvError::Disconnected) => {
                            // Sender dropped, we're done
                            break;
                        }
                    }
                } else {
                    // Normal operation - blocking receive
                    match receiver.recv() {
                        Ok(msg) => msg,
                        Err(_) => {
                            // Sender dropped, exit
                            break;
                        }
                    }
                };

                // Process the message
                match msg {
                    LogMessage::Data(data) => {
                        if let Err(e) = file_appender.write_all(&data) {
                            // Log error via stderr since we can't use tracing
                            eprintln!("Failed to write to log file: {}", e);
                        }
                        message_count += 1;

                        // Flush every FLUSH_INTERVAL messages
                        if message_count >= FLUSH_INTERVAL {
                            if let Err(e) = file_appender.flush() {
                                eprintln!("Failed to flush log file: {}", e);
                            }
                            message_count = 0;
                        }
                    }
                    LogMessage::Flush => {
                        if let Err(e) = file_appender.flush() {
                            eprintln!("Failed to flush log file: {}", e);
                        }
                        message_count = 0;
                    }
                    LogMessage::Shutdown => {
                        shutdown_requested = true;
                    }
                }
            }

            // Final flush before exit
            if let Err(e) = file_appender.flush() {
                eprintln!("Failed to final flush log file: {}", e);
            }
        });

        Self { handle }
    }

    fn join(self) -> thread::Result<()> {
        self.handle.join()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_channel_writer_basic() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        // Create writer
        let appender = SizeBasedFileAppender::new(&log_path, 1024 * 1024, 5).unwrap();
        let mut writer = ChannelWriter::new(appender).unwrap();

        // Write some data
        writer.write_all(b"Hello, World!\n").unwrap();
        writer.write_all(b"Second line\n").unwrap();

        // Flush explicitly
        writer.flush().unwrap();

        // Shutdown (waits for drain)
        writer.shutdown();

        // Verify file contains both lines
        let content = std::fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.as_slice(), &["Hello, World!", "Second line"]);
    }

    #[test]
    fn test_channel_writer_empty_message() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let appender = SizeBasedFileAppender::new(&log_path, 1024 * 1024, 5).unwrap();
        let mut writer = ChannelWriter::new(appender).unwrap();

        // Write empty message - should not fail
        writer.write_all(b"").unwrap();
        writer.shutdown();

        // File should exist but be empty
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.is_empty());
    }

    #[test]
    fn test_channel_writer_unicode_content() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let appender = SizeBasedFileAppender::new(&log_path, 1024 * 1024, 5).unwrap();
        let mut writer = ChannelWriter::new(appender).unwrap();

        // Write unicode and special characters
        let unicode_msg = "Unicode: 你好世界 émoji 🎉\n";
        writer.write_all(unicode_msg.as_bytes()).unwrap();
        writer.shutdown();

        // Verify unicode preserved
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert_eq!(content, unicode_msg);
    }

    #[test]
    fn test_channel_writer_concurrent() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let appender = SizeBasedFileAppender::new(&log_path, 1024 * 1024, 5).unwrap();
        let writer = ChannelWriter::new(appender).unwrap();

        // Share writer across threads
        let writer = Arc::new(Mutex::new(writer));

        // Spawn 10 threads, each writing 100 lines
        let mut handles = vec![];
        for thread_id in 0..10 {
            let writer_clone = Arc::clone(&writer);
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let msg = format!("Thread {} message {}\n", thread_id, i);
                    let mut w = writer_clone.lock().unwrap();
                    w.write_all(msg.as_bytes()).unwrap();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Shutdown writer
        let writer = Arc::try_unwrap(writer).unwrap().into_inner().unwrap();
        writer.shutdown();

        // Verify all 1000 messages written
        let content = std::fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1000); // 10 threads * 100 messages
    }
}
