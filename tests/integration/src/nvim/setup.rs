//! Integration tests for setup API function
//!
//! Tests verify that api::setup correctly updates plugin state when called with various configs.

use hermes::nvim::{
    api::setup,
    configuration::{
        BufferConfigPartial, ClientConfigPartial, LogConfigPartial, LogFileConfigPartial,
        Permissions, PermissionsPartial, SetupArgs, TerminalConfig, TerminalConfigPartial,
    },
    state::PluginState,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test: setup() function can be created
#[nvim_oxi::test]
fn setup_function_can_be_created() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let _setup_fn = setup(plugin_state);
    Ok(())
}

/// Test: setup() updates permissions correctly
#[nvim_oxi::test]
fn setup_updates_permissions_correctly() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(plugin_state.clone());

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    let config = ClientConfigPartial {
        permissions: Some(PermissionsPartial {
            fs_write_access: Some(false),
            fs_read_access: None,
            terminal_access: Some(false),
            can_request_permissions: None,
            allow_notifications: None,
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    // Verify changes using single assertion comparing expected vs actual
    let state = plugin_state.blocking_lock();
    let expected = Permissions {
        fs_write_access: false,        // changed
        fs_read_access: true,          // default preserved
        terminal_access: false,        // changed
        can_request_permissions: true, // default preserved
        allow_notifications: true,     // default preserved
    };
    assert_eq!(state.config.permissions, expected);
    Ok(())
}

/// Test: setup() updates terminal config correctly
#[nvim_oxi::test]
fn setup_updates_terminal_config_correctly() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(plugin_state.clone());

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    let config = ClientConfigPartial {
        terminal: Some(TerminalConfigPartial {
            delete: Some(true),
            hidden: Some(false),
            enabled: Some(false),
            buffered: Some(false),
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    let expected = TerminalConfig {
        delete: true,    // changed
        hidden: false,   // changed
        enabled: false,  // changed
        buffered: false, // changed
    };
    assert_eq!(state.config.terminal, expected);
    Ok(())
}

/// Test: setup() updates buffer config correctly
#[nvim_oxi::test]
fn setup_updates_buffer_config_correctly() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(plugin_state.clone());

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    let config = ClientConfigPartial {
        buffer: Some(BufferConfigPartial {
            auto_save: Some(true),
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert!(state.config.buffer.auto_save); // Single assertion
    Ok(())
}

/// Test: setup() updates log config correctly
#[nvim_oxi::test]
fn setup_updates_log_config_correctly() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(plugin_state.clone());

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    let config = ClientConfigPartial {
        log: Some(LogConfigPartial {
            file: None,
            level: Some(hermes::utilities::LogLevel::Debug),
            local_list: Some(hermes::utilities::LogLevel::Info),
            message: Some(hermes::utilities::LogLevel::Warn),
            notification: None,
            quick_fix_list: None,
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert_eq!(
        (
            state.config.log.level,
            state.config.log.local_list,
            state.config.log.message
        ),
        (
            hermes::utilities::LogLevel::Debug,
            hermes::utilities::LogLevel::Info,
            hermes::utilities::LogLevel::Warn
        )
    );
    Ok(())
}

/// Test: Multiple setup calls merge correctly
#[nvim_oxi::test]
fn setup_partial_update_preserves_existing_values() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(plugin_state.clone());

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    // First call: set fs_write_access to false
    let config1 = ClientConfigPartial {
        permissions: Some(PermissionsPartial {
            fs_write_access: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };
    func.call(SetupArgs(Some(config1)))
        .expect("Failed to call setup 1");

    // Second call: set terminal_access to false (permissions should keep previous values)
    let config2 = ClientConfigPartial {
        permissions: Some(PermissionsPartial {
            terminal_access: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };
    func.call(SetupArgs(Some(config2)))
        .expect("Failed to call setup 2");

    // Verify using single assertion comparing expected vs actual
    let state = plugin_state.blocking_lock();
    let expected = Permissions {
        fs_write_access: false,        // from first call
        fs_read_access: true,          // default preserved
        terminal_access: false,        // from second call
        can_request_permissions: true, // default preserved
        allow_notifications: true,     // default preserved
    };
    assert_eq!(state.config.permissions, expected);
    Ok(())
}

/// Test: setup() creates log file config when it doesn't exist
#[nvim_oxi::test]
fn setup_creates_log_file_config_when_none() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(plugin_state.clone());

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    // Setup with file config
    let config = ClientConfigPartial {
        log: Some(LogConfigPartial {
            file: Some(LogFileConfigPartial {
                enabled: Some(true),
                path: Some("/tmp/test.log".to_string()),
                level: Some(hermes::utilities::LogLevel::Warn),
                max_size: None,
                max_files: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    // Verify file config was created using single assertion
    let state = plugin_state.blocking_lock();
    let file_config = state
        .config
        .log
        .file
        .as_ref()
        .expect("File config should exist");
    assert!(
        file_config.enabled
            && file_config.path == "/tmp/test.log"
            && file_config.level == hermes::utilities::LogLevel::Warn
    );
    Ok(())
}
