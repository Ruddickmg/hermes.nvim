use agent_client_protocol::PermissionOption;

use crate::acp::{Result, error::Error};

// TODO: I can't figure out how to test this functionality, hopeully someday this can be tested in an automated fashion
/// Show permission request UI with vim.ui.select
/// The callback will be invoked with the selected option_id when user makes a selection
/// Callback is FnOnce since it should only be called once
pub fn show_permission_ui<F>(options: &[PermissionOption], prompt: &str, callback: F) -> Result<()>
where
    F: Fn(String) + 'static,
{
    let lua = nvim_oxi::mlua::lua();

    // Create array of {label, id} tables for Lua
    let items: Vec<mlua::Table> = options
        .iter()
        .map(|opt| match lua.create_table() {
            Ok(table) => {
                if let Err(e) = table.set("label", opt.name.to_string()) {
                    Err(e)
                } else if let Err(e) = table.set("id", opt.option_id.to_string()) {
                    Err(e)
                } else {
                    Ok(table)
                }
            }
            Err(e) => Err(e),
        })
        .collect::<std::result::Result<Vec<mlua::Table>, mlua::Error>>()
        .map_err(|e| Error::Internal(e.to_string()))?;

    let items_array = lua
        .create_sequence_from(items)
        .map_err(|e| Error::Internal(format!("Failed to create Lua array: {}", e)))?;

    let lua_callback = lua
        .create_function(move |_, option_id: String| {
            callback(option_id);
            Ok(())
        })
        .map_err(|e| Error::Internal(format!("Failed to create callback: {}", e)))?;

    // Execute vim.ui.select, passing items, prompt, and callback as arguments
    lua.load(
        r#"
        local items, prompt, cb = ...
        vim.ui.select(items, {
            prompt = prompt,
            format_item = function(item)
                return item.label
            end,
        }, function(choice, idx)
            if choice then
                cb(choice.id)
            else
                cb("")
            end
        end)
    "#,
    )
    .call::<()>((items_array, prompt.to_string(), lua_callback))
    .map_err(|e| Error::Internal(format!("Failed to show permission UI: {}", e)))?;

    Ok(())
}
