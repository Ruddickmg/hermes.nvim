use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

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

pub fn read_file_content(path: &PathBuf, start: Option<u32>, end: Option<u32>) -> Result<String> {
    use std::io::BufRead;

    let file = File::open(&path).map_err(|e| Error::Internal(e.to_string()))?;
    let reader = BufReader::new(file);

    // Convert options to u64 for safe arithmetic
    let start_line = start.map(|s| s as u64).unwrap_or(0);
    let end_line = end.map(|e| e as u64);

    // Validate bounds: if end is specified, start must be <= end
    if let Some(e) = end_line {
        if start_line > e {
            return Err(Error::Internal(format!(
                "Invalid line range: start ({}) > end ({})",
                start_line, e
            )));
        }
    }

    let mut content = String::new();
    let mut current_line: u64 = 0;

    for line_result in reader.lines() {
        let line = line_result.map_err(|e| Error::Internal(e.to_string()))?;

        // Check if we should stop reading
        if let Some(end) = end_line {
            if current_line >= end {
                break;
            }
        }

        // Check if we should include this line
        let should_include = current_line >= start_line;

        if should_include {
            content.push_str(&line);
            content.push('\n');
        }

        current_line += 1;
    }

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_file_with_lines(line_count: usize) -> (tempfile::NamedTempFile, Vec<String>) {
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        let mut lines = Vec::new();

        for i in 0..line_count {
            let line = format!("line{}", i);
            writeln!(temp_file, "{}", line).unwrap();
            lines.push(line);
        }

        (temp_file, lines)
    }

    #[test]
    fn read_file_content_full_file() {
        let (temp_file, expected_lines) = create_test_file_with_lines(5);
        let content = read_file_content(&temp_file.path().to_path_buf(), None, None).unwrap();

        let actual_lines: Vec<&str> = content.trim_end().split('\n').collect();
        assert_eq!(actual_lines.len(), 5);
        assert_eq!(
            actual_lines,
            expected_lines
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn read_file_content_with_start_line() {
        let (temp_file, expected_lines) = create_test_file_with_lines(5);
        let content = read_file_content(&temp_file.path().to_path_buf(), Some(2), None).unwrap();

        let actual_lines: Vec<&str> = content.trim_end().split('\n').collect();
        assert_eq!(actual_lines.len(), 3); // lines 2, 3, 4
        assert_eq!(actual_lines[0], "line2");
        assert_eq!(actual_lines[2], "line4");
    }

    #[test]
    fn read_file_content_with_end_line() {
        let (temp_file, _expected_lines) = create_test_file_with_lines(5);
        let content = read_file_content(&temp_file.path().to_path_buf(), None, Some(3)).unwrap();

        let actual_lines: Vec<&str> = content.trim_end().split('\n').collect();
        assert_eq!(actual_lines.len(), 3); // lines 0, 1, 2
        assert_eq!(actual_lines[0], "line0");
        assert_eq!(actual_lines[2], "line2");
    }

    #[test]
    fn read_file_content_with_start_and_end() {
        let (temp_file, _expected_lines) = create_test_file_with_lines(5);
        let content = read_file_content(&temp_file.path().to_path_buf(), Some(1), Some(4)).unwrap();

        let actual_lines: Vec<&str> = content.trim_end().split('\n').collect();
        assert_eq!(actual_lines.len(), 3); // lines 1, 2, 3
        assert_eq!(actual_lines[0], "line1");
        assert_eq!(actual_lines[1], "line2");
        assert_eq!(actual_lines[2], "line3");
    }

    #[test]
    fn read_file_content_end_zero_returns_empty() {
        let (temp_file, _expected_lines) = create_test_file_with_lines(5);
        let content = read_file_content(&temp_file.path().to_path_buf(), None, Some(0)).unwrap();

        assert!(content.is_empty());
    }

    #[test]
    fn read_file_content_empty_file() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let content = read_file_content(&temp_file.path().to_path_buf(), None, None).unwrap();

        assert!(content.is_empty());
    }

    #[test]
    fn read_file_content_invalid_range_errors() {
        let (temp_file, _expected_lines) = create_test_file_with_lines(5);
        let result = read_file_content(&temp_file.path().to_path_buf(), Some(5), Some(2));

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid line range"));
    }

    #[test]
    fn read_file_content_nonexistent_file_errors() {
        let result = read_file_content(&PathBuf::from("/nonexistent/file.txt"), None, None);

        assert!(result.is_err());
    }
}
