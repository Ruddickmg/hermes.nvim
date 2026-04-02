//! Integration tests for file utilities
use assert_fs::NamedTempFile;
use assert_fs::prelude::*;
use hermes::utilities::{
    acquire_or_create_buffer, detect_project_storage_path, find_existing_buffer,
    mark_buffer_modified, refresh_view, save_buffer_to_disk, update_buffer_content,
};

#[nvim_oxi::test]
fn test_find_existing_buffer_finds_open_file() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_find.txt").unwrap();

    // Open the file in Neovim
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Should find the buffer
    let buffer = find_existing_buffer(temp_file.path());
    assert!(
        buffer.is_some(),
        "Should find existing buffer for open file"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_find_existing_buffer_correct_path() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_find.txt").unwrap();

    // Open the file in Neovim
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Should find the buffer
    let buffer = find_existing_buffer(temp_file.path()).expect("Buffer should exist");

    // Verify it's the right buffer
    let name = buffer
        .get_name()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to get buffer name: {}", e)))?;
    assert_eq!(
        name,
        temp_file.path(),
        "Buffer should point to correct file"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_find_existing_buffer_not_found_for_unopened() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_not_open.txt").unwrap();

    // Don't open the file
    let buffer = find_existing_buffer(temp_file.path());
    assert!(buffer.is_none(), "Should not find buffer for unopened file");

    Ok(())
}

#[nvim_oxi::test]
fn test_acquire_or_create_buffer_creates_new() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_new.txt").unwrap();

    // Acquire buffer (file doesn't exist yet in Neovim)
    let (_buffer, was_already_open) = acquire_or_create_buffer(temp_file.path())
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    assert!(
        !was_already_open,
        "Buffer should not have been already open"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_acquire_or_create_buffer_new_has_correct_path() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_new.txt").unwrap();

    // Acquire buffer (file doesn't exist yet in Neovim)
    let (buffer, _) = acquire_or_create_buffer(temp_file.path())
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify buffer exists
    let name = buffer
        .get_name()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to get buffer name: {}", e)))?;
    assert_eq!(
        name,
        temp_file.path(),
        "Buffer should point to correct file"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_acquire_or_create_buffer_finds_existing() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_existing.txt").unwrap();

    // Open the file first
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Acquire buffer again
    let (_buffer, was_already_open) = acquire_or_create_buffer(temp_file.path())
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    assert!(
        was_already_open,
        "Should detect that buffer was already open"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_acquire_or_create_buffer_existing_has_correct_path() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_existing.txt").unwrap();

    // Open the file first
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Acquire buffer again
    let (buffer, _) = acquire_or_create_buffer(temp_file.path())
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify buffer exists
    let name = buffer
        .get_name()
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to get buffer name: {}", e)))?;
    assert_eq!(
        name,
        temp_file.path(),
        "Buffer should point to correct file"
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_update_buffer_content() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_update.txt").unwrap();
    temp_file.write_str("initial content").unwrap();

    // Open file
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    // Get buffer and update content
    let mut buffer = find_existing_buffer(temp_file.path()).expect("Buffer should exist");
    update_buffer_content(&mut buffer, "updated content")
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify buffer content changed
    let lines: Vec<String> = buffer
        .get_lines(0.., false)
        .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to get lines: {}", e)))?
        .map(|s| s.to_string())
        .collect();
    let content = lines.join("\n");
    assert_eq!(
        content, "updated content",
        "Buffer content should be updated"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_mark_buffer_modified() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_modified.txt").unwrap();
    temp_file.write_str("content").unwrap();

    // Open file
    nvim_oxi::api::command(&format!("edit {}", temp_file.path().display()))?;

    let buffer = find_existing_buffer(temp_file.path()).expect("Buffer should exist");

    // Mark as modified
    mark_buffer_modified(&buffer).map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify modified flag is set
    let is_modified: bool = nvim_oxi::api::get_option_value::<bool>(
        "modified",
        &nvim_oxi::api::opts::OptionOpts::builder()
            .buffer(buffer.clone())
            .build(),
    )
    .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to get modified: {}", e)))?;

    assert!(is_modified, "Buffer should be marked as modified");

    Ok(())
}

#[nvim_oxi::test]
fn test_save_buffer_to_disk() -> nvim_oxi::Result<()> {
    let temp_file = NamedTempFile::new("test_save.txt").unwrap();

    // Create buffer with content
    let (mut buffer, _) = acquire_or_create_buffer(temp_file.path())
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Update content
    update_buffer_content(&mut buffer, "saved content")
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Save to disk
    save_buffer_to_disk(&buffer).map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    // Verify file on disk
    let disk_content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(
        disk_content.contains("saved content"),
        "File on disk should contain saved content"
    );

    Ok(())
}

#[nvim_oxi::test]
fn test_refresh_view() -> nvim_oxi::Result<()> {
    // refresh_view should not error
    refresh_view().map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    Ok(())
}

#[nvim_oxi::test]
fn test_detect_project_storage_path_returns_hermes_path() -> nvim_oxi::Result<()> {
    let path = detect_project_storage_path().map_err(|e| {
        nvim_oxi::api::Error::Other(format!("Failed to detect storage path: {}", e))
    })?;

    assert!(!path.is_empty(), "Path should not be empty");
    assert!(
        path.ends_with("/hermes"),
        "Path should end with /hermes: {}",
        path
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_detect_project_storage_path_matches_neovim_stdpath() -> nvim_oxi::Result<()> {
    // Get the actual stdpath from Neovim to verify we're using the correct source
    let state_dir =
        nvim_oxi::api::call_function::<(String,), String>("stdpath", ("state".to_string(),))
            .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to call stdpath: {}", e)))?;

    assert!(!state_dir.is_empty(), "Neovim stdpath should not be empty");

    // Get the path from our function
    let path = detect_project_storage_path().map_err(|e| {
        nvim_oxi::api::Error::Other(format!("Failed to detect storage path: {}", e))
    })?;

    // Verify the path matches {stdpath('state')}/hermes
    let expected_path = format!("{}/hermes", state_dir);
    assert_eq!(
        path, expected_path,
        "Path should match stdpath('state')/hermes: got {}, expected {}",
        path, expected_path
    );
    Ok(())
}

#[nvim_oxi::test]
fn test_detect_project_storage_path_uses_xdg_state_home() -> nvim_oxi::Result<()> {
    use std::env;
    use tempfile::TempDir;

    // Create a temporary directory to act as our custom XDG_STATE_HOME
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_str().unwrap();

    // Set XDG_STATE_HOME environment variable (unsafe required in Rust 2024)
    unsafe {
        env::set_var("XDG_STATE_HOME", temp_path);
    }

    // Get stdpath from Neovim - should now return {temp_path}/nvim
    let _state_dir =
        nvim_oxi::api::call_function::<(String,), String>("stdpath", ("state".to_string(),))
            .map_err(|e| nvim_oxi::api::Error::Other(format!("Failed to call stdpath: {}", e)))?;

    // Get the path from our function
    let path = detect_project_storage_path().map_err(|e| {
        nvim_oxi::api::Error::Other(format!("Failed to detect storage path: {}", e))
    })?;

    // Verify the function returns {XDG_STATE_HOME}/nvim/hermes
    let expected_path = format!("{}/nvim/hermes", temp_path);
    assert_eq!(
        path, expected_path,
        "Path should use XDG_STATE_HOME/nvim/hermes: got {}, expected {}",
        path, expected_path
    );

    // Cleanup: remove the environment variable (unsafe required in Rust 2024)
    unsafe {
        env::remove_var("XDG_STATE_HOME");
    }

    Ok(())
}

#[nvim_oxi::test]
fn test_detect_project_storage_path_returns_valid_path() -> nvim_oxi::Result<()> {
    let path = detect_project_storage_path().map_err(|e| {
        nvim_oxi::api::Error::Other(format!("Failed to detect storage path: {}", e))
    })?;

    // Verify the path is absolute (starts with / on Unix)
    assert!(path.starts_with('/'), "Path should be absolute: {}", path);

    Ok(())
}
