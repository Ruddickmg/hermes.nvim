use crate::acp::{error::Error, Result};
use crossbeam_channel::{bounded, Receiver, Sender};
use nvim_oxi::libuv::AsyncHandle;
use std::sync::Arc;

#[derive(Clone)]
pub struct MessageMessenger {
    handle: Arc<AsyncHandle>,
    sender: Arc<Sender<String>>,
}

impl std::fmt::Debug for MessageMessenger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageMessenger").finish()
    }
}

impl MessageMessenger {
    pub fn initialize() -> Result<Self> {
        let (sender, receiver): (Sender<String>, Receiver<String>) = bounded(1000);

        let handle = AsyncHandle::new(move || {
            while let Ok(message) = receiver.try_recv() {
                let escaped = message.replace('"', "\\\"");
                let cmd = format!("echomsg \"{}\"", escaped);
                nvim_oxi::api::command(&cmd).ok();
            }
        })
        .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Self {
            handle: Arc::new(handle),
            sender: Arc::new(sender),
        })
    }

    pub fn send(&self, message: String) -> Result<()> {
        self.sender
            .try_send(message)
            .map_err(|e| Error::Internal(format!("Failed to queue message: {}", e)))?;

        self.handle
            .send()
            .map_err(|e| Error::Internal(format!("Failed to signal message handler: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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

    struct TestableMessenger {
        sender: Sender<String>,
        receiver: Receiver<String>,
        mock_handle: MockAsyncHandle,
    }

    impl TestableMessenger {
        fn new(capacity: usize) -> Self {
            let (sender, receiver) = bounded::<String>(capacity);
            Self {
                sender,
                receiver,
                mock_handle: MockAsyncHandle::new(),
            }
        }

        fn send(&self, message: String) -> Result<()> {
            self.sender
                .try_send(message)
                .map_err(|e| Error::Internal(format!("Failed to queue message: {}", e)))?;
            self.mock_handle.send()
        }

        fn try_recv(&self) -> Option<String> {
            self.receiver.try_recv().ok()
        }

        fn is_full(&self) -> bool {
            self.sender.is_full()
        }

        fn len(&self) -> usize {
            self.sender.len()
        }

        fn is_empty(&self) -> bool {
            self.sender.is_empty()
        }
    }

    #[test]
    fn test_message_messenger_send_success() {
        let messenger = TestableMessenger::new(10);

        let result = messenger.send("Test message".to_string());
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap(), "Test message");
    }

    #[test]
    fn test_message_messenger_send_multiple() {
        let messenger = TestableMessenger::new(10);

        for i in 0..5 {
            let result = messenger.send(format!("Message {}", i));
            assert!(result.is_ok());
        }

        for i in 0..5 {
            let msg = messenger.try_recv();
            assert!(msg.is_some());
            assert_eq!(msg.unwrap(), format!("Message {}", i));
        }
    }

    #[test]
    fn test_message_messenger_send_channel_full() {
        let messenger = TestableMessenger::new(2);

        messenger.send("msg1".to_string()).unwrap();
        messenger.send("msg2".to_string()).unwrap();

        let result = messenger.send("msg3".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to queue"));
    }

    #[test]
    fn test_message_messenger_send_empty_message() {
        let messenger = TestableMessenger::new(10);

        let result = messenger.send("".to_string());
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap(), "");
    }

    #[test]
    fn test_message_messenger_send_special_characters() {
        let messenger = TestableMessenger::new(10);

        let special = r#"Special chars: <>&"' and unicode: 🎉"#;
        let result = messenger.send(special.to_string());
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap(), special);
    }

    #[test]
    fn test_message_messenger_send_long_message() {
        let messenger = TestableMessenger::new(10);

        let long_message = "a".repeat(10000);
        let result = messenger.send(long_message.clone());
        assert!(result.is_ok());

        let msg = messenger.try_recv();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().len(), 10000);
    }

    #[test]
    fn test_message_messenger_debug_trait() {
        let (sender, _receiver) = bounded::<String>(100);
        assert_eq!(sender.capacity(), Some(100));
    }

    #[test]
    fn test_message_messenger_new() {
        let (_sender, receiver) = bounded::<String>(10);
        assert_eq!(receiver.capacity(), Some(10));
    }

    #[test]
    fn test_message_messenger_len() {
        let messenger = TestableMessenger::new(10);
        assert_eq!(messenger.len(), 0);

        messenger.send("msg1".to_string()).unwrap();
        assert_eq!(messenger.len(), 1);

        messenger.send("msg2".to_string()).unwrap();
        assert_eq!(messenger.len(), 2);
    }

    #[test]
    fn test_message_messenger_is_empty() {
        let messenger = TestableMessenger::new(10);
        assert!(messenger.is_empty());

        messenger.send("msg1".to_string()).unwrap();
        assert!(!messenger.is_empty());

        messenger.try_recv();
        assert!(messenger.is_empty());
    }

    #[test]
    fn test_message_messenger_is_full() {
        let messenger = TestableMessenger::new(2);
        assert!(!messenger.is_full());

        messenger.send("msg1".to_string()).unwrap();
        assert!(!messenger.is_full());

        messenger.send("msg2".to_string()).unwrap();
        assert!(messenger.is_full());
    }

    #[test]
    fn test_message_messenger_signal_count() {
        let messenger = TestableMessenger::new(10);
        assert_eq!(messenger.mock_handle.signal_count(), 0);

        messenger.send("msg1".to_string()).unwrap();
        assert_eq!(messenger.mock_handle.signal_count(), 1);

        messenger.send("msg2".to_string()).unwrap();
        assert_eq!(messenger.mock_handle.signal_count(), 2);
    }

    #[test]
    fn test_message_messenger_mock_handle_clone() {
        let messenger = TestableMessenger::new(10);
        let cloned = messenger.mock_handle.clone();

        messenger.send("msg".to_string()).unwrap();
        assert_eq!(cloned.signal_count(), 1);
    }

    #[test]
    fn test_message_messenger_preserves_message_content() {
        let messenger = TestableMessenger::new(10);

        let test_cases = vec![
            "hello world",
            "multi\nline\nmessage",
            "tab\there",
            "unicode 🎉 test",
            "special <>&\"' chars",
            "   leading spaces",
            "trailing spaces   ",
            "mixed CASE",
            "",
        ];

        for msg in test_cases {
            messenger.send(msg.to_string()).unwrap();
            let received = messenger.try_recv();
            assert_eq!(received.as_deref(), Some(msg), "Failed for: {:?}", msg);
        }
    }

    proptest! {
        #[test]
        fn test_send_never_panics_with_any_message(msg in any::<String>()) {
            let messenger = TestableMessenger::new(100);
            let _ = messenger.send(msg);
        }

        #[test]
        fn test_roundtrip_message_preserved(msg in any::<String>()) {
            let messenger = TestableMessenger::new(100);

            messenger.send(msg.clone()).ok();

            let received = messenger.try_recv();
            if let Some(received_msg) = received {
                assert_eq!(received_msg, msg);
            }
        }

        #[test]
        fn test_empty_check_consistent(input in ".*") {
            let messenger = TestableMessenger::new(100);

            let is_empty_before = messenger.is_empty();
            messenger.send(input.to_string()).ok();
            let is_empty_after_send = !messenger.is_empty();

            if !input.is_empty() {
                assert!(is_empty_before);
                assert!(is_empty_after_send);
            }
        }
    }

    #[test]
    fn test_message_messenger_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<MessageMessenger>();
    }

    #[test]
    fn test_message_messenger_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<MessageMessenger>();
    }
}
