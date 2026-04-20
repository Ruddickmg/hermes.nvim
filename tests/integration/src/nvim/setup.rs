use crate::helpers::mock_runtime;
use hermes::{
    Handler, PluginState,
    api::{Api, SetupArgs},
    nvim::{
        configuration::{
            BufferConfigPartial, ClientConfigPartial, LogConfigPartial, LogFileConfigPartial,
            LogTargetConfigPartial,
        },
        requests::Requests,
    },
    utilities::detect_project_storage_path,
};
use nvim_oxi;
use pretty_assertions::assert_eq;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

fn create_test_api(
    plugin_state: Arc<Mutex<PluginState>>,
    logger: &'static hermes::utilities::Logger,
) -> hermes::api::Api {
    let runtime = mock_runtime();
    let requests = Rc::new(Requests::new(runtime.clone(), plugin_state.clone()).expect("Failed to create requests"));
    let handler = Arc::new(
        Handler::new(plugin_state.clone(), runtime.clone(), requests.clone()).expect("Failed to create handler"),
    );
    Api::new(plugin_state, logger, handler, requests, runtime)
}

/// Helper to block on an async future in synchronous tests
fn block_on<F>(fut: F) -> F::Output
where
    F: std::future::Future,
{
    futures::executor::block_on(fut)
}

/// Test: setup() updates permissions correctly
#[nvim_oxi::test]
fn setup_updates_permissions_correctly() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    let config = ClientConfigPartial {
        permissions: Some(hermes::nvim::configuration::PermissionsPartial {
            fs_write_access: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert!(!state.config.permissions.fs_write_access); // Single assertion
    Ok(())
}

/// Test: setup() updates buffer config correctly
#[nvim_oxi::test]
fn setup_updates_buffer_config_correctly() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    let config = ClientConfigPartial {
        buffer: Some(BufferConfigPartial {
            auto_save: Some(true),
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert!(state.config.buffer.auto_save); // Single assertion
    Ok(())
}

/// Test: setup() updates stdio log level correctly
#[nvim_oxi::test]
fn setup_updates_stdio_log_level() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

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

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

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
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

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

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

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
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

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

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

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
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    // First call: set permissions
    block_on(api.setup(SetupArgs(Some(ClientConfigPartial {
        permissions: Some(hermes::nvim::configuration::PermissionsPartial {
            fs_write_access: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    }))))
    .expect("Failed to call setup first time");

    // Second call: set buffer config (should keep permissions)
    block_on(api.setup(SetupArgs(Some(ClientConfigPartial {
        buffer: Some(BufferConfigPartial {
            auto_save: Some(true),
        }),
        ..Default::default()
    }))))
    .expect("Failed to call setup second time");

    let state = plugin_state.blocking_lock();
    assert!(!state.config.permissions.fs_write_access);
    Ok(())
}

/// Test: setup() updates buffer config on subsequent calls
#[nvim_oxi::test]
fn setup_updates_buffer_config_on_subsequent_calls() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    // First call: set permissions
    block_on(api.setup(SetupArgs(Some(ClientConfigPartial {
        permissions: Some(hermes::nvim::configuration::PermissionsPartial {
            fs_write_access: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    }))))
    .expect("Failed to call setup first time");

    // Second call: set buffer config
    block_on(api.setup(SetupArgs(Some(ClientConfigPartial {
        buffer: Some(BufferConfigPartial {
            auto_save: Some(true),
        }),
        ..Default::default()
    }))))
    .expect("Failed to call setup second time");

    let state = plugin_state.blocking_lock();
    assert!(state.config.buffer.auto_save);
    Ok(())
}

/// Test: setup() works with empty config
#[nvim_oxi::test]
fn setup_with_empty_config_does_not_fail() -> nvim_oxi::Result<()> {
    // Test that setup() works with an empty/default config.
    // The Logger is already initialized by previous tests, so we just verify
    // that calling setup with an empty config doesn't panic.
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    // Empty config - should not panic
    let result = block_on(api.setup(SetupArgs(None)));

    // Verify no error was returned
    assert!(result.is_ok(), "Setup with empty config should not fail");

    Ok(())
}

#[nvim_oxi::test]
fn setup_with_empty_config_uses_default_permissions() -> nvim_oxi::Result<()> {
    // Test that setup() uses default permissions when given empty config.
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    // Empty config - should not panic
    block_on(api.setup(SetupArgs(None))).expect("Setup should not fail");

    // Verify state uses defaults
    let state = plugin_state.blocking_lock();
    assert!(
        state.config.permissions.fs_read_access,
        "Default fs_read_access should be true"
    );
    Ok(())
}

/// Test: setup() works with None
#[nvim_oxi::test]
fn setup_works_with_none() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    // None config
    block_on(api.setup(SetupArgs(None))).expect("Failed to call setup");

    // Should use defaults
    let state = plugin_state.blocking_lock();
    assert!(state.config.permissions.fs_read_access); // Default true
    Ok(())
}

/// Test: setup() enables log file config
#[nvim_oxi::test]
fn setup_enables_log_file_config() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

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

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    let file_config = state.config.log.file.clone();
    assert_eq!(file_config.path, log_path.to_string_lossy().to_string());
    Ok(())
}

/// Test: setup() sets log file path
#[nvim_oxi::test]
fn setup_sets_log_file_path() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

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

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    let file_config = state.config.log.file.clone();
    assert_eq!(file_config.path, "/tmp/test.log");
    Ok(())
}

/// Test: setup() sets log file level
#[nvim_oxi::test]
fn setup_sets_log_file_level() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

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

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    let file_config = state.config.log.file.clone();
    assert_eq!(file_config.level, hermes::utilities::LogLevel::Warn);
    Ok(())
}

/// Test: setup() updates log target format
#[nvim_oxi::test]
fn setup_updates_stdio_log_format() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

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

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

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
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

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

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = plugin_state.blocking_lock();
    assert_eq!(
        state.config.log.notification.format,
        hermes::utilities::logging::LogFormat::Pretty
    );
    Ok(())
}
