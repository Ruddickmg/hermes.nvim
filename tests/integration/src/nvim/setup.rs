use hermes::{
    PluginState,
    api::{SetupArgs, setup},
    nvim::configuration::{
        BufferConfigPartial, ClientConfigPartial, LogConfigPartial, LogFileConfigPartial,
        LogTargetConfigPartial,
    },
};
use nvim_oxi;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use tokio::sync::Mutex;

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

/// Test: setup() updates stdio log level correctly
#[nvim_oxi::test]
fn setup_updates_stdio_log_level() -> nvim_oxi::Result<()> {
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
            notification: None,
            message: None,
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert_eq!(
        state.config.log.stdio.level,
        hermes::utilities::LogLevel::Debug
    );
    Ok(())
}

/// Test: setup() updates notification log level correctly
#[nvim_oxi::test]
fn setup_updates_notification_log_level() -> nvim_oxi::Result<()> {
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
            stdio: None,
            notification: Some(LogTargetConfigPartial {
                level: Some(hermes::utilities::LogLevel::Info),
                format: None,
            }),
            message: None,
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert_eq!(
        state.config.log.notification.level,
        hermes::utilities::LogLevel::Info
    );
    Ok(())
}

/// Test: setup() updates message log level correctly
#[nvim_oxi::test]
fn setup_updates_message_log_level() -> nvim_oxi::Result<()> {
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
            stdio: None,
            notification: None,
            message: Some(LogTargetConfigPartial {
                level: Some(hermes::utilities::LogLevel::Warn),
                format: None,
            }),
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert_eq!(
        state.config.log.message.level,
        hermes::utilities::LogLevel::Warn
    );
    Ok(())
}

/// Test: setup() preserves permissions on subsequent calls
#[nvim_oxi::test]
fn setup_preserves_permissions_on_subsequent_calls() -> nvim_oxi::Result<()> {
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
    assert!(!state.config.permissions.fs_write_access);
    Ok(())
}

/// Test: setup() updates buffer config on subsequent calls
#[nvim_oxi::test]
fn setup_updates_buffer_config_on_subsequent_calls() -> nvim_oxi::Result<()> {
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

    // Second call: set buffer config
    func.call(SetupArgs(Some(ClientConfigPartial {
        buffer: Some(BufferConfigPartial {
            auto_save: Some(true),
        }),
        ..Default::default()
    })))
    .expect("Failed to call setup second time");

    let state = plugin_state.blocking_lock();
    assert!(state.config.buffer.auto_save);
    Ok(())
}

/// Test: setup() works with empty config
#[nvim_oxi::test]
fn setup_works_with_empty_config() -> nvim_oxi::Result<()> {
    // Test that setup() works with an empty/default config.
    // The Logger is already initialized by previous tests, so we just verify
    // that calling setup with an empty config doesn't panic and updates state correctly.
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    // Empty config - should not panic
    let result = func.call(SetupArgs(None));

    // Verify no error was returned
    assert!(result.is_ok(), "Setup with empty config should not fail");

    // Verify state uses defaults
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

/// Test: setup() enables log file config
#[nvim_oxi::test]
fn setup_enables_log_file_config() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let setup_fn = setup(
        plugin_state.clone(),
        hermes::utilities::logging::Logger::inititalize(),
    );

    let func: nvim_oxi::Function<SetupArgs, ()> =
        nvim_oxi::conversion::FromObject::from_object(setup_fn)
            .expect("Failed to convert setup function");

    let temp_dir = std::env::temp_dir();
    let log_path = temp_dir.join("test_log_file.log");

    let config = ClientConfigPartial {
        log: Some(LogConfigPartial {
            file: Some(LogFileConfigPartial {
                path: Some(log_path.to_string_lossy().to_string()),
                level: None,
                format: None,
                max_size: None,
                max_files: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    let file_config = state.config.log.file.clone();
    assert_eq!(file_config.path, log_path.to_string_lossy().to_string());
    Ok(())
}

/// Test: setup() sets log file path
#[nvim_oxi::test]
fn setup_sets_log_file_path() -> nvim_oxi::Result<()> {
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
                path: Some("/tmp/test.log".to_string()),
                level: None,
                format: None,
                max_size: None,
                max_files: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    let file_config = state.config.log.file.clone();
    assert_eq!(file_config.path, "/tmp/test.log");
    Ok(())
}

/// Test: setup() sets log file level
#[nvim_oxi::test]
fn setup_sets_log_file_level() -> nvim_oxi::Result<()> {
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
                path: None,
                level: Some(hermes::utilities::LogLevel::Warn),
                format: None,
                max_size: None,
                max_files: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    let file_config = state.config.log.file.clone();
    assert_eq!(file_config.level, hermes::utilities::LogLevel::Warn);
    Ok(())
}

/// Test: setup() updates log target format
#[nvim_oxi::test]
fn setup_updates_stdio_log_format() -> nvim_oxi::Result<()> {
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
                level: None,
                format: Some(hermes::utilities::logging::LogFormat::Json),
            }),
            notification: None,
            message: None,
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert_eq!(
        state.config.log.stdio.format,
        hermes::utilities::logging::LogFormat::Json
    );
    Ok(())
}

/// Test: setup() updates notification log format
#[nvim_oxi::test]
fn setup_updates_notification_log_format() -> nvim_oxi::Result<()> {
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
            stdio: None,
            notification: Some(LogTargetConfigPartial {
                level: None,
                format: Some(hermes::utilities::logging::LogFormat::Pretty),
            }),
            message: None,
        }),
        ..Default::default()
    };

    func.call(SetupArgs(Some(config)))
        .expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert_eq!(
        state.config.log.notification.format,
        hermes::utilities::logging::LogFormat::Pretty
    );
    Ok(())
}
