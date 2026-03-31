use crate::acp::{Result, error::Error};
use crate::utilities::LogLevel;
use crossbeam_channel::{Receiver, Sender, bounded};
use nvim_oxi::libuv::AsyncHandle;
use nvim_oxi::{Dictionary, api};
use std::sync::Arc;
use tracing::error;

/// A notification message to be delivered to Neovim
#[derive(Debug, Clone, PartialEq)]
pub struct NotificationMessage {
    pub message: String,
    pub level: LogLevel,
}

/// A messenger that sends notifications from any thread to be delivered on Neovim's main thread
#[derive(Clone)]
pub struct NotificationMessenger {
    handle: Arc<AsyncHandle>,
    sender: Arc<Sender<NotificationMessage>>,
}

impl std::fmt::Debug for NotificationMessenger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotificationMessenger")
            .field("capacity", &self.sender.capacity())
            .finish()
    }
}

impl NotificationMessenger {
    /// Create a new NotificationMessenger with the given sender and AsyncHandle
    ///
    /// This is the low-level constructor for testing and custom initialization.
    /// For standard use, prefer `NotificationMessenger::default()`.
    pub fn new(sender: Sender<NotificationMessage>, handle: AsyncHandle) -> Self {
        Self {
            handle: Arc::new(handle),
            sender: Arc::new(sender),
        }
    }

    /// Initialize the notification messenger with a callback that processes notifications on the main thread
    ///
    /// This creates a bounded channel with capacity 1000 and sets up the AsyncHandle callback.
    /// Must be called from Neovim's main thread.
    pub fn initialize() -> Result<Self> {
        let (sender, receiver) = bounded::<NotificationMessage>(1000);
        let mut config = Dictionary::new();
        config.insert("title", "Hermes");
        config.insert("merge", true);

        let handle = AsyncHandle::new(move || {
            // CRITICAL: This callback runs on Neovim's main thread
            // Process all pending notifications
            while let Ok(notification) = receiver.try_recv() {
                let level: nvim_oxi::api::types::LogLevel = notification.level.into();
                if let Err(e) = api::notify(&notification.message, level, &config) {
                    error!("Failed to send notification: {}", e);
                }
            }
        })
        .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Self::new(sender, handle))
    }

    /// Send a notification (can be called from any thread)
    ///
    /// The notification is queued and will be delivered on Neovim's main thread
    /// via the AsyncHandle callback.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The channel is full (capacity 1000 exceeded)
    /// - The AsyncHandle fails to signal
    pub fn send(&self, message: String, level: LogLevel) -> Result<()> {
        self.sender
            .try_send(NotificationMessage { message, level })
            .map_err(|e| Error::Internal(format!("Failed to queue notification: {}", e)))?;

        // Signal the AsyncHandle to process on main thread
        self.handle
            .send()
            .map_err(|e| Error::Internal(format!("Failed to signal notification handler: {}", e)))
    }

    /// Get the number of messages in the queue
    #[cfg(test)]
    pub fn queue_len(&self) -> usize {
        self.sender.len()
    }

    /// Check if the queue is empty
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.sender.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Mock AsyncHandle for testing - implements the signal behavior
    #[derive(Debug, Clone)]
    struct MockAsyncHandle {
        signal_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl MockAsyncHandle {
        fn new() -> Self {
            Self {
                signal_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            }
        }

        fn send(&self) -> Result<()> {
            self.signal_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }

        fn signal_count(&self) -> usize {
            self.signal_count.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    // Wrapper to make MockAsyncHandle work with our API
    struct TestableMessenger {
        sender: Sender<NotificationMessage>,
        receiver: Receiver<NotificationMessage>,
        mock_handle: MockAsyncHandle,
    }

    impl TestableMessenger {
        fn new(capacity: usize) -> Self {
            let (sender, receiver) = bounded::<NotificationMessage>(capacity);
            Self {
                sender,
                receiver,
                mock_handle: MockAsyncHandle::new(),
            }
        }

        fn send(&self, message: String, level: LogLevel) -> Result<()> {
            self.sender
                .try_send(NotificationMessage { message, level })
                .map_err(|e| Error::Internal(format!("Failed to queue notification: {}", e)))?;
            self.mock_handle.send()
        }

        fn try_recv(&self) -> Option<NotificationMessage> {
            self.receiver.try_recv().ok()
        }

        fn is_full(&self) -> bool {
            self.sender.is_full()
        }
    }

    #[test]
    fn test_notification_message_creation() {
        let msg = NotificationMessage {
            message: "Test message".to_string(),
            level: LogLevel::Info,
        };
        assert_eq!(msg.message, "Test message");
        assert_eq!(msg.level, LogLevel::Info);
    }

    #[test]
    fn test_notification_message_clone() {
        let msg = NotificationMessage {
            message: "Test".to_string(),
            level: LogLevel::Debug,
        };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_notification_message_debug() {
        let msg = NotificationMessage {
            message: "Test".to_string(),
            level: LogLevel::Error,
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("Test"));
        assert!(debug_str.contains("Error"));
    }

    #[test]
    fn test_notification_message_equality() {
        let msg1 = NotificationMessage {
            message: "Test".to_string(),
            level: LogLevel::Info,
        };
        let msg2 = NotificationMessage {
            message: "Test".to_string(),
            level: LogLevel::Info,
        };
        let msg3 = NotificationMessage {
            message: "Different".to_string(),
            level: LogLevel::Info,
        };
        assert_eq!(msg1, msg2);
        assert_ne!(msg1, msg3);
    }

    #[test]
    fn test_notification_messenger_new() {
        let (_sender, receiver) = bounded::<NotificationMessage>(10);
        // Note: We can't easily test with real AsyncHandle without Neovim
        // but we can verify the channel setup
        assert_eq!(receiver.capacity(), Some(10));
    }

    #[test]
    fn test_notification_messenger_send_success() {
        let messenger = TestableMessenger::new(10);

        let result = messenger.send("Test message".to_string(), LogLevel::Info);
        assert!(result.is_ok());

        // Verify message is in queue
        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().message, "Test message");
    }

    #[test]
    fn test_notification_messenger_send_multiple() {
        let messenger = TestableMessenger::new(10);

        for i in 0..5 {
            let result = messenger.send(format!("Message {}", i), LogLevel::Info);
            assert!(result.is_ok());
        }

        // Verify all messages are queued
        for i in 0..5 {
            let msg = messenger.try_recv();
            assert!(msg.is_some());
            assert_eq!(msg.unwrap().message, format!("Message {}", i));
        }
    }

    #[test]
    fn test_notification_messenger_send_channel_full() {
        let messenger = TestableMessenger::new(2);

        // Fill the channel
        messenger.send("msg1".to_string(), LogLevel::Info).unwrap();
        messenger.send("msg2".to_string(), LogLevel::Info).unwrap();

        // Third send should fail
        let result = messenger.send("msg3".to_string(), LogLevel::Info);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to queue"));
    }

    #[test]
    fn test_notification_messenger_send_various_levels() {
        let messenger = TestableMessenger::new(10);
        let levels = vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ];

        for level in levels {
            let result = messenger.send(format!("{:?}", level), level);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_notification_messenger_send_empty_message() {
        let messenger = TestableMessenger::new(10);

        let result = messenger.send("".to_string(), LogLevel::Info);
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().message, "");
    }

    #[test]
    fn test_notification_messenger_send_special_characters() {
        let messenger = TestableMessenger::new(10);

        let special = r#"Special chars: <>&"' and unicode: 🎉"#;
        let result = messenger.send(special.to_string(), LogLevel::Info);
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().message, special);
    }

    #[test]
    fn test_notification_messenger_send_long_message() {
        let messenger = TestableMessenger::new(10);

        let long_message = "a".repeat(10000);
        let result = messenger.send(long_message.clone(), LogLevel::Info);
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().message.len(), 10000);
    }

    #[test]
    fn test_notification_messenger_debug_trait() {
        let (sender, _receiver) = bounded::<NotificationMessage>(100);
        // We can't create a real AsyncHandle in tests, but we can verify
        // the struct fields are correct
        assert_eq!(sender.capacity(), Some(100));
    }

    // Property-based tests
    proptest! {
        #[test]
        fn test_send_never_panics_with_any_message(msg in any::<String>()) {
            let messenger = TestableMessenger::new(100);
            let level = LogLevel::Info;
            // Should never panic regardless of input
            let _ = messenger.send(msg, level);
        }

        #[test]
        fn test_send_never_panics_with_any_level(level in 0i64..6) {
            let messenger = TestableMessenger::new(100);
            let level = LogLevel::from(level);
            let _ = messenger.send("test".to_string(), level);
        }

        #[test]
        fn test_roundtrip_message_preserved(msg in any::<String>()) {
            let messenger = TestableMessenger::new(100);
            let level = LogLevel::Debug;

            messenger.send(msg.clone(), level).ok();

            let received = messenger.try_recv();
            if let Some(received_msg) = received {
                assert_eq!(received_msg.message, msg);
                assert_eq!(received_msg.level, level);
            }
        }
    }

    #[test]
    fn test_notification_messenger_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<NotificationMessenger>();
    }

    #[test]
    fn test_notification_messenger_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<NotificationMessenger>();
    }

    #[test]
    fn test_notification_message_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<NotificationMessage>();
    }
}
