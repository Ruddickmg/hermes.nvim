//! Integration tests for setup API function
//!
//! Tests verify that api::setup correctly updates plugin state when called with various configs.

use hermes::nvim::{
    api::setup,
    configuration::{
        BufferConfigPartial, ClientConfigPartial, LogConfigPartial, LogFileConfigPartial,
        PermissionsPartial, SetupArgs, TerminalConfigPartial,
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

    // Verify changes
    let state = plugin_state.blocking_lock();
    assert!(!state.config.permissions.fs_write_access); // changed to false
    assert!(!state.config.permissions.terminal_access); // changed to false
    assert!(state.config.permissions.fs_read_access); // still true (default)
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
    assert!(state.config.terminal.delete); // changed to true
    assert!(!state.config.terminal.hidden); // changed to false
    assert!(!state.config.terminal.enabled); // changed to false
    assert!(!state.config.terminal.buffered); // changed to false
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
    assert!(state.config.buffer.auto_save); // changed to true
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
    assert_eq!(state.config.log.level, hermes::utilities::LogLevel::Debug);
    assert_eq!(
        state.config.log.local_list,
        hermes::utilities::LogLevel::Info
    );
    assert_eq!(state.config.log.message, hermes::utilities::LogLevel::Warn);
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

    // Verify: fs_write_access should still be false from first call
    let state = plugin_state.blocking_lock();
    assert!(!state.config.permissions.fs_write_access); // from first call
    assert!(!state.config.permissions.terminal_access); // from second call
    assert!(state.config.permissions.fs_read_access); // never changed, still default
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

    // Initially, log.file should be None
    {
        let state = plugin_state.blocking_lock();
        assert!(state.config.log.file.is_none());
    }

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

    // Verify file config was created
    let state = plugin_state.blocking_lock();
    assert!(state.config.log.file.is_some());
    let file_config = state.config.log.file.as_ref().unwrap();
    assert!(file_config.enabled);
    assert_eq!(file_config.path, "/tmp/test.log");
    assert_eq!(file_config.level, hermes::utilities::LogLevel::Warn);
    Ok(())
}
