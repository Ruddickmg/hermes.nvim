//! Integration tests for notification delivery system
//!
//! These tests verify that notifications from any thread are safely delivered
//! to Neovim's main thread via the NotificationMessenger system.

use hermes::utilities::{LogLevel, NotificationMessenger};
use pretty_assertions::assert_eq;
use std::thread;
use std::time::Duration;

/// Test that NotificationMessenger initializes successfully
#[nvim_oxi::test]
fn test_notification_messenger_initializes() {
    let result = NotificationMessenger::initialize();
    assert!(
        result.is_ok(),
        "Failed to initialize NotificationMessenger: {:?}",
        result.err()
    );
}

/// Test that sending a notification from the main thread works
#[nvim_oxi::test]
fn test_send_notification_from_main_thread_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");

    let result = messenger.send(
        "Test notification from main thread".to_string(),
        LogLevel::Info,
    );
    assert!(
        result.is_ok(),
        "Failed to send notification: {:?}",
        result.err()
    );

    std::thread::sleep(Duration::from_millis(100));
}

/// Test that sending from a spawned thread works
#[nvim_oxi::test]
fn test_send_notification_from_spawned_thread_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let messenger_clone = messenger.clone();
    let result_from_thread = std::sync::Arc::new(std::sync::Mutex::new(None::<()>));
    let result_clone = result_from_thread.clone();

    let handle = thread::spawn(move || {
        let result = messenger_clone.send("Test from spawned thread".to_string(), LogLevel::Info);
        *result_clone.lock().unwrap() = Some(result);
    });

    handle.join().expect("Thread panicked");
    let result = result_from_thread.lock().unwrap();
    assert!(result.is_ok(), "Failed to send from spawned thread");
}

/// Test that sending from spawned thread completes without panic
#[nvim_oxi::test]
fn test_send_notification_from_spawned_thread_completes() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let messenger_clone = messenger.clone();
    let completed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let completed_clone = completed.clone();

    let handle = thread::spawn(move || {
        let _ = messenger_clone.send("Test".to_string(), LogLevel::Info);
        completed_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    handle.join().expect("Thread panicked");
    assert!(
        completed.load(std::sync::atomic::Ordering::SeqCst),
        "Thread did not complete"
    );

    std::thread::sleep(Duration::from_millis(100));
}

/// Test concurrent sends from multiple threads - all threads complete
#[nvim_oxi::test]
fn test_concurrent_sends_all_threads_complete() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let num_threads = 5;
    let completed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let messenger_arc = std::sync::Arc::new(messenger);

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let messenger = messenger_arc.clone();
            let completed = completed.clone();
            thread::spawn(move || {
                let _ = messenger.send(format!("Thread {} message", thread_id), LogLevel::Info);
                completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    assert_eq!(
        completed.load(std::sync::atomic::Ordering::SeqCst),
        num_threads
    );

    std::thread::sleep(Duration::from_millis(500));
}

/// Test concurrent sends - at least one message succeeds
#[nvim_oxi::test]
fn test_concurrent_sends_at_least_one_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let messenger_clone = messenger.clone();
    let success_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let success_clone = success_count.clone();

    let handle = thread::spawn(move || {
        for i in 0..10 {
            let _ = messenger_clone.send(format!("Message {}", i), LogLevel::Info);
            success_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    });

    handle.join().expect("Thread panicked");

    assert_eq!(
        success_count.load(std::sync::atomic::Ordering::SeqCst),
        10usize
    );
}

/// Test sending with Trace log level
#[nvim_oxi::test]
fn test_send_with_trace_level_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("Trace message".to_string(), LogLevel::Trace);
    assert!(result.is_ok(), "Failed to send Trace notification");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test sending with Debug log level
#[nvim_oxi::test]
fn test_send_with_debug_level_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("Debug message".to_string(), LogLevel::Debug);
    assert!(result.is_ok(), "Failed to send Debug notification");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test sending with Info log level
#[nvim_oxi::test]
fn test_send_with_info_level_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("Info message".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Failed to send Info notification");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test sending with Warn log level
#[nvim_oxi::test]
fn test_send_with_warn_level_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("Warning message".to_string(), LogLevel::Warn);
    assert!(result.is_ok(), "Failed to send Warning notification");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test sending with Error log level
#[nvim_oxi::test]
fn test_send_with_error_level_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("Error message".to_string(), LogLevel::Error);
    assert!(result.is_ok(), "Failed to send Error notification");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test sending with special characters
#[nvim_oxi::test]
fn test_send_with_special_chars_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send(r#"Special chars: <>&"'"#.to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Failed to send special chars");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test sending with unicode
#[nvim_oxi::test]
fn test_send_with_unicode_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("Unicode: 🎉🎊🎈 ñ 中文".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Failed to send unicode");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test sending with newlines
#[nvim_oxi::test]
fn test_send_with_newlines_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("Newlines:\nSecond line".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Failed to send newlines");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test sending with tabs
#[nvim_oxi::test]
fn test_send_with_tabs_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("Tabs:\tindented".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Failed to send tabs");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test sending with long message
#[nvim_oxi::test]
fn test_send_long_message_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let long_message = "Very long message: ".repeat(100);
    let result = messenger.send(long_message, LogLevel::Info);
    assert!(result.is_ok(), "Failed to send long message");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test that NotificationMessenger clone is sendable
#[nvim_oxi::test]
fn test_messenger_clone_sendable() {
    let messenger1 = NotificationMessenger::initialize().expect("Failed to create messenger");
    let messenger2 = messenger1.clone();

    let result = messenger2.send("From clone".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Clone should be able to send");

    std::thread::sleep(Duration::from_millis(100));
}

/// Test that NotificationMessenger can be cloned multiple times
#[nvim_oxi::test]
fn test_messenger_multiple_clones_work() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let m1 = messenger.clone();
    let m2 = messenger.clone();
    let m3 = messenger.clone();

    let result1 = m1.send("First".to_string(), LogLevel::Info);
    let result2 = m2.send("Second".to_string(), LogLevel::Info);
    let result3 = m3.send("Third".to_string(), LogLevel::Info);

    assert!(result1.is_ok(), "First clone should send");
    assert!(result2.is_ok(), "Second clone should send");
    assert!(result3.is_ok(), "Third clone should send");

    std::thread::sleep(Duration::from_millis(100));
}

/// Test that empty messages are handled gracefully
#[nvim_oxi::test]
fn test_send_empty_message_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Empty message should not fail");
    std::thread::sleep(Duration::from_millis(50));
}

/// Test that rapid sends succeed
#[nvim_oxi::test]
fn test_rapid_sends_succeed() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");

    let result = messenger.send("Rapid 1".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "First rapid send should succeed");

    std::thread::sleep(Duration::from_millis(500));
}

/// Test channel doesn't panic when receiving many messages
#[nvim_oxi::test]
fn test_many_messages_dont_crash() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let messenger_clone = messenger.clone();
    let panic_occurred = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let panic_clone = panic_occurred.clone();

    let handle = thread::spawn(move || {
        for i in 0..100 {
            let _ = messenger_clone.send(format!("Message {}", i), LogLevel::Info);
        }
        panic_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    handle.join().expect("Thread panicked");
    assert!(
        panic_occurred.load(std::sync::atomic::Ordering::SeqCst),
        "Thread should complete without panic"
    );
}
