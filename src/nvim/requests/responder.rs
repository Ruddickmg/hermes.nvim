use crate::acp::{error::Error, Result};
use agent_client_protocol::{
    RequestPermissionOutcome, RequestPermissionRequest, WriteTextFileRequest, WriteTextFileResponse,
};
use nvim_oxi::mlua;
use std::path::Path;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum Responder {
    Cancelled,
    PermissionResponse(
        oneshot::Sender<RequestPermissionOutcome>,
        RequestPermissionRequest,
    ),
    WriteFileResponse(oneshot::Sender<WriteTextFileResponse>, WriteTextFileRequest),
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

    nvim_oxi::api::command(&format!("badd {}", path.to_string_lossy()))
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
    buf.call(|()| {
        nvim_oxi::api::command("write").ok();
    })
    .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(())
}

/// Refresh the display to show updated buffer content
pub fn refresh_view() -> Result<()> {
    nvim_oxi::api::command("redraw").map_err(|e| Error::Internal(e.to_string()))
}

impl Responder {
    pub fn default(self) -> Result<()> {
        match self {
            Self::PermissionResponse(_, data) => {
                let _options = data
                    .options
                    .iter()
                    .map(|option| option.name.to_string())
                    .collect::<Vec<String>>();
                mlua::Lua::new().globals();
            }
            Self::WriteFileResponse(responder, data) => {
                let path = data.path.clone();
                let (mut buffer, was_already_open) = acquire_or_create_buffer(&path)?;

                update_buffer_content(&mut buffer, &data.content)?;

                if was_already_open {
                    mark_buffer_modified(&buffer)?;
                    // TODO: Make auto-save configurable
                    // if auto_save_enabled {
                    //     save_buffer_to_disk(&buf)?;
                    refresh_view()?;
                } else {
                    save_buffer_to_disk(&buffer)?;
                }

                responder.send(WriteTextFileResponse::new()).map_err(|_| {
                    Error::Internal(
                        "Failed to respond to ACP about successful file write".to_string(),
                    )
                })?;
            }
            Self::Cancelled => {}
        }
        Ok(())
    }
}
