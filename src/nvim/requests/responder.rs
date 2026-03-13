use crate::acp::{error::Error, Result};
use agent_client_protocol::{
    RequestPermissionOutcome, RequestPermissionRequest, WriteTextFileRequest, WriteTextFileResponse,
};
use nvim_oxi::mlua;
use tokio::sync::oneshot;
use tracing::error;

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

                // if no file is open then open it or create it
                nvim_oxi::api::command(&format!("badd {}", path.to_string_lossy()))
                    .map_err(|e| Error::Internal(e.to_string()))?;
                // check for an open file buffer
                if let Some(mut buffer) = nvim_oxi::api::list_bufs()
                    .into_iter()
                    .find(|b| b.get_name().map(|p| p == path).unwrap_or(false))
                {
                    buffer
                        .set_lines(
                            0..,
                            false,
                            data.content
                                .lines()
                                .map(String::from)
                                .collect::<Vec<String>>(),
                        )
                        .map_err(|e| Error::Internal(e.to_string()))?;
                    buffer
                        .call(|_| {
                            nvim_oxi::api::command("write")
                                .map_err(|e| {
                                    error!(
                                        "Error occurred while attempting to write to file: {:?}",
                                        e
                                    );
                                    e
                                })
                                .ok();
                        })
                        .map_err(|e| Error::Internal(e.to_string()))?;
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
