use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use super::sink::LogSink;

const CHANNEL_CAPACITY: usize = 10_000;
const FLUSH_INTERVAL_FILE: usize = 100; // Flush file every 100 messages
const FLUSH_INTERVAL_UI: usize = 10; // Flush UI every 10 messages
const FLUSH_TIMEOUT_MS: u64 = 100; // Flush after 100ms regardless

#[derive(Debug)]
enum LogMessage {
    Data(String),
    Flush,
    Shutdown,
}

/// Channel-based writer that sends logs to a dedicated thread
///
/// Generic over the LogSink type, allowing different output destinations.
pub struct ChannelWriter<S: LogSink> {
    sender: Sender<LogMessage>,
    // We keep a reference to the worker for shutdown
    _worker: Arc<Mutex<Option<Worker<S>>>>,
}

impl<S: LogSink> Clone for ChannelWriter<S> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            _worker: self._worker.clone(),
        }
    }
}

impl<S: LogSink> std::fmt::Debug for ChannelWriter<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelWriter")
            .field("sender", &"Sender<LogMessage>")
            .finish_non_exhaustive()
    }
}

impl<S: LogSink> ChannelWriter<S> {
    /// Create a new channel writer with the given sink
    ///
    /// Spawns a dedicated logging thread that will consume messages
    /// and send them to the sink.
    pub fn new(sink: S, flush_interval: usize) -> io::Result<Self> {
        let (sender, receiver) = bounded(CHANNEL_CAPACITY);

        // Spawn the logging worker thread
        let worker = Worker::spawn(sink, receiver, flush_interval);

        Ok(Self {
            sender,
            _worker: Arc::new(Mutex::new(Some(worker))),
        })
    }

    /// Create a new channel writer with file sink (flush every 100 messages)
    pub fn new_file(sink: S) -> io::Result<Self>
    where
        S: LogSink,
    {
        Self::new(sink, FLUSH_INTERVAL_FILE)
    }

    /// Create a new channel writer with UI sink (flush every 10 messages or 100ms)
    pub fn new_ui(sink: S) -> io::Result<Self>
    where
        S: LogSink,
    {
        Self::new(sink, FLUSH_INTERVAL_UI)
    }

    /// Signal the worker thread to shutdown and wait for it to complete
    ///
    /// This is blocking - it will wait until all queued messages are processed.
    pub fn shutdown(self) {
        // Signal shutdown
        let _ = self.sender.send(LogMessage::Shutdown);

        // Take ownership of worker and wait for it to finish
        if let Ok(mut worker_guard) = self._worker.lock()
            && let Some(worker) = worker_guard.take()
        {
            let _ = worker.join();
        }
    }
}

impl<S: LogSink> Write for ChannelWriter<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Convert bytes to string for easier processing
        // If invalid UTF-8, use lossy conversion
        let message = String::from_utf8_lossy(buf).to_string();

        // Send to channel - non-blocking, ~50ns
        // If channel is full, this will block until space available
        match self.sender.send(LogMessage::Data(message)) {
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

/// Worker thread that consumes from channel and writes to sink
struct Worker<S: LogSink> {
    handle: JoinHandle<()>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: LogSink> Worker<S> {
    fn spawn(mut sink: S, receiver: Receiver<LogMessage>, flush_interval: usize) -> Self {
        let handle = thread::spawn(move || {
            let mut message_buffer: Vec<String> = Vec::with_capacity(flush_interval);
            let mut shutdown_requested = false;
            let timeout = Duration::from_millis(FLUSH_TIMEOUT_MS);

            loop {
                // Try to receive a message with timeout
                let msg = if shutdown_requested {
                    // In shutdown mode, drain remaining messages without blocking
                    match receiver.try_recv() {
                        Ok(msg) => msg,
                        Err(_) => {
                            // Channel empty or disconnected, flush and exit
                            if !message_buffer.is_empty() {
                                if let Err(e) = sink.write_batch(&message_buffer) {
                                    eprintln!("Failed to write final batch: {}", e);
                                }
                                message_buffer.clear();
                            }
                            let _ = sink.flush();
                            break;
                        }
                    }
                } else {
                    // Normal operation - blocking receive with timeout
                    match receiver.recv_timeout(timeout) {
                        Ok(msg) => msg,
                        Err(RecvTimeoutError::Timeout) => {
                            // Timeout occurred - check if we need to flush
                            if !message_buffer.is_empty() {
                                if let Err(e) = sink.write_batch(&message_buffer) {
                                    eprintln!("Failed to write batch on timeout: {}", e);
                                }
                                message_buffer.clear();
                            }
                            continue;
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            // Sender dropped, flush and exit
                            if !message_buffer.is_empty() {
                                if let Err(e) = sink.write_batch(&message_buffer) {
                                    eprintln!("Failed to write final batch: {}", e);
                                }
                                message_buffer.clear();
                            }
                            let _ = sink.flush();
                            break;
                        }
                    }
                };

                // Process the received message
                match msg {
                    LogMessage::Data(data) => {
                        message_buffer.push(data);

                        // Check if we should flush (buffer full)
                        if message_buffer.len() >= flush_interval {
                            if let Err(e) = sink.write_batch(&message_buffer) {
                                eprintln!("Failed to write batch: {}", e);
                            }
                            message_buffer.clear();
                        }
                    }
                    LogMessage::Flush => {
                        // Flush immediately
                        if !message_buffer.is_empty() {
                            if let Err(e) = sink.write_batch(&message_buffer) {
                                eprintln!("Failed to write batch on flush: {}", e);
                            }
                            message_buffer.clear();
                        }
                        if let Err(e) = sink.flush() {
                            eprintln!("Failed to flush sink: {}", e);
                        }
                    }
                    LogMessage::Shutdown => {
                        shutdown_requested = true;
                    }
                }
            }

            // Final flush before exit
            if !message_buffer.is_empty()
                && let Err(e) = sink.write_batch(&message_buffer)
            {
                eprintln!("Failed to write final batch: {}", e);
            }
            if let Err(e) = sink.flush() {
                eprintln!("Failed final flush: {}", e);
            }
        });

        Self {
            handle,
            _phantom: std::marker::PhantomData,
        }
    }

    fn join(self) -> thread::Result<()> {
        self.handle.join()
    }
}

/// Guard struct for channel writer access
pub struct ChannelWriterGuard<S: LogSink> {
    inner: Arc<Mutex<Option<ChannelWriter<S>>>>,
}

impl<S: LogSink> ChannelWriterGuard<S> {
    pub fn new(inner: Arc<Mutex<Option<ChannelWriter<S>>>>) -> Self {
        Self { inner }
    }
}

impl<S: LogSink> std::io::Write for ChannelWriterGuard<S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::other(format!("Lock poisoned: {}", e)))?;

        match guard.as_mut() {
            Some(writer) => writer.write(buf),
            None => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::other(format!("Lock poisoned: {}", e)))?;

        match guard.as_mut() {
            Some(writer) => writer.flush(),
            None => Ok(()),
        }
    }
}

/// Guard struct for direct writer access (non-blocking sinks)
pub struct DirectWriterGuard<S> {
    inner: Arc<Mutex<S>>,
}

impl<S> DirectWriterGuard<S> {
    pub fn new(inner: Arc<Mutex<S>>) -> Self {
        Self { inner }
    }
}

impl<S: std::io::Write> std::io::Write for DirectWriterGuard<S> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::other(format!("Lock poisoned: {}", e)))?;
        guard.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| std::io::Error::other(format!("Lock poisoned: {}", e)))?;
        guard.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::logging::sink::NullSink;
    use pretty_assertions::assert_eq;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock LogSink that tracks all messages and flush calls for testing
    #[derive(Clone)]
    struct TrackingSink {
        messages: Arc<Mutex<Vec<String>>>,
        flush_count: Arc<AtomicUsize>,
    }

    impl TrackingSink {
        fn new() -> Self {
            Self {
                messages: Arc::new(Mutex::new(Vec::new())),
                flush_count: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn get_messages(&self) -> Vec<String> {
            self.messages.lock().unwrap().clone()
        }

        fn get_flush_count(&self) -> usize {
            self.flush_count.load(Ordering::SeqCst)
        }
    }

    impl LogSink for TrackingSink {
        fn write_batch(&mut self, messages: &[String]) -> io::Result<()> {
            self.messages.lock().unwrap().extend_from_slice(messages);
            Ok(())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn test_channel_writer_with_null_sink() {
        let sink = NullSink;
        let mut writer = ChannelWriter::new(sink, 5).unwrap();

        writer.write_all(b"Hello, World!\n").unwrap();
        writer.write_all(b"Second line\n").unwrap();
        writer.flush().unwrap();
        writer.shutdown();
    }

    #[test]
    fn test_channel_writer_empty_message() {
        let sink = NullSink;
        let mut writer = ChannelWriter::new(sink, 5).unwrap();

        writer.write_all(b"").unwrap();
        writer.shutdown();
    }

    #[test]
    fn test_channel_writer_tracks_all_messages() {
        let sink = TrackingSink::new();
        let mut writer = ChannelWriter::new(sink.clone(), 100).unwrap();

        for i in 0..100 {
            writer
                .write_all(format!("Message {}\n", i).as_bytes())
                .unwrap();
        }

        writer.flush().unwrap();
        std::thread::sleep(Duration::from_millis(200));
        writer.shutdown();

        let messages = sink.get_messages();
        assert_eq!(messages.len(), 100);
    }

    #[test]
    fn test_channel_writer_batches_messages() {
        let sink = TrackingSink::new();
        let mut writer = ChannelWriter::new(sink.clone(), 10).unwrap();

        for i in 0..25 {
            writer
                .write_all(format!("Batch msg {}\n", i).as_bytes())
                .unwrap();
        }

        writer.flush().unwrap();
        std::thread::sleep(Duration::from_millis(200));
        writer.shutdown();

        let messages = sink.get_messages();
        assert_eq!(messages.len(), 25);
    }

    #[test]
    fn test_channel_writer_flush_triggers_sink_flush() {
        let sink = TrackingSink::new();
        let mut writer = ChannelWriter::new(sink.clone(), 100).unwrap();

        writer.write_all(b"Test message\n").unwrap();
        writer.flush().unwrap();
        std::thread::sleep(Duration::from_millis(100));

        let flush_count = sink.get_flush_count();
        assert_eq!(flush_count, 1);

        writer.shutdown();
    }

    #[test]
    fn test_channel_writer_timeout_flush() {
        let sink = TrackingSink::new();
        let mut writer = ChannelWriter::new(sink.clone(), 100).unwrap();

        writer.write_all(b"First message\n").unwrap();
        writer.write_all(b"Second message\n").unwrap();

        std::thread::sleep(Duration::from_millis(150));
        writer.shutdown();

        let messages = sink.get_messages();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_channel_worker_death_graceful() {
        let sink = NullSink;
        let mut writer = ChannelWriter::new(sink, 5).unwrap();

        writer.write_all(b"Before death\n").unwrap();

        // Clone the writer to keep one alive after shutdown
        let mut writer2 = writer.clone();
        writer.shutdown();

        // Writing to the cloned writer after the original is shut down should still work
        // because they share the same channel
        let result = writer2.write_all(b"After shutdown of clone\n");
        assert!(result.is_ok());

        writer2.shutdown();
    }

    #[test]
    fn test_channel_writer_clone_shares_channel() {
        let sink = TrackingSink::new();
        let mut writer1 = ChannelWriter::new(sink.clone(), 100).unwrap();
        let mut writer2 = writer1.clone();

        writer1.write_all(b"From writer1\n").unwrap();
        writer2.write_all(b"From writer2\n").unwrap();

        writer1.flush().unwrap();
        std::thread::sleep(Duration::from_millis(100));
        writer1.shutdown();

        let messages = sink.get_messages();
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_channel_writer_empty_shutdown() {
        let sink = TrackingSink::new();
        let sink_clone = sink.clone();
        let writer = ChannelWriter::new(sink, 5).unwrap();

        writer.shutdown();

        // Shutdown triggers at least one flush, but the exact count may vary
        // depending on timing (shutdown flush + disconnect flush)
        let flush_count = sink_clone.get_flush_count();
        assert!(
            flush_count >= 1,
            "Should have at least one flush during shutdown"
        );
    }

    #[test]
    fn test_channel_writer_concurrent_writes_safe() {
        let sink = TrackingSink::new();
        let mut writer = ChannelWriter::new(sink.clone(), 1000).unwrap();

        let mut handles = vec![];
        for thread_id in 0..5 {
            let mut writer_clone = writer.clone();
            let handle = std::thread::spawn(move || {
                for msg_id in 0..20 {
                    let msg = format!("Thread {} Message {}\n", thread_id, msg_id);
                    writer_clone.write_all(msg.as_bytes()).unwrap();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        writer.flush().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        writer.shutdown();

        let messages = sink.get_messages();
        assert_eq!(messages.len(), 100);
    }
}
