//! Integration tests for message delivery system
//!
//! These tests verify that messages from any thread are safely delivered
//! to Neovim's main thread via the MessageMessenger system, including
//! the deferred scheduling via vim.schedule.

use hermes::utilities::MessageMessenger;
use std::thread;
use std::time::Duration;

use crate::helpers::ui::wait_for;

/// Helper to get Neovim messages via :messages
fn get_nvim_messages() -> String {
    let opts = nvim_oxi::api::opts::ExecOpts::builder()
        .output(true)
        .build();
    nvim_oxi::api::exec2("messages", &opts)
        .ok()
        .flatten()
        .map(|s| s.to_string())
        .unwrap_or_default()
}

#[nvim_oxi::test]
fn message_messenger_initializes_successfully() {
    let result = MessageMessenger::initialize();
    assert!(
        result.is_ok(),
        "Failed to initialize MessageMessenger: {:?}",
        result.err()
    );
}

#[nvim_oxi::test]
fn send_from_main_thread_succeeds() {
    let messenger = MessageMessenger::initialize().expect("Failed to create messenger");
    let result = messenger.send("test_msg_main_thread".to_string());
    assert!(result.is_ok(), "Failed to send message from main thread");
}

#[nvim_oxi::test]
fn send_from_spawned_thread_succeeds() {
    let messenger = MessageMessenger::initialize().expect("Failed to create messenger");
    let messenger_clone = messenger.clone();

    let result = thread::spawn(move || messenger_clone.send("test_msg_spawned_thread".to_string()))
        .join()
        .expect("Thread panicked");

    assert!(result.is_ok(), "Failed to send message from spawned thread");
}

#[nvim_oxi::test]
fn message_delivered_to_neovim_via_schedule() -> nvim_oxi::Result<()> {
    // Clear existing messages
    nvim_oxi::api::command("messages clear")?;

    let messenger = MessageMessenger::initialize().expect("Failed to create messenger");
    let unique_msg = "hermes_integ_test_schedule_delivery_12345";

    // Send from a thread (exercises the full path: channel -> AsyncHandle -> schedule -> echomsg)
    let messenger_clone = messenger.clone();
    let msg = unique_msg.to_string();
    thread::spawn(move || {
        messenger_clone.send(msg).expect("Send should succeed");
    })
    .join()
    .expect("Thread panicked");

    // Wait for the message to appear in :messages
    // The message goes through: crossbeam channel -> AsyncHandle callback -> vim.schedule -> echomsg
    let delivered = wait_for(
        || get_nvim_messages().contains(unique_msg),
        Duration::from_millis(1000),
    );

    assert!(
        delivered,
        "Message should be delivered to Neovim via vim.schedule"
    );
    Ok(())
}

#[nvim_oxi::test]
fn multiple_messages_delivered_via_schedule() -> nvim_oxi::Result<()> {
    nvim_oxi::api::command("messages clear")?;

    let messenger = MessageMessenger::initialize().expect("Failed to create messenger");
    let messenger_clone = messenger.clone();

    thread::spawn(move || {
        for i in 0..3 {
            messenger_clone
                .send(format!("hermes_batch_msg_{}", i))
                .expect("Send should succeed");
        }
    })
    .join()
    .expect("Thread panicked");

    let all_delivered = wait_for(
        || {
            let msgs = get_nvim_messages();
            msgs.contains("hermes_batch_msg_0")
                && msgs.contains("hermes_batch_msg_1")
                && msgs.contains("hermes_batch_msg_2")
        },
        Duration::from_millis(1000),
    );

    assert!(
        all_delivered,
        "All batch messages should be delivered via vim.schedule"
    );
    Ok(())
}
