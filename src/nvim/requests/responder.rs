use crate::acp::{Result, error::Error};
use agent_client_protocol::{
    PermissionOption, RequestPermissionOutcome, RequestPermissionRequest, WriteTextFileRequest,
    WriteTextFileResponse,
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

// TODO: move these helper functions into a "utilities" directory
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

/// Show permission request UI with vim.ui.select
/// The callback will be invoked with the selected option_id when user makes a selection
/// Callback is FnOnce since it should only be called once
pub fn show_permission_ui<F>(options: &[PermissionOption], prompt: &str, callback: F) -> Result<()>
where
    F: FnOnce(String) + 'static,
{
    let lua = nvim_oxi::mlua::lua();

    // Create array of {label, id} tables for Lua
    let items: Vec<mlua::Table> = options
        .iter()
        .map(|opt| {
            let table = lua.create_table().unwrap();
            table.set("label", opt.name.to_string()).unwrap();
            table.set("id", opt.option_id.to_string()).unwrap();
            table
        })
        .collect();

    let items_array = lua
        .create_sequence_from(items)
        .map_err(|e| Error::Internal(format!("Failed to create Lua array: {}", e)))?;

    // Wrap FnOnce in RefCell<Option> so we can take() and consume it once
    let callback_opt = std::cell::RefCell::new(Some(callback));

    // Create Lua function from the Rust closure
    // take() removes the callback from Option and consumes it
    let lua_callback = lua
        .create_function(move |_, option_id: String| {
            if let Some(cb) = callback_opt.borrow_mut().take() {
                cb(option_id);
            }
            Ok(())
        })
        .map_err(|e| Error::Internal(format!("Failed to create callback: {}", e)))?;

    // Store callback in a global variable so Lua can access it
    lua.globals()
        .set("__hermes_permission_callback", lua_callback)
        .map_err(|e| Error::Internal(format!("Failed to set callback: {}", e)))?;

    // Execute vim.ui.select - callback is accessed via global
    lua.load(format!(
        r#"
        local items = ...
        vim.ui.select(items, {{
            prompt = "{}",
            format_item = function(item) 
                return item.label 
            end,
        }}, function(choice, idx)
            if choice then
                __hermes_permission_callback(choice.id)
            end
        end)
    "#,
        prompt
    ))
    .call::<()>(items_array)
    .map_err(|e| Error::Internal(format!("Failed to show permission UI: {}", e)))?;

    Ok(())
}
