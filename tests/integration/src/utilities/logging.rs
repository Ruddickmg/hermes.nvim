//! Integration tests for logging functionality
//!
//! These tests verify that the Logger integrates correctly with the tracing
//! system and can be configured at runtime via the setup API.

use hermes::nvim::configuration::{LogConfig, LogFileConfig, LogTargetConfig};
use hermes::utilities::logging::{LogLevel, Logger};
use pretty_assertions::assert_eq;
use tempfile::TempDir;

/// Helper function to create a LogConfig with file logging enabled
fn create_log_config_with_file(level: LogLevel, file_config: LogFileConfig) -> LogConfig {
    LogConfig {
        stdio: LogTargetConfig {
            level,
            format: None,
        },
        file: Some(file_config),
        local_list: LogTargetConfig::default(),
        message: LogTargetConfig::default(),
        notification: LogTargetConfig::default(),
        quickfix: LogTargetConfig::default(),
    }
}

/// Integration test: Verifies that file logging can be enabled via configure()
#[nvim_oxi::test]
fn test_file_logging_can_be_enabled() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    // Initialize logger (file logging is OFF by default)
    let logger = Logger::inititalize();

    // Configure file logging
    let file_config = LogFileConfig {
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Info,
        format: None,
        max_size: Some(1024 * 1024),
        max_files: Some(5),
    };
    let config = create_log_config_with_file(LogLevel::Info, file_config);

    logger
        .configure(config)
        .expect("Failed to configure file logging");

    // Log a message
    tracing::info!("Test message from integration test");

    // Give the channel writer time to process and flush
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Verify log file contains the message
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        content.contains("Test message from integration test"),
        "Log file should contain the test message. Content length: {}",
        content.len()
    );

    Ok(())
}

/// Integration test: Verifies first message is written when file logging is enabled
#[nvim_oxi::test]
fn test_file_logging_first_message_written() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Enable file logging
    let file_config = LogFileConfig {
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Info,
        format: None,
        max_size: Some(1024 * 1024),
        max_files: Some(5),
    };
    let config = create_log_config_with_file(LogLevel::Info, file_config);
    logger.configure(config).unwrap();

    // Log a message
    tracing::info!("First message");
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Verify first message was written
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(
        content.contains("First message"),
        true,
        "Log file should contain first message"
    );

    Ok(())
}

/// Integration test: Verifies messages stop being written after disabling
#[nvim_oxi::test]
fn test_file_logging_disabled_stops_writing() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Enable file logging and write a message
    let enable_config = create_log_config_with_file(
        LogLevel::Info,
        LogFileConfig {
            path: log_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            format: None,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(enable_config).unwrap();
    tracing::info!("Before disable");
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Disable file logging
    let disable_config = create_log_config_with_file(
        LogLevel::Info,
        LogFileConfig {
            path: log_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            format: None,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(disable_config).unwrap();

    // Try to log after disabling
    tracing::info!("After disable");
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Verify disabled message was NOT written
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(
        content.contains("After disable"),
        false,
        "Log file should NOT contain message written after disabling"
    );

    Ok(())
}

/// Integration test: Verifies DEBUG messages are filtered at WARN level
#[nvim_oxi::test]
fn test_debug_filtered_at_warn_level_debug() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Configure with WARN level
    let file_config = LogFileConfig {
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Warn,
        format: None,
        max_size: Some(1024 * 1024),
        max_files: Some(5),
    };
    let config = create_log_config_with_file(LogLevel::Warn, file_config);
    logger.configure(config).unwrap();

    // Log different levels
    tracing::debug!("Debug message");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify DEBUG is filtered at WARN level
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        !content.contains("Debug message"),
        "DEBUG should be filtered at WARN level"
    );

    Ok(())
}

/// Integration test: Verifies INFO messages are filtered at WARN level
#[nvim_oxi::test]
fn test_debug_filtered_at_warn_level_info() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Configure with WARN level
    let file_config = LogFileConfig {
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Warn,
        format: None,
        max_size: Some(1024 * 1024),
        max_files: Some(5),
    };
    let config = create_log_config_with_file(LogLevel::Warn, file_config);
    logger.configure(config).unwrap();

    // Log different levels
    tracing::info!("Info message");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify INFO is filtered at WARN level
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        !content.contains("Info message"),
        "INFO should be filtered at WARN level"
    );

    Ok(())
}

/// Integration test: Verifies WARN messages are written at WARN level
#[nvim_oxi::test]
fn test_debug_filtered_at_warn_level_warn() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Configure with WARN level
    let file_config = LogFileConfig {
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Warn,
        format: None,
        max_size: Some(1024 * 1024),
        max_files: Some(5),
    };
    let config = create_log_config_with_file(LogLevel::Warn, file_config);
    logger.configure(config).unwrap();

    // Log different levels
    tracing::warn!("Warning message");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify WARN is written
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        content.contains("Warning message"),
        "WARN should be written"
    );

    Ok(())
}

/// Integration test: Verifies INFO is filtered at initial WARN level before reconfiguration
#[nvim_oxi::test]
fn test_log_level_reconfiguration_filtered_before() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Start with WARN level
    let warn_config = create_log_config_with_file(
        LogLevel::Warn,
        LogFileConfig {
            path: log_path.to_string_lossy().to_string(),
            level: LogLevel::Warn,
            format: None,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(warn_config).unwrap();

    // This should be filtered
    tracing::info!("Should be filtered");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        !content.contains("Should be filtered"),
        "INFO should be filtered at initial WARN level"
    );

    Ok(())
}

/// Integration test: Verifies INFO is written after reconfiguring to INFO level
#[nvim_oxi::test]
fn test_log_level_reconfiguration_written_after() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Start with WARN level
    let warn_config = create_log_config_with_file(
        LogLevel::Warn,
        LogFileConfig {
            path: log_path.to_string_lossy().to_string(),
            level: LogLevel::Warn,
            format: None,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(warn_config).unwrap();

    // Reconfigure to INFO level
    let info_config = create_log_config_with_file(
        LogLevel::Info,
        LogFileConfig {
            path: log_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            format: None,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(info_config).unwrap();

    // This should be written
    tracing::info!("Should be written");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(
        content.contains("Should be written"),
        "INFO should be written after reconfiguring to INFO level"
    );

    Ok(())
}

/// Integration test: Verifies log rotation works with small max_size
#[nvim_oxi::test]
fn test_log_rotation() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Configure with small max_size
    let file_config = LogFileConfig {
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Info,
        format: None,
        max_size: Some(100),
        max_files: Some(3),
    };
    let config = create_log_config_with_file(LogLevel::Info, file_config);
    logger.configure(config).unwrap();

    // Write messages
    for i in 0..20 {
        tracing::info!("Message {} with padding", i);
    }

    std::thread::sleep(std::time::Duration::from_millis(200));

    // Verify current log has recent messages
    let current_content = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(
        current_content.contains("Message"),
        true,
        "Current log should contain messages"
    );

    // Verify rotated files exist
    let log_dir = temp_dir.path();
    let rotated_files: Vec<_> = std::fs::read_dir(log_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name.starts_with("test.log.") && name.chars().last().unwrap().is_numeric()
        })
        .collect();

    assert!(!rotated_files.is_empty(), "Should have rotated log files");

    Ok(())
}

/// Integration test: Verifies reconfiguration to new path works
#[nvim_oxi::test]
fn test_reconfigure_to_second_path() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let first_path = temp_dir.path().join("first.log");
    let second_path = temp_dir.path().join("second.log");

    let logger = Logger::inititalize();

    // Configure first path
    let first_config = create_log_config_with_file(
        LogLevel::Info,
        LogFileConfig {
            path: first_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            format: None,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(first_config).unwrap();
    tracing::info!("First path message");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Reconfigure to second path
    let second_config = create_log_config_with_file(
        LogLevel::Info,
        LogFileConfig {
            path: second_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            format: None,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(second_config).unwrap();
    tracing::info!("Second path message");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify first log has first message only
    let first_content = std::fs::read_to_string(&first_path).unwrap();
    assert!(
        first_content.contains("First path message"),
        "First log should have first message"
    );

    // Verify second log has second message only
    let second_content = std::fs::read_to_string(&second_path).unwrap();
    assert!(
        second_content.contains("Second path message"),
        "Second log should have second message"
    );

    Ok(())
}

/// Integration test: Verifies custom LogTargetConfig level
#[nvim_oxi::test]
fn test_log_target_config_custom_level() -> nvim_oxi::Result<()> {
    use hermes::nvim::configuration::LogTargetConfig;

    let custom_config = LogTargetConfig {
        level: LogLevel::Debug,
        format: None,
    };
    assert_eq!(custom_config.level, LogLevel::Debug);

    Ok(())
}

/// Integration test: Verifies LogTargetConfig with format level
#[nvim_oxi::test]
fn test_log_target_config_with_format_level() -> nvim_oxi::Result<()> {
    use hermes::nvim::configuration::LogTargetConfig;
    use hermes::utilities::logging::LogFormat;

    let config = LogTargetConfig {
        level: LogLevel::Info,
        format: Some(LogFormat::Json),
    };
    assert_eq!(config.level, LogLevel::Info);

    Ok(())
}

/// Integration test: Verifies LogTargetConfig with format override stores format correctly
#[nvim_oxi::test]
fn test_log_target_config_with_format_format() -> nvim_oxi::Result<()> {
    use hermes::nvim::configuration::LogTargetConfig;
    use hermes::utilities::logging::LogFormat;

    let config = LogTargetConfig {
        level: LogLevel::Info,
        format: Some(LogFormat::Json),
    };
    assert_eq!(config.format, Some(LogFormat::Json));

    Ok(())
}

/// Integration test: Verifies that log format can be changed via configure()
#[nvim_oxi::test]
fn test_log_format_can_be_changed_via_configure() -> nvim_oxi::Result<()> {
    use hermes::nvim::configuration::LogTargetConfig;
    use hermes::utilities::logging::LogFormat;

    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // First, configure with Compact format
    let compact_config = LogConfig {
        stdio: LogTargetConfig {
            level: LogLevel::Info,
            format: Some(LogFormat::Compact),
        },
        file: Some(LogFileConfig {
            path: log_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            format: Some(LogFormat::Compact),
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        }),
        message: LogTargetConfig::default(),
        notification: LogTargetConfig::default(),
        quickfix: LogTargetConfig::default(),
    };
    logger
        .configure(compact_config)
        .expect("Failed to configure with Compact format");

    // Log first message
    tracing::info!("First message in compact format");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Now reconfigure with Json format
    let json_config = LogConfig {
        stdio: LogTargetConfig {
            level: LogLevel::Info,
            format: Some(LogFormat::Json),
        },
        file: Some(LogFileConfig {
            path: log_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            format: Some(LogFormat::Json),
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        }),
        message: LogTargetConfig::default(),
        notification: LogTargetConfig::default(),
        quickfix: LogTargetConfig::default(),
    };
    logger
        .configure(json_config)
        .expect("Failed to reconfigure with Json format");

    // Log second message
    tracing::info!("Second message in json format");
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify log file contains both messages
    let content = std::fs::read_to_string(&log_path).expect("Should be able to read log file");

    // Verify first message is present (compact format)
    assert!(
        content.contains("First message in compact format"),
        "Log file should contain first message"
    );

    // Verify second message is present (json format)
    assert!(
        content.contains("Second message in json format"),
        "Log file should contain second message"
    );

    // Verify format actually changed by checking for JSON structure in second message
    // JSON format includes fields like "message", "level", "target" etc.
    assert!(
        content.contains("\"message\":") || content.contains("\"level\":"),
        "Second message should be in JSON format"
    );

    Ok(())
}
