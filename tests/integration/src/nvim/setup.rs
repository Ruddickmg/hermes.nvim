use crate::helpers::{mock_connection_manager, mock_runtime};
use async_lock::Mutex;
use hermes::{
    Handler, PluginState,
    api::{Api, SetupArgs},
    nvim::{
        configuration::{
            BufferConfigPartial, ClientConfigPartial, LogConfigPartial, LogFileConfigPartial,
            LogTargetConfigPartial, ProgressConfigPartial,
        },
        requests::Requests,
    },
    utilities::detect_project_storage_path,
};
use nvim_oxi;
use pretty_assertions::assert_eq;
use std::rc::Rc;
use std::sync::Arc;

fn create_test_api(
    plugin_state: Arc<Mutex<PluginState>>,
    logger: &'static hermes::utilities::Logger,
) -> hermes::api::Api {
    let runtime = mock_runtime();
    let requests = Rc::new(
        Requests::new(runtime.clone(), plugin_state.clone()).expect("Failed to create requests"),
    );
    let connection_manager = mock_connection_manager(&plugin_state);
    let handler = Arc::new(
        Handler::new(
            plugin_state.clone(),
            connection_manager.clone(),
            runtime.clone(),
            requests.clone(),
        )
        .expect("Failed to create handler"),
    );
    Api::new(plugin_state, logger, handler, requests, connection_manager)
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

    let state = smol::block_on(plugin_state.lock());
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

    let state = smol::block_on(plugin_state.lock());
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
                show_ansi: None,
            }),
            notification: None,
            message: None,
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = smol::block_on(plugin_state.lock());
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
                show_ansi: None,
            }),
            message: None,
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = smol::block_on(plugin_state.lock());
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
                show_ansi: None,
            }),
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = smol::block_on(plugin_state.lock());
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

    let state = smol::block_on(plugin_state.lock());
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

    let state = smol::block_on(plugin_state.lock());
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
    let state = smol::block_on(plugin_state.lock());
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
    let state = smol::block_on(plugin_state.lock());
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
                show_ansi: None,
                max_size: None,
                max_files: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = smol::block_on(plugin_state.lock());
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
                show_ansi: None,
                max_size: None,
                max_files: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = smol::block_on(plugin_state.lock());
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
                show_ansi: None,
                max_size: None,
                max_files: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = smol::block_on(plugin_state.lock());
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
                show_ansi: None,
            }),
            notification: None,
            message: None,
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = smol::block_on(plugin_state.lock());
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
                show_ansi: None,
            }),
            message: None,
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = smol::block_on(plugin_state.lock());
    assert_eq!(
        state.config.log.notification.format,
        hermes::utilities::logging::LogFormat::Pretty
    );
    Ok(())
}

/// Test: setup() updates progress cmdline config
#[nvim_oxi::test]
fn setup_updates_progress_cmdline_state() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    let config = ClientConfigPartial {
        progress: Some(ProgressConfigPartial {
            cmdline: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let state = smol::block_on(plugin_state.lock());
    assert!(state.config.progress.cmdline);
    Ok(())
}

/// Test: setup() enables progress in cmdline via messagesopt
#[nvim_oxi::test]
fn setup_enables_progress_in_cmdline_via_setup() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    let config = ClientConfigPartial {
        progress: Some(ProgressConfigPartial {
            cmdline: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    block_on(api.setup(SetupArgs(Some(config)))).expect("Failed to call setup");

    let messagesopt = nvim_oxi::api::call_function::<(String,), String>(
        "execute",
        ("set messagesopt?".to_string(),),
    )
    .unwrap_or_default();
    assert!(
        messagesopt.contains("progress:c"),
        "messagesopt should contain progress:c after setup with cmdline: true, got: {}",
        messagesopt
    );
    Ok(())
}

/// Test: setup() disables progress in cmdline via messagesopt
#[nvim_oxi::test]
fn setup_disables_progress_in_cmdline_via_setup() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    let enable_config = ClientConfigPartial {
        progress: Some(ProgressConfigPartial {
            cmdline: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    block_on(api.setup(SetupArgs(Some(enable_config)))).expect("Failed to enable");

    let disable_config = ClientConfigPartial {
        progress: Some(ProgressConfigPartial {
            cmdline: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };
    block_on(api.setup(SetupArgs(Some(disable_config)))).expect("Failed to disable");

    let messagesopt = nvim_oxi::api::call_function::<(String,), String>(
        "execute",
        ("set messagesopt?".to_string(),),
    )
    .unwrap_or_default();
    assert!(
        !messagesopt.contains("progress:c"),
        "messagesopt should not contain progress:c after setup with cmdline: false, got: {}",
        messagesopt
    );
    Ok(())
}

/// Test: setup() does not call show_progress_in_cmdline when progress config is omitted
#[nvim_oxi::test]
fn setup_does_not_update_cmdline_when_progress_omitted() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    // First enable cmdline progress
    let enable_config = ClientConfigPartial {
        progress: Some(ProgressConfigPartial {
            cmdline: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    block_on(api.setup(SetupArgs(Some(enable_config)))).expect("Failed to enable");

    // Now setup with non-progress config (omitting progress entirely)
    let buffer_config = ClientConfigPartial {
        buffer: Some(BufferConfigPartial {
            auto_save: Some(true),
        }),
        ..Default::default()
    };
    block_on(api.setup(SetupArgs(Some(buffer_config)))).expect("Failed to set buffer config");

    // Verify cmdline progress is still enabled (not reset by the guard)
    let messagesopt = nvim_oxi::api::call_function::<(String,), String>(
        "execute",
        ("set messagesopt?".to_string(),),
    )
    .unwrap_or_default();
    assert!(
        messagesopt.contains("progress:c"),
        "messagesopt should still contain progress:c after setup without progress config, got: {}",
        messagesopt
    );
    Ok(())
}

/// Test: setup() does not call show_progress_in_cmdline when cmdline value is unchanged
#[nvim_oxi::test]
fn setup_does_not_update_cmdline_when_value_unchanged() -> nvim_oxi::Result<()> {
    let plugin_state = Arc::new(Mutex::new(PluginState::new()));
    let logger =
        hermes::utilities::logging::Logger::inititalize(&detect_project_storage_path().unwrap())
            .unwrap();
    let api = create_test_api(plugin_state.clone(), logger);

    // Start with default (cmdline: false)
    // Set cmdline to true
    let enable_config = ClientConfigPartial {
        progress: Some(ProgressConfigPartial {
            cmdline: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    block_on(api.setup(SetupArgs(Some(enable_config)))).expect("Failed to enable");

    // Set cmdline to true again (same value)
    let same_config = ClientConfigPartial {
        progress: Some(ProgressConfigPartial {
            cmdline: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    block_on(api.setup(SetupArgs(Some(same_config)))).expect("Failed to set same value");

    // Verify cmdline progress is still enabled
    let messagesopt = nvim_oxi::api::call_function::<(String,), String>(
        "execute",
        ("set messagesopt?".to_string(),),
    )
    .unwrap_or_default();
    assert!(
        messagesopt.contains("progress:c"),
        "messagesopt should still contain progress:c when same value set twice, got: {}",
        messagesopt
    );
    Ok(())
}
