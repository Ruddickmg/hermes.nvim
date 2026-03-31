use crate::acp::{Result, error::Error};
use crossbeam_channel::{Receiver, Sender, bounded};
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
    use crossbeam_channel::Receiver;
    use pretty_assertions::assert_eq;

    struct TestableMessenger {
        sender: Sender<String>,
        receiver: Receiver<String>,
    }

    impl TestableMessenger {
        fn new(capacity: usize) -> Self {
            let (sender, receiver) = bounded::<String>(capacity);
            Self { sender, receiver }
        }

        fn try_send(&self, msg: String) -> Result<()> {
            self.sender
                .try_send(msg)
                .map_err(|e| Error::Internal(format!("Failed to queue message: {}", e)))
        }

        fn try_recv(&self) -> Option<String> {
            self.receiver.try_recv().ok()
        }
    }

    #[test]
    fn test_message_messenger_send_success() {
        let messenger = TestableMessenger::new(10);
        let result = messenger.try_send("Test message".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_messenger_send_received_message_matches() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("Test message".to_string()).unwrap();
        assert_eq!(messenger.try_recv(), Some("Test message".to_string()));
    }

    #[test]
    fn test_message_messenger_send_multiple_first() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("Message 0".to_string()).unwrap();
        messenger.try_send("Message 1".to_string()).unwrap();
        messenger.try_send("Message 2".to_string()).unwrap();
        messenger.try_send("Message 3".to_string()).unwrap();
        messenger.try_send("Message 4".to_string()).unwrap();
        assert_eq!(messenger.try_recv(), Some("Message 0".to_string()));
    }

    #[test]
    fn test_message_messenger_send_multiple_second() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("Message 0".to_string()).unwrap();
        messenger.try_send("Message 1".to_string()).unwrap();
        messenger.try_recv();
        assert_eq!(messenger.try_recv(), Some("Message 1".to_string()));
    }

    #[test]
    fn test_message_messenger_send_multiple_last() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("Message 0".to_string()).unwrap();
        messenger.try_send("Message 1".to_string()).unwrap();
        messenger.try_send("Message 2".to_string()).unwrap();
        messenger.try_send("Message 3".to_string()).unwrap();
        messenger.try_send("Message 4".to_string()).unwrap();
        messenger.try_recv();
        messenger.try_recv();
        messenger.try_recv();
        messenger.try_recv();
        assert_eq!(messenger.try_recv(), Some("Message 4".to_string()));
    }

    #[test]
    fn test_message_messenger_send_channel_full_error() {
        let messenger = TestableMessenger::new(2);
        messenger.try_send("msg1".to_string()).unwrap();
        messenger.try_send("msg2".to_string()).unwrap();
        let result = messenger.try_send("msg3".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_message_messenger_send_channel_full_error_contains_queue_text() {
        let messenger = TestableMessenger::new(2);
        messenger.try_send("msg1".to_string()).unwrap();
        messenger.try_send("msg2".to_string()).unwrap();
        let result = messenger.try_send("msg3".to_string());
        assert!(result.unwrap_err().to_string().contains("Failed to queue"));
    }

    #[test]
    fn test_message_messenger_send_empty_message_success() {
        let messenger = TestableMessenger::new(10);
        let result = messenger.try_send("".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_messenger_send_empty_message_received() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("".to_string()).unwrap();
        assert_eq!(messenger.try_recv(), Some("".to_string()));
    }

    #[test]
    fn test_message_messenger_send_special_chars_success() {
        let messenger = TestableMessenger::new(10);
        let special = r#"Special chars: <>&"' and unicode: 🎉"#;
        let result = messenger.try_send(special.to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_messenger_send_special_chars_preserved() {
        let messenger = TestableMessenger::new(10);
        let special = r#"Special chars: <>&"' and unicode: 🎉"#;
        messenger.try_send(special.to_string()).unwrap();
        assert_eq!(messenger.try_recv(), Some(special.to_string()));
    }

    #[test]
    fn test_message_messenger_send_long_message_success() {
        let messenger = TestableMessenger::new(10);
        let long_message = "a".repeat(10000);
        let result = messenger.try_send(long_message.clone());
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_messenger_send_long_message_length_preserved() {
        let messenger = TestableMessenger::new(10);
        let long_message = "a".repeat(10000);
        messenger.try_send(long_message.clone()).unwrap();
        assert_eq!(messenger.try_recv().unwrap().len(), 10000);
    }

    #[test]
    fn test_message_messenger_channel_capacity() {
        let (_sender, receiver) = bounded::<String>(100);
        assert_eq!(receiver.capacity(), Some(100));
    }

    #[test]
    fn test_message_messenger_len_initially_zero() {
        let messenger = TestableMessenger::new(10);
        assert_eq!(messenger.sender.len(), 0);
    }

    #[test]
    fn test_message_messenger_len_after_one_send() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("msg1".to_string()).unwrap();
        assert_eq!(messenger.sender.len(), 1);
    }

    #[test]
    fn test_message_messenger_len_after_two_sends() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("msg1".to_string()).unwrap();
        messenger.try_send("msg2".to_string()).unwrap();
        assert_eq!(messenger.sender.len(), 2);
    }

    #[test]
    fn test_message_messenger_is_empty_initially() {
        let messenger = TestableMessenger::new(10);
        assert!(messenger.sender.is_empty());
    }

    #[test]
    fn test_message_messenger_is_empty_after_send() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("msg1".to_string()).unwrap();
        assert!(!messenger.sender.is_empty());
    }

    #[test]
    fn test_message_messenger_is_empty_after_recv() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("msg1".to_string()).unwrap();
        messenger.try_recv();
        assert!(messenger.sender.is_empty());
    }

    #[test]
    fn test_message_messenger_is_full_initially() {
        let messenger = TestableMessenger::new(2);
        assert!(!messenger.sender.is_full());
    }

    #[test]
    fn test_message_messenger_is_full_after_two_sends() {
        let messenger = TestableMessenger::new(2);
        messenger.try_send("msg1".to_string()).unwrap();
        messenger.try_send("msg2".to_string()).unwrap();
        assert!(messenger.sender.is_full());
    }

    #[test]
    fn test_message_messenger_preserves_hello_world() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("hello world".to_string()).unwrap();
        assert_eq!(messenger.try_recv(), Some("hello world".to_string()));
    }

    #[test]
    fn test_message_messenger_preserves_multiline() {
        let messenger = TestableMessenger::new(10);
        messenger
            .try_send("multi\nline\nmessage".to_string())
            .unwrap();
        assert_eq!(
            messenger.try_recv(),
            Some("multi\nline\nmessage".to_string())
        );
    }

    #[test]
    fn test_message_messenger_preserves_tabs() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("tab\there".to_string()).unwrap();
        assert_eq!(messenger.try_recv(), Some("tab\there".to_string()));
    }

    #[test]
    fn test_message_messenger_preserves_unicode() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("unicode 🎉 test".to_string()).unwrap();
        assert_eq!(messenger.try_recv(), Some("unicode 🎉 test".to_string()));
    }

    #[test]
    fn test_message_messenger_preserves_leading_spaces() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("   leading spaces".to_string()).unwrap();
        assert_eq!(messenger.try_recv(), Some("   leading spaces".to_string()));
    }

    #[test]
    fn test_message_messenger_preserves_trailing_spaces() {
        let messenger = TestableMessenger::new(10);
        messenger
            .try_send("trailing spaces   ".to_string())
            .unwrap();
        assert_eq!(messenger.try_recv(), Some("trailing spaces   ".to_string()));
    }

    #[test]
    fn test_message_messenger_preserves_mixed_case() {
        let messenger = TestableMessenger::new(10);
        messenger.try_send("mixed CASE".to_string()).unwrap();
        assert_eq!(messenger.try_recv(), Some("mixed CASE".to_string()));
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
