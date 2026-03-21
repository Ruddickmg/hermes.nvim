use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use super::sink::LogSink;

const CHANNEL_CAPACITY: usize = 10_000;
const FLUSH_INTERVAL_FILE: usize = 100;      // Flush file every 100 messages
const FLUSH_INTERVAL_UI: usize = 10;       // Flush UI every 10 messages
const FLUSH_TIMEOUT_MS: u64 = 100;         // Flush after 100ms regardless

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
            && let Some(worker) = worker_guard.take() {
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
                && let Err(e) = sink.write_batch(&message_buffer) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::logging::sink::NullSink;

    #[test]
    fn test_channel_writer_with_null_sink() {
        let sink = NullSink;
        let mut writer = ChannelWriter::new(sink, 5).unwrap();

        // Write some data
        writer.write_all(b"Hello, World!\n").unwrap();
        writer.write_all(b"Second line\n").unwrap();

        // Flush explicitly
        writer.flush().unwrap();

        // Shutdown
        writer.shutdown();
    }

    #[test]
    fn test_channel_writer_empty_message() {
        let sink = NullSink;
        let mut writer = ChannelWriter::new(sink, 5).unwrap();

        // Write empty message - should not fail
        writer.write_all(b"").unwrap();
        writer.shutdown();
    }
}
