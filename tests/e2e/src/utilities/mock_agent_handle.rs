//! Handle for controlling a MockAgent from tests

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tracing::{error, info};

use super::mock_config::MockConfig;

/// Handle to a running mock agent
///
/// Dropping this handle will gracefully shut down the mock agent thread.
pub struct MockAgentHandle {
    pub config: Arc<Mutex<MockConfig>>,
    pub port: u16,
    thread_handle: Option<JoinHandle<()>>,
    shutdown_sender: Option<async_channel::Sender<()>>,
}

impl MockAgentHandle {
    /// Create a new handle with the given configuration and port
    pub fn new(
        config: Arc<Mutex<MockConfig>>,
        port: u16,
        thread_handle: JoinHandle<()>,
        shutdown_sender: async_channel::Sender<()>,
    ) -> Self {
        Self {
            config,
            port,
            thread_handle: Some(thread_handle),
            shutdown_sender: Some(shutdown_sender),
        }
    }

    /// Get the host for connecting to this mock agent
    pub fn host(&self) -> String {
        "localhost".to_string()
    }

    /// Get the port for connecting to this mock agent
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Explicitly shut down the mock agent
    pub fn close(self) {
        // Drop will handle the cleanup
        drop(self);
    }

    /// Check if the mock agent thread is still running
    pub fn is_running(&self) -> bool {
        self.thread_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

impl Drop for MockAgentHandle {
    fn drop(&mut self) {
        info!("MockAgentHandle dropping - initiating graceful shutdown");

        // Step 1: Send shutdown signal
        if let Some(sender) = self.shutdown_sender.take() {
            let _ = sender.try_send(());
        }

        // Step 2: Join thread with 10 second timeout
        if let Some(handle) = self.thread_handle.take() {
            let timeout = std::time::Duration::from_secs(10);
            let start = std::time::Instant::now();

            loop {
                if start.elapsed() >= timeout {
                    panic!(
                        "Mock agent thread did not shut down within 10 seconds - possible deadlock"
                    );
                }

                if handle.is_finished() {
                    match handle.join() {
                        Ok(_) => {
                            info!("Mock agent thread shut down successfully");
                            break;
                        }
                        Err(e) => {
                            error!("Mock agent thread panicked: {:?}", e);
                            break;
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}
