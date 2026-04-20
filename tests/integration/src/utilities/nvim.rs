//! Integration tests for NvimMessenger
//!
//! Tests the NvimMessenger helper which bridges async Tokio runtime with Neovim's synchronous API.
//! These tests verify the actual cross-thread communication flow using wait_for helpers.
use hermes::utilities::{NvimMessenger, NvimRuntime, TransmitToNvim};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::helpers::ui::wait_for;

fn mock_runtime() -> NvimRuntime {
    NvimRuntime::new(Rc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create mock runtime"),
    ))
}

// === Cross-thread communication tests ===

#[nvim_oxi::test]
fn blocking_send_from_thread_reaches_callback() -> nvim_oxi::Result<()> {
    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let callback = move |data: String| -> std::future::Ready<nvim_oxi::Result<()>> {
        received_clone.lock().unwrap().push(data);
        std::future::ready(Ok(()))
    };

    let handler =
        NvimMessenger::initialize(mock_runtime(), callback).expect("Handler should initialize");

    // Spawn thread that sends data
    std::thread::spawn(move || {
        handler
            .blocking_send("test message".to_string())
            .expect("Send should succeed");
    });

    // Wait for callback to receive data
    let data_received = wait_for(
        || received.lock().unwrap().len() == 1,
        Duration::from_millis(500),
    );

    assert!(
        data_received,
        "Callback should receive data from spawned thread"
    );
    Ok(())
}

#[nvim_oxi::test]
fn async_send_from_thread_reaches_callback() -> nvim_oxi::Result<()> {
    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let callback = move |data: String| -> std::future::Ready<nvim_oxi::Result<()>> {
        received_clone.lock().unwrap().push(data);
        std::future::ready(Ok(()))
    };

    let handler =
        NvimMessenger::initialize(mock_runtime(), callback).expect("Handler should initialize");

    // Spawn thread with tokio runtime that sends data asynchronously
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            handler
                .send("async test message".to_string())
                .await
                .expect("Async send should succeed");
        });
    });

    // Wait for callback to receive data
    let data_received = wait_for(
        || received.lock().unwrap().len() == 1,
        Duration::from_millis(500),
    );

    assert!(
        data_received,
        "Callback should receive async data from spawned thread"
    );
    Ok(())
}

#[nvim_oxi::test]
fn cloned_handler_sends_from_thread_reaches_callback() -> nvim_oxi::Result<()> {
    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let callback = move |data: String| -> std::future::Ready<nvim_oxi::Result<()>> {
        received_clone.lock().unwrap().push(data);
        std::future::ready(Ok(()))
    };

    let handler =
        NvimMessenger::initialize(mock_runtime(), callback).expect("Handler should initialize");
    let cloned_handler = handler.clone();

    // Spawn thread that sends data through cloned handler
    std::thread::spawn(move || {
        cloned_handler
            .blocking_send("message through clone".to_string())
            .expect("Send through clone should succeed");
    });

    // Wait for callback to receive data
    let data_received = wait_for(
        || received.lock().unwrap().len() == 1,
        Duration::from_millis(500),
    );

    assert!(
        data_received,
        "Cloned handler should trigger original callback from thread"
    );
    Ok(())
}

#[nvim_oxi::test]
fn multiple_sends_from_thread_all_received() -> nvim_oxi::Result<()> {
    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let callback = move |data: String| -> std::future::Ready<nvim_oxi::Result<()>> {
        received_clone.lock().unwrap().push(data);
        std::future::ready(Ok(()))
    };

    let handler =
        NvimMessenger::initialize(mock_runtime(), callback).expect("Handler should initialize");

    // Spawn thread that sends multiple messages
    std::thread::spawn(move || {
        for i in 0..3 {
            handler
                .blocking_send(format!("message {}", i))
                .expect("Send should succeed");
        }
    });

    // Wait for all 3 messages to be received
    let all_received = wait_for(
        || received.lock().unwrap().len() == 3,
        Duration::from_millis(500),
    );

    assert!(
        all_received,
        "All three messages from thread should be received"
    );
    Ok(())
}

#[nvim_oxi::test]
fn preserves_order_across_thread_boundary() -> nvim_oxi::Result<()> {
    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let callback = move |data: String| -> std::future::Ready<nvim_oxi::Result<()>> {
        received_clone.lock().unwrap().push(data);
        std::future::ready(Ok(()))
    };

    let handler =
        NvimMessenger::initialize(mock_runtime(), callback).expect("Handler should initialize");

    // Spawn thread that sends messages in specific order
    std::thread::spawn(move || {
        let messages = vec!["first", "second", "third"];
        for msg in &messages {
            handler
                .blocking_send(msg.to_string())
                .expect("Send should succeed");
        }
    });

    // Wait for all 3 messages and verify order
    let correct_order = wait_for(
        || {
            let data = received.lock().unwrap();
            data.len() == 3 && data[0] == "first" && data[1] == "second" && data[2] == "third"
        },
        Duration::from_millis(500),
    );

    assert!(
        correct_order,
        "Messages from thread should be received in order"
    );
    Ok(())
}

#[nvim_oxi::test]
fn numeric_type_from_thread_reaches_callback() -> nvim_oxi::Result<()> {
    let received = Arc::new(Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let callback = move |data: i32| -> std::future::Ready<nvim_oxi::Result<()>> {
        received_clone.lock().unwrap().push(data);
        std::future::ready(Ok(()))
    };

    let handler =
        NvimMessenger::initialize(mock_runtime(), callback).expect("Handler should initialize");

    // Spawn thread that sends numeric data
    std::thread::spawn(move || {
        handler.blocking_send(42).expect("Send should succeed");
    });

    // Wait for callback to receive the numeric value
    let value_received = wait_for(
        || received.lock().unwrap().first() == Some(&42),
        Duration::from_millis(500),
    );

    assert!(
        value_received,
        "Numeric value from thread should reach callback"
    );
    Ok(())
}

#[nvim_oxi::test]
fn callback_error_is_handled_gracefully() -> nvim_oxi::Result<()> {
    // Test that when the callback returns an error, it's logged but not propagated
    // This covers the error handling path in src/utilities/nvim.rs:29
    // The error!() macro logs the error, and the send operation should still succeed

    // Callback that returns an error - error should be logged via error!()
    let callback = move |_data: String| -> std::future::Ready<nvim_oxi::Result<()>> {
        std::future::ready(Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(
            "Test callback error".to_string(),
        ))))
    };

    let handler =
        NvimMessenger::initialize(mock_runtime(), callback).expect("Handler should initialize");

    // Spawn thread that sends data - this should succeed even though callback returns error
    let send_result = std::thread::spawn(move || handler.blocking_send("test message".to_string()))
        .join()
        .expect("Thread should not panic");

    // Verify that send succeeded (error was logged, not propagated)
    assert!(
        send_result.is_ok(),
        "Send should succeed even when callback returns error"
    );
    Ok(())
}

#[nvim_oxi::test]
fn callback_panic_is_caught_without_crashing() -> nvim_oxi::Result<()> {
    // Test that when the callback panics, it's caught via catch_unwind
    // and logged via inspect_err on line 32 of src/utilities/nvim.rs
    // The process should NOT crash, and send should still succeed

    // Callback that panics - this tests the catch_unwind protection
    let callback = move |_data: String| -> std::future::Ready<nvim_oxi::Result<()>> {
        panic!("intentional test panic in NvimMessenger callback");
    };

    let handler =
        NvimMessenger::initialize(mock_runtime(), callback).expect("Handler should initialize");

    // Spawn thread that sends data - this should succeed even though callback panics
    let send_result =
        std::thread::spawn(move || handler.blocking_send("trigger panic".to_string()))
            .join()
            .expect("Thread should not panic");

    // Verify that send succeeded (panic was caught and logged, not propagated)
    assert!(
        send_result.is_ok(),
        "Send should succeed even when callback panics"
    );
    Ok(())
}
