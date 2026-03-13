use crate::acp::{error::Error, Result};
use agent_client_protocol::{
    RequestPermissionOutcome, RequestPermissionRequest, WriteTextFileRequest, WriteTextFileResponse,
};
use nvim_oxi::mlua;
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

                // Check for existing buffer that is also listed (visible to user)
                let existing_buffer = nvim_oxi::api::list_bufs().into_iter().find(|b| {
                    b.get_name().map(|p| p == path).unwrap_or(false)
                        && nvim_oxi::api::get_option_value::<bool>(
                            "buflisted",
                            &nvim_oxi::api::opts::OptionOpts::builder()
                                .buffer(b.clone())
                                .build(),
                        )
                        .unwrap_or(false)
                });

                let was_already_open = existing_buffer.is_some();

                let _buffer = if let Some(mut buf) = existing_buffer {
                    // Buffer was already open: update in place, mark modified, don't write to disk
                    buf.set_lines(
                        0..,
                        false,
                        data.content
                            .lines()
                            .map(String::from)
                            .collect::<Vec<String>>(),
                    )
                    .map_err(|e| Error::Internal(e.to_string()))?;

                    // Mark buffer as modified so user knows there are unsaved changes
                    nvim_oxi::api::set_option_value(
                        "modified",
                        true,
                        &nvim_oxi::api::opts::OptionOpts::builder()
                            .buffer(buf.clone())
                            .build(),
                    )
                    .map_err(|e| Error::Internal(e.to_string()))?;

                    // TODO: Future enhancement - check if buffer has unsaved changes before updating
                    // and make this behavior configurable (prompt user, auto-merge, or overwrite)

                    buf
                } else {
                    // Buffer not open: create it and write to disk
                    nvim_oxi::api::command(&format!("badd {}", path.to_string_lossy()))
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    let mut buf = nvim_oxi::api::list_bufs()
                        .into_iter()
                        .find(|b| b.get_name().map(|p| p == path).unwrap_or(false))
                        .ok_or_else(|| {
                            Error::Internal(format!(
                                "Buffer for file '{}' not found after badd",
                                path.display()
                            ))
                        })?;

                    buf.set_lines(
                        0..,
                        false,
                        data.content
                            .lines()
                            .map(String::from)
                            .collect::<Vec<String>>(),
                    )
                    .map_err(|e| Error::Internal(e.to_string()))?;

                    // Only write to disk if buffer wasn't already open
                    buf.call(|()| nvim_oxi::api::command("write").ok())
                        .map_err(|e| Error::Internal(e.to_string()))?;

                    buf
                };

                // Refresh view if buffer was already open (show updated content to user)
                if was_already_open {
                    nvim_oxi::api::command("redraw").map_err(|e| Error::Internal(e.to_string()))?;
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
