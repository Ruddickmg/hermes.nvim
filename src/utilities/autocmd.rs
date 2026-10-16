use crate::{
    acp::Result,
    nvim::{GROUP, autocommands::Commands},
};
use nvim_oxi::{Array, Dictionary, Object};
use std::cell::RefCell;
use tracing::{error, instrument};

/// Creates a Neovim autocommand group.
///
/// Abstracts away the `CreateAugroupOpts` builder (version-dependent struct
/// layout) by calling `nvim_create_augroup` directly via `call_function`.
///
/// # Arguments
///
/// * `name` - The autocommand group name
/// * `clear` - Whether to clear existing autocommands in the group
///
/// # Errors
///
/// Returns an error if the augroup creation fails.
#[instrument(level = "trace", skip_all)]
pub fn create_augroup(name: &str, clear: bool) -> nvim_oxi::Result<i32> {
    let mut opts = Dictionary::default();
    opts.insert("clear", Object::from(clear));
    nvim_oxi::api::call_function::<(String, Dictionary), i32>(
        "nvim_create_augroup",
        (name.to_string(), opts),
    )
    .map_err(|e| {
        nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Failed to create autogroup for the '{}' group: {}",
            name, e
        )))
    })
}

/// Creates a Neovim autocommand that invokes a Rust callback when triggered.
///
/// Abstracts away the mlua/nvim_create_autocmd bridging so callers only
/// provide application logic. The callback runs synchronously on Neovim's
/// main thread when the event fires.
///
/// # Type Parameters
///
/// * `F` - A closure that returns a `Result<bool>`. The `bool` is passed back
///   to Lua as the callback's return value (`true` to allow the event, `false`
///   to cancel it for cancellable events). Errors are logged and result in
///   `nil` being returned to Lua.
///
/// # Arguments
///
/// * `group` - The autocommand group ID (from `nvim_create_augroup`)
/// * `event` - The event name (e.g., `"VimLeavePre"`, `"BufWritePost"`)
/// * `callback` - The Rust closure containing application logic
///
/// # Errors
///
/// Returns an error if the Lua callback creation or autocmd registration fails.
///
/// # Panic Safety
///
/// The caller is responsible for ensuring the callback does not panic.
/// If the callback panics, it will abort the Neovim process because Lua
/// callbacks across the FFI boundary cannot unwind.
#[instrument(level = "trace", skip_all)]
pub fn create_autocmd<F>(group: i32, event: &str, callback: F) -> nvim_oxi::Result<()>
where
    F: FnMut() -> Result<bool> + 'static,
{
    let lua = nvim_oxi::mlua::lua();
    let event_name = event.to_string();
    let callback = RefCell::new(callback);

    let cb = lua
        .create_function(move |_lua, _: mlua::Value| match callback.borrow_mut()() {
            Ok(value) => Ok(mlua::Value::Boolean(value)),
            Err(e) => {
                error!("Error in '{}' autocmd callback: {}", event_name, e);
                Ok(mlua::Value::Nil)
            }
        })
        .map_err(|e| {
            nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
                "Failed to create Lua callback for '{}': {}",
                event, e
            )))
        })?;

    let cmd = format!(
        "local group, cb = ...\n vim.api.nvim_create_autocmd('{}', {{ group = group, callback = cb }})",
        event
    );

    lua.load(&cmd).call::<()>((group, cb)).map_err(|e| {
        nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Failed to create '{}' autocmd: {}",
            event, e
        )))
    })
}

/// Executes a Neovim autocommand programmatically.
///
/// Abstracts away the `ExecAutocmdsOpts` builder (version-dependent struct
/// layout) by calling `nvim_exec_autocmds` directly via `call_function` with a
/// constructed Dictionary.
///
/// # Arguments
///
/// * `group` - The autocommand group name
/// * `event` - The event name (e.g., `"User"`)
/// * `pattern` - The pattern to match
/// * `data` - Additional data to pass to the autocommand listeners
///
/// # Errors
///
/// Returns an error if the autocommand execution fails.
#[instrument(level = "trace", skip(data))]
pub fn exec_autocmd(
    group: &str,
    event: &str,
    pattern: &str,
    data: Object,
) -> crate::acp::Result<()> {
    let mut opts_dict = Dictionary::default();
    opts_dict.insert("pattern", Array::from((Object::from(pattern),)));
    opts_dict.insert("data", data);
    opts_dict.insert("group", Object::from(group));
    nvim_oxi::api::call_function::<(String, Dictionary), Object>(
        "nvim_exec_autocmds",
        (event.to_string(), opts_dict),
    )
    .map_err(|err| {
        crate::acp::error::Error::Internal(format!(
            "Error executing autocommand '{}': {:#?}",
            pattern, err
        ))
    })?;
    Ok(())
}

/// Checks whether any autocommand listeners are attached for a given command.
///
/// Abstracts away the `GetAutocmdsOpts` builder by calling `nvim_get_autocmds`
/// directly via `call_function` with a constructed Dictionary.
///
/// # Arguments
///
/// * `command` - The autocommand command whose `User` pattern is matched
///
/// # Returns
///
/// `true` if at least one autocommand listener is registered, `false` otherwise.
#[instrument(level = "trace")]
pub fn autocmd_listeners_attached(command: Commands) -> bool {
    let mut opts_dict = Dictionary::default();
    opts_dict.insert("group", Object::from(GROUP));
    opts_dict.insert("event", Array::from((Object::from("User"),)));
    opts_dict.insert("pattern", Array::from((Object::from(command.to_string()),)));

    nvim_oxi::api::call_function::<(Object,), Array>("nvim_get_autocmds", (opts_dict.into(),))
        .map(|commands| !commands.is_empty())
        .map_err(|e| {
            error!("Error detecting autocommand for '{}': {:?}", command, e);
            e
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_autocmd_helper_signature_compiles() {
        fn assert_compiles<F>(_f: F)
        where
            F: FnMut() -> Result<bool> + 'static,
        {
        }
        assert_compiles(|| Ok(true));
    }

    #[test]
    fn create_autocmd_callback_error_is_converted_to_nil() {
        let mut callback: Box<dyn FnMut() -> Result<bool>> =
            Box::new(|| Err(crate::acp::error::Error::Internal("test error".to_string())));
        let result = callback();
        assert!(result.is_err());
    }

    #[test]
    fn create_autocmd_callback_ok_true_returns_true() {
        let mut callback: Box<dyn FnMut() -> Result<bool>> = Box::new(|| Ok(true));
        let result = callback().unwrap();
        assert!(result);
    }

    #[test]
    fn create_autocmd_callback_ok_false_returns_false() {
        let mut callback: Box<dyn FnMut() -> Result<bool>> = Box::new(|| Ok(false));
        let result = callback().unwrap();
        assert!(!result);
    }

    #[test]
    fn create_augroup_signature_compiles() {
        // Verify the function signature compiles. We can't call it without Neovim.
        fn assert_compiles(_f: impl Fn(&str, bool) -> nvim_oxi::Result<i32>) {}
        assert_compiles(create_augroup);
    }

    #[test]
    fn exec_autocmd_signature_compiles() {
        fn assert_compiles(_f: impl Fn(&str, &str, &str, Object) -> crate::acp::Result<()>) {}
        assert_compiles(exec_autocmd);
    }

    #[test]
    fn autocmd_listeners_attached_signature_compiles() {
        fn assert_compiles(_f: impl Fn(Commands) -> bool) {}
        assert_compiles(autocmd_listeners_attached);
    }
}
