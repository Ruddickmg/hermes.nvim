//! Integration tests for file utilities
use assert_fs::prelude::*;
use assert_fs::NamedTempFile;
use hermes::utilities::{
    acquire_or_create_buffer, find_existing_buffer, mark_buffer_modified, refresh_view,
    save_buffer_to_disk, update_buffer_content,
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

    // Verify it's the right buffer
    let buf = buffer.unwrap();
    let name = buf
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
    let (buffer, was_already_open) = acquire_or_create_buffer(temp_file.path())
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    assert!(
        !was_already_open,
        "Buffer should not have been already open"
    );

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
    let (buffer, was_already_open) = acquire_or_create_buffer(temp_file.path())
        .map_err(|e| nvim_oxi::api::Error::Other(e.to_string()))?;

    assert!(
        was_already_open,
        "Should detect that buffer was already open"
    );

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
