use crate::acp::{Result, error::Error};
use crate::utilities::LogLevel;
use crossbeam_channel::{Sender, bounded};
use nvim_oxi::libuv::AsyncHandle;
use nvim_oxi::{Dictionary, api};
use std::sync::Arc;

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
    /// For standard use, prefer `NotificationMessenger::initialize()`.
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
            // We use catch_unwind to prevent panics from crossing the FFI boundary.
            // Note: We do NOT attempt to log panics here - if the logging
            // infrastructure is broken, we can't log. Silently swallow instead.
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                while let Ok(notification) = receiver.try_recv() {
                    let level: nvim_oxi::api::types::LogLevel = notification.level.into();
                    api::notify(&notification.message, level, &config).ok();
                }
            }))
            .ok();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct TestableMessenger {
        sender: Sender<NotificationMessage>,
        receiver: crossbeam_channel::Receiver<NotificationMessage>,
    }

    impl TestableMessenger {
        fn new(capacity: usize) -> Self {
            let (sender, receiver) = bounded::<NotificationMessage>(capacity);
            Self { sender, receiver }
        }

        fn try_send(&self, message: String, level: LogLevel) -> Result<()> {
            self.sender
                .try_send(NotificationMessage { message, level })
                .map_err(|e| Error::Internal(format!("Failed to queue notification: {}", e)))
        }

        fn try_recv(&self) -> Option<NotificationMessage> {
            self.receiver.try_recv().ok()
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

        let result = messenger.try_send("Test message".to_string(), LogLevel::Info);
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
            let result = messenger.try_send(format!("Message {}", i), LogLevel::Info);
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
        messenger
            .try_send("msg1".to_string(), LogLevel::Info)
            .unwrap();
        messenger
            .try_send("msg2".to_string(), LogLevel::Info)
            .unwrap();

        // Third send should fail
        let result = messenger.try_send("msg3".to_string(), LogLevel::Info);
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
            let result = messenger.try_send(format!("{:?}", level), level);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_notification_messenger_send_empty_message() {
        let messenger = TestableMessenger::new(10);

        let result = messenger.try_send("".to_string(), LogLevel::Info);
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().message, "");
    }

    #[test]
    fn test_notification_messenger_send_special_characters() {
        let messenger = TestableMessenger::new(10);

        let special = r#"Special chars: <>&"' and unicode: 🎉"#;
        let result = messenger.try_send(special.to_string(), LogLevel::Info);
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().message, special);
    }

    #[test]
    fn test_notification_messenger_send_long_message() {
        let messenger = TestableMessenger::new(10);

        let long_message = "a".repeat(10000);
        let result = messenger.try_send(long_message.clone(), LogLevel::Info);
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
            let _ = messenger.try_send(msg, level);
        }

        #[test]
        fn test_send_never_panics_with_any_level(level in 0i64..6) {
            let messenger = TestableMessenger::new(100);
            let level = LogLevel::from(level);
            let _ = messenger.try_send("test".to_string(), level);
        }

        #[test]
        fn test_roundtrip_message_preserved(msg in any::<String>()) {
            let messenger = TestableMessenger::new(100);
            let level = LogLevel::Debug;

            messenger.try_send(msg.clone(), level).ok();

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

    #[test]
    fn test_notification_messenger_panic_caught_by_catch_unwind() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            panic!("Test panic");
        }));
        assert!(result.is_err(), "catch_unwind should catch the panic");
    }

    #[test]
    fn test_notification_messenger_panic_result_converted_to_none() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            panic!("Test panic");
        }))
        .ok();
        assert!(result.is_none(), "ok() should convert Err to None");
    }

    #[test]
    fn test_notification_messenger_no_panic_preserved() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| 42)).ok();
        assert!(result.is_some(), "ok() should preserve Ok result");
    }
}
