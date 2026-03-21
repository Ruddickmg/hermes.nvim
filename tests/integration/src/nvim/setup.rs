use hermes::{
    nvim::{
        api::SetupArgs,
        configuration::{
            BufferConfigPartial, ClientConfigPartial, LogConfigPartial, LogFileConfigPartial,
            LogTargetConfigPartial,
        },
        PluginState,
    },
    setup,
};
use nvim_oxi::{self, lua::Poppable};
use std::sync::{Arc, Mutex};

/// Test: setup() updates permissions correctly
#[nvim_oxi::test]
fn setup_updates_permissions_correctly() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    let config = ClientConfigPartial {
        permissions: Some(hermes::nvim::configuration::PermissionsPartial {
            fs_write_access: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert!(!state.config.permissions.fs_write_access); // Single assertion
    Ok(())
}

/// Test: setup() updates buffer config correctly
#[nvim_oxi::test]
fn setup_updates_buffer_config_correctly() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

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
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    let config = ClientConfigPartial {
        log: Some(LogConfigPartial {
            file: None,
            stdio: Some(LogTargetConfigPartial {
                level: Some(hermes::utilities::LogLevel::Debug),
                format: None,
            }),
            notification: Some(LogTargetConfigPartial {
                level: Some(hermes::utilities::LogLevel::Info),
                format: None,
            }),
            message: Some(LogTargetConfigPartial {
                level: Some(hermes::utilities::LogLevel::Warn),
                format: None,
            }),
            quickfix: None,
            local_list: None,
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert_eq!(
        state.config.log.stdio.level,
        hermes::utilities::LogLevel::Debug
    ); // Stdio level updated
    assert_eq!(
        state.config.log.notification.level,
        hermes::utilities::LogLevel::Info
    ); // Notification level updated
    assert_eq!(
        state.config.log.message.level,
        hermes::utilities::LogLevel::Warn
    ); // Message level updated
    Ok(())
}

/// Test: setup() merges configs on multiple calls
#[nvim_oxi::test]
fn setup_merges_configs_on_multiple_calls() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    // First call: set permissions
    func.call(SetupArgs(Some(ClientConfigPartial {
        permissions: Some(hermes::nvim::configuration::PermissionsPartial {
            fs_write_access: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    })))
    .expect("Failed to call setup first time");

    // Second call: set buffer config (should keep permissions)
    func.call(SetupArgs(Some(ClientConfigPartial {
        buffer: Some(BufferConfigPartial {
            auto_save: Some(true),
        }),
        ..Default::default()
    })))
    .expect("Failed to call setup second time");

    let state = plugin_state.blocking_lock();
    assert!(!state.config.permissions.fs_write_access); // Preserved
    assert!(state.config.buffer.auto_save); // Updated
    Ok(())
}

/// Test: setup() works with empty config
#[nvim_oxi::test]
fn setup_works_with_empty_config() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    // Empty config
    func.call(SetupArgs(Some(ClientConfigPartial::default())))
        .expect("Failed to call setup");

    // Should use defaults
    let state = plugin_state.blocking_lock();
    assert!(state.config.permissions.fs_read_access); // Default true
    Ok(())
}

/// Test: setup() works with None
#[nvim_oxi::test]
fn setup_works_with_none() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    // None config
    func.call(SetupArgs(None)).expect("Failed to call setup");

    // Should use defaults
    let state = plugin_state.blocking_lock();
    assert!(state.config.permissions.fs_read_access); // Default true
    Ok(())
}

/// Test: setup() updates log file config
#[nvim_oxi::test]
fn setup_updates_log_file_config() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    let config = ClientConfigPartial {
        log: Some(LogConfigPartial {
            file: Some(LogFileConfigPartial {
                enabled: Some(true),
                path: Some("/tmp/test.log".to_string()),
                level: Some(hermes::utilities::LogLevel::Warn),
                format: None,
                max_size: Some(1024 * 1024),
                max_files: Some(5),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    let file_config = state
        .config
        .log
        .file
        .as_ref()
        .expect("File config should exist");
    assert!(file_config.enabled);
    assert_eq!(file_config.path, "/tmp/test.log");
    assert_eq!(file_config.level, hermes::utilities::LogLevel::Warn);
    Ok(())
}

/// Test: setup() updates log target format
#[nvim_oxi::test]
fn setup_updates_log_target_format() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    let config = ClientConfigPartial {
        log: Some(LogConfigPartial {
            stdio: Some(LogTargetConfigPartial {
                level: Some(hermes::utilities::LogLevel::Info),
                format: Some(hermes::utilities::logging::LogFormat::Json),
            }),
            notification: Some(LogTargetConfigPartial {
                level: Some(hermes::utilities::LogLevel::Error),
                format: Some(hermes::utilities::logging::LogFormat::Pretty),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert_eq!(
        state.config.log.stdio.format,
        Some(hermes::utilities::logging::LogFormat::Json)
    );
    assert_eq!(
        state.config.log.notification.format,
        Some(hermes::utilities::logging::LogFormat::Pretty)
    );
    Ok(())
}
