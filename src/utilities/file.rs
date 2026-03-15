use std::path::Path;

use crate::acp::{Result, error::Error};

// TODO: move these helper functions into a "utilities" directory

/// Escape a filename so it is safe to use as an argument in an Ex command.
///
/// This function backslash-escapes characters that are significant to Ex
/// command parsing (such as spaces and `|`) so that they are treated as
/// literal filename characters.
fn escape_for_ex(filename: &str) -> String {
    let mut escaped = String::with_capacity(filename.len());
    for ch in filename.chars() {
        match ch {
            ' ' | '\t' | '\\' | '|' | '"' | '\'' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Find an existing buffer that is listed (visible to user)
pub fn find_existing_buffer(path: &Path) -> Option<nvim_oxi::api::Buffer> {
    nvim_oxi::api::list_bufs().into_iter().find(|b| {
        b.get_name().map(|p| p == path).unwrap_or(false)
            && nvim_oxi::api::get_option_value::<bool>(
                "buflisted",
                &nvim_oxi::api::opts::OptionOpts::builder()
                    .buffer(b.clone())
                    .build(),
            )
            .unwrap_or(false)
    })
}

/// Acquire buffer - returns (buffer, was_already_open)
pub fn acquire_or_create_buffer(path: &Path) -> Result<(nvim_oxi::api::Buffer, bool)> {
    if let Some(buf) = find_existing_buffer(path) {
        return Ok((buf, true));
    }

    let escaped_path = escape_for_ex(&path.to_string_lossy());
    nvim_oxi::api::command(&format!("badd {}", escaped_path))
        .map_err(|e| Error::Internal(e.to_string()))?;

    let buf = nvim_oxi::api::list_bufs()
        .into_iter()
        .find(|b| b.get_name().map(|p| p == path).unwrap_or(false))
        .ok_or_else(|| {
            Error::Internal(format!(
                "Buffer for file '{}' not found after badd",
                path.display()
            ))
        })?;

    Ok((buf, false))
}

/// Update buffer content from text
pub fn update_buffer_content(buf: &mut nvim_oxi::api::Buffer, content: &str) -> Result<()> {
    buf.set_lines(
        0..,
        false,
        content.lines().map(String::from).collect::<Vec<String>>(),
    )
    .map_err(|e| Error::Internal(e.to_string()))
}

/// Mark buffer as having unsaved changes
pub fn mark_buffer_modified(buf: &nvim_oxi::api::Buffer) -> Result<()> {
    nvim_oxi::api::set_option_value(
        "modified",
        true,
        &nvim_oxi::api::opts::OptionOpts::builder()
            .buffer(buf.clone())
            .build(),
    )
    .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(())
}

/// Save buffer to disk
pub fn save_buffer_to_disk(buf: &nvim_oxi::api::Buffer) -> Result<()> {
    // Run :write in the context of the given buffer and propagate any errors.
    buf.call(|_| {
        nvim_oxi::api::command("write")
            .inspect_err(|e| {
                tracing::error!(
                    "An error occurred while triggering write in Neovim: {:?}",
                    e
                )
            })
            .ok();
    })
    .map_err(|e| Error::Internal(e.to_string()))?;

    Ok(())
}

/// Refresh the display to show updated buffer content
pub fn refresh_view() -> Result<()> {
    nvim_oxi::api::command("redraw").map_err(|e| Error::Internal(e.to_string()))
}
