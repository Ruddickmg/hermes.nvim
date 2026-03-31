//! Integration tests for notification delivery system
//!
//! These tests verify that notifications from any thread are safely delivered
//! to Neovim's main thread via the NotificationMessenger system.

use hermes::utilities::{LogLevel, NotificationMessenger};
use std::thread;

#[nvim_oxi::test]
fn test_notification_messenger_initializes() {
    let result = NotificationMessenger::initialize();
    assert!(
        result.is_ok(),
        "Failed to initialize NotificationMessenger: {:?}",
        result.err()
    );
}

#[nvim_oxi::test]
fn test_send_notification_from_main_thread_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("Test notification".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Failed to send notification");
}

#[nvim_oxi::test]
fn test_send_notification_from_spawned_thread_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let messenger_clone = messenger.clone();
    let result_from_thread: std::sync::Arc<std::sync::Mutex<Option<hermes::acp::Result<()>>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let result_clone = result_from_thread.clone();

    let handle = thread::spawn(move || {
        let result = messenger_clone.send("Test from spawned thread".to_string(), LogLevel::Info);
        *result_clone.lock().unwrap() = Some(result);
    });

    handle.join().expect("Thread panicked");
    let guard = result_from_thread.lock().unwrap();
    assert!(
        guard.as_ref().expect("Should have result").is_ok(),
        "Failed to send from spawned thread"
    );
}

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
}

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
        num_threads,
        "All {} threads should complete",
        num_threads
    );
}

#[nvim_oxi::test]
fn test_send_with_various_log_levels_succeeds() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let levels = vec![
        LogLevel::Trace,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];

    for level in levels {
        let result = messenger.send("Test message".to_string(), level);
        assert!(result.is_ok(), "Send should succeed for all log levels");
    }
}

#[nvim_oxi::test]
fn test_messenger_clone_can_send() {
    let messenger = NotificationMessenger::initialize().expect("Failed to create messenger");
    let messenger2 = messenger.clone();
    let result = messenger2.send("From clone".to_string(), LogLevel::Info);
    assert!(result.is_ok(), "Clone should be able to send");
}

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
