//! Integration tests for notification delivery system
//!
//! These tests verify that notifications from any thread are safely delivered
//! to Neovim's main thread via the NotificationMessenger system.

use hermes::utilities::{LogLevel, NotificationMessenger};
use std::thread;
use std::time::Duration;

/// Test that NotificationMessenger initializes successfully
#[nvim_oxi::test]
fn test_notification_messenger_initializes() {
    let result = NotificationMessenger::default();
    assert!(
        result.is_ok(),
        "Failed to initialize NotificationMessenger: {:?}",
        result.err()
    );
}

/// Test that sending a notification from the main thread works
#[nvim_oxi::test]
fn test_send_notification_from_main_thread() {
    let messenger = NotificationMessenger::default().expect("Failed to create messenger");

    let result = messenger.send(
        "Test notification from main thread".to_string(),
        LogLevel::Info,
    );
    assert!(
        result.is_ok(),
        "Failed to send notification: {:?}",
        result.err()
    );

    // Give AsyncHandle time to process
    std::thread::sleep(Duration::from_millis(100));
}

/// Test that sending from a spawned thread works
#[nvim_oxi::test]
fn test_send_notification_from_spawned_thread() {
    let messenger = NotificationMessenger::default().expect("Failed to create messenger");
    let messenger_clone = messenger.clone();

    let handle = thread::spawn(move || {
        let result = messenger_clone.send("Test from spawned thread".to_string(), LogLevel::Info);
        assert!(result.is_ok());
    });

    handle.join().expect("Thread panicked");

    // Give AsyncHandle time to process
    std::thread::sleep(Duration::from_millis(100));
}

/// Test concurrent sends from multiple threads
#[nvim_oxi::test]
fn test_concurrent_sends_from_multiple_threads() {
    let messenger = NotificationMessenger::default().expect("Failed to create messenger");
    let num_threads = 5;
    let messages_per_thread = 10;

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let messenger = messenger.clone();
            thread::spawn(move || {
                for msg_id in 0..messages_per_thread {
                    let message = format!("Thread {} Message {}", thread_id, msg_id);
                    let result = messenger.send(message, LogLevel::Info);
                    // May fail if channel is full, but should not panic
                    if let Err(e) = result {
                        eprintln!("Failed to send: {}", e);
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Give AsyncHandle time to process all messages
    std::thread::sleep(Duration::from_millis(500));
}

/// Test sending notifications with various log levels
#[nvim_oxi::test]
fn test_send_with_various_log_levels() {
    let messenger = NotificationMessenger::default().expect("Failed to create messenger");

    let levels = vec![
        (LogLevel::Trace, "Trace message"),
        (LogLevel::Debug, "Debug message"),
        (LogLevel::Info, "Info message"),
        (LogLevel::Warn, "Warning message"),
        (LogLevel::Error, "Error message"),
    ];

    for (level, message) in levels {
        let result = messenger.send(message.to_string(), level);
        assert!(result.is_ok(), "Failed to send {:?} notification", level);
    }

    // Give AsyncHandle time to process
    std::thread::sleep(Duration::from_millis(200));
}

/// Test sending notifications with special characters and unicode
#[nvim_oxi::test]
fn test_send_with_special_characters() {
    let messenger = NotificationMessenger::default().expect("Failed to create messenger");

    let test_messages = vec![
        r#"Special chars: <>&"'"#,
        "Unicode: 🎉🎊🎈 ñ 中文",
        "Newlines:\nSecond line",
        "Tabs:\tindented",
    ];

    // Handle the long message separately since it's a String
    let long_message = "Very long message: ".repeat(100);

    for message in test_messages {
        let result = messenger.send(message.to_string(), LogLevel::Info);
        assert!(result.is_ok(), "Failed to send special message");
    }

    let result = messenger.send(long_message, LogLevel::Info);
    assert!(result.is_ok(), "Failed to send long message");

    // Give AsyncHandle time to process
    std::thread::sleep(Duration::from_millis(200));
}

/// Test that NotificationMessenger can be cloned and used from multiple references
#[nvim_oxi::test]
fn test_messenger_clone() {
    let messenger1 = NotificationMessenger::default().expect("Failed to create messenger");
    let messenger2 = messenger1.clone();
    let messenger3 = messenger1.clone();

    // All three should be able to send
    messenger1
        .send("From original".to_string(), LogLevel::Info)
        .ok();
    messenger2
        .send("From clone 1".to_string(), LogLevel::Info)
        .ok();
    messenger3
        .send("From clone 2".to_string(), LogLevel::Info)
        .ok();

    // Give AsyncHandle time to process
    std::thread::sleep(Duration::from_millis(100));
}

/// Test that empty messages are handled gracefully
#[nvim_oxi::test]
fn test_send_empty_message() {
    let messenger = NotificationMessenger::default().expect("Failed to create messenger");

    let result = messenger.send("".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Empty message should not fail");

    // Give AsyncHandle time to process
    std::thread::sleep(Duration::from_millis(50));
}

/// Test error handling when channel is full
/// Note: This test tries to fill the channel, but since it's bounded at 1000,
/// we can send many messages rapidly
#[nvim_oxi::test]
fn test_channel_full_behavior() {
    let messenger = NotificationMessenger::default().expect("Failed to create messenger");

    // Send messages rapidly to potentially fill the channel
    let mut success_count = 0;
    let mut fail_count = 0;

    for i in 0..2000 {
        let result = messenger.send(format!("Message {}", i), LogLevel::Info);
        if result.is_ok() {
            success_count += 1;
        } else {
            fail_count += 1;
        }
    }

    // We should have some successful sends
    assert!(success_count > 0, "No messages were successfully queued");

    // Channel may have filled up, so we might have some failures
    // This is acceptable behavior - we just verify the system doesn't crash

    // Give AsyncHandle time to process
    std::thread::sleep(Duration::from_millis(500));

    println!("Success: {}, Failed: {}", success_count, fail_count);
}
