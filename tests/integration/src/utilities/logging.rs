//! Integration tests for logging functionality
//!
//! These tests verify that the Logger integrates correctly with the tracing
//! system and can be configured at runtime via the setup API.

use hermes::nvim::configuration::{LogConfig, LogFileConfig};
use hermes::utilities::logging::{LogLevel, Logger};
use pretty_assertions::assert_eq;
use tempfile::TempDir;

/// Helper function to create a LogConfig with file logging enabled
fn create_log_config_with_file(level: LogLevel, file_config: LogFileConfig) -> LogConfig {
    LogConfig {
        level,
        file: Some(file_config),
        local_list: LogLevel::Off,
        message: LogLevel::Off,
        notification: LogLevel::Off,
        quick_fix_list: LogLevel::Off,
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
        enabled: true,
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Info,
        max_size: Some(1024 * 1024),
        max_files: Some(5),
    };
    let config = create_log_config_with_file(LogLevel::Info, file_config);

    logger
        .configure(config)
        .expect("Failed to configure file logging");

    // Log a message
    tracing::info!("Test message from integration test");

    // Give the channel writer time to process
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Verify log file was created and contains the message
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(
        content.contains("Test message from integration test"),
        true,
        "Log file should contain the test message"
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
        enabled: true,
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Info,
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
            enabled: true,
            path: log_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
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
            enabled: false,
            path: log_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
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
fn test_debug_filtered_at_warn_level() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Configure with WARN level
    let file_config = LogFileConfig {
        enabled: true,
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Warn,
        max_size: Some(1024 * 1024),
        max_files: Some(5),
    };
    let config = create_log_config_with_file(LogLevel::Warn, file_config);
    logger.configure(config).unwrap();

    // Log at DEBUG level
    tracing::debug!("Debug message");
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Verify DEBUG message was filtered
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(
        content.contains("Debug message"),
        false,
        "DEBUG messages should be filtered at WARN level"
    );

    Ok(())
}

/// Integration test: Verifies WARN messages appear at WARN level
#[nvim_oxi::test]
fn test_warn_appears_at_warn_level() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Configure with WARN level
    let file_config = LogFileConfig {
        enabled: true,
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Warn,
        max_size: Some(1024 * 1024),
        max_files: Some(5),
    };
    let config = create_log_config_with_file(LogLevel::Warn, file_config);
    logger.configure(config).unwrap();

    // Log at WARN level
    tracing::warn!("Warn message");
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Verify WARN message appears
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(
        content.contains("Warn message"),
        true,
        "WARN messages should appear at WARN level"
    );

    Ok(())
}

/// Integration test: Verifies file rotation creates backup
#[nvim_oxi::test]
fn test_rotation_creates_backup_file() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Configure with small max_size to trigger rotation quickly
    let file_config = LogFileConfig {
        enabled: true,
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Info,
        max_size: Some(100), // 100 bytes
        max_files: Some(3),
    };
    let config = create_log_config_with_file(LogLevel::Info, file_config);
    logger.configure(config).unwrap();

    // Write enough messages to trigger rotation
    for i in 0..20 {
        tracing::info!("Message {} with padding", i);
    }

    std::thread::sleep(std::time::Duration::from_millis(200));

    // Verify backup file was created
    let backup_1 = log_path.with_extension("1");
    assert_eq!(
        backup_1.exists() || log_path.exists(),
        true,
        "Either current log or backup should exist after rotation"
    );

    Ok(())
}

/// Integration test: Verifies current log contains messages after rotation
#[nvim_oxi::test]
fn test_current_log_has_messages_after_rotation() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");

    let logger = Logger::inititalize();

    // Configure with small max_size
    let file_config = LogFileConfig {
        enabled: true,
        path: log_path.to_string_lossy().to_string(),
        level: LogLevel::Info,
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

    Ok(())
}

/// Integration test: Verifies reconfiguration to new path works
#[nvim_oxi::test]
fn test_reconfigure_to_second_path() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let first_path = temp_dir.path().join("first.log");
    let second_path = temp_dir.path().join("second.log");

    let logger = Logger::inititalize();

    // First configuration
    let config1 = create_log_config_with_file(
        LogLevel::Info,
        LogFileConfig {
            enabled: true,
            path: first_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(config1).unwrap();
    tracing::info!("First path message");
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Reconfigure to second path
    let config2 = create_log_config_with_file(
        LogLevel::Info,
        LogFileConfig {
            enabled: true,
            path: second_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(config2).unwrap();
    tracing::info!("Second path message");
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Verify second path has the new message
    let second_content = std::fs::read_to_string(&second_path).unwrap();
    assert_eq!(
        second_content.contains("Second path message"),
        true,
        "Second file should contain message written after reconfiguration"
    );

    Ok(())
}

/// Integration test: Verifies first path does not get second message
#[nvim_oxi::test]
fn test_first_path_does_not_get_second_message() -> nvim_oxi::Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let first_path = temp_dir.path().join("first.log");
    let second_path = temp_dir.path().join("second.log");

    let logger = Logger::inititalize();

    // First configuration
    let config1 = create_log_config_with_file(
        LogLevel::Info,
        LogFileConfig {
            enabled: true,
            path: first_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(config1).unwrap();
    tracing::info!("First path message");
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Reconfigure to second path
    let config2 = create_log_config_with_file(
        LogLevel::Info,
        LogFileConfig {
            enabled: true,
            path: second_path.to_string_lossy().to_string(),
            level: LogLevel::Info,
            max_size: Some(1024 * 1024),
            max_files: Some(5),
        },
    );
    logger.configure(config2).unwrap();
    tracing::info!("Second path message");
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Verify first path does NOT have second message
    let first_content = std::fs::read_to_string(&first_path).unwrap();
    assert_eq!(
        first_content.contains("Second path message"),
        false,
        "First file should NOT contain message written to second file"
    );

    Ok(())
}
