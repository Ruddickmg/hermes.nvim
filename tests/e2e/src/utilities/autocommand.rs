use hermes::nvim::autocommands::Commands;
use nvim_oxi::api;
use nvim_oxi::{Object, serde::Deserializer};
use serde::de::DeserializeOwned;
use std::{
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tracing::error;

pub fn nvim_object_to_struct<T>(obj: Object) -> Result<T, nvim_oxi::Error>
where
    T: DeserializeOwned,
{
    T::deserialize(Deserializer::new(obj))
        .map_err(|e| nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(e.to_string())))
}

static LISTENER_ID: AtomicU64 = AtomicU64::new(0);

/// Execute a Lua code string via mlua and return the result as an nvim Object.
fn exec_lua(code: &str) -> Result<Object, nvim_oxi::Error> {
    let lua = nvim_oxi::mlua::lua();
    let value: mlua::Value = lua.load(code).eval().map_err(|e| {
        nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(format!(
            "Lua eval error: {}",
            e
        )))
    })?;
    Ok(lua_value_to_object(value))
}

/// Convert an mlua Value into an nvim_oxi Object.
fn lua_value_to_object(value: mlua::Value) -> Object {
    match value {
        mlua::Value::Nil => Object::nil(),
        mlua::Value::Boolean(b) => Object::from(b),
        mlua::Value::Integer(i) => Object::from(i),
        mlua::Value::Number(n) => Object::from(n),
        mlua::Value::String(s) => match s.to_str() {
            Ok(str_val) => Object::from(str_val.to_owned().as_str()),
            Err(_) => Object::nil(),
        },
        mlua::Value::Table(t) => {
            // Check if the table is array-like (consecutive integer keys starting at 1)
            let len = t.raw_len();
            let is_array = len > 0 && {
                let mut all_int = true;
                for pair in t.clone().pairs::<mlua::Value, mlua::Value>() {
                    if let Ok((mlua::Value::Integer(_), _)) = pair {
                        continue;
                    }
                    all_int = false;
                    break;
                }
                all_int
            };

            if is_array {
                let mut arr = nvim_oxi::Array::new();
                for i in 1..=len {
                    if let Ok(v) = t.raw_get::<mlua::Value>(i) {
                        arr.push(lua_value_to_object(v));
                    }
                }
                Object::from(arr)
            } else {
                let mut dict = nvim_oxi::Dictionary::new();
                for pair in t.pairs::<mlua::Value, mlua::Value>() {
                    if let Ok((key, val)) = pair {
                        let key_str = match key {
                            mlua::Value::String(s) => match s.to_str() {
                                Ok(str_val) => str_val.to_string(),
                                Err(_) => continue,
                            },
                            mlua::Value::Integer(i) => i.to_string(),
                            _ => continue,
                        };
                        dict.insert(nvim_oxi::String::from(key_str), lua_value_to_object(val));
                    }
                }
                Object::from(dict)
            }
        }
        _ => Object::nil(),
    }
}

pub fn listen_for_autocommand<T>(
    autocommand: Commands,
) -> Box<dyn Fn(Duration) -> Result<T, nvim_oxi::Error>>
where
    T: Debug + DeserializeOwned + Send + Clone + 'static,
{
    let pattern = autocommand.to_string();
    let id = LISTENER_ID.fetch_add(1, Ordering::Relaxed);
    let data_var = format!("_hermes_e2e_data_{}", id);

    // Use mlua to execute Lua directly, avoiding both the CreateAutocmdOpts
    // builder (version-dependent struct layout) and nvim_exec_lua (not
    // callable via nvim_call_function).
    let lua_code = format!(
        r#"vim.api.nvim_create_autocmd("User",{{pattern={{"{}"}},group="hermes",callback=function(a)_G["{}"]=a.data end}})"#,
        pattern, data_var
    );

    exec_lua(&lua_code).expect("Failed to create autocommand listener via mlua");

    Box::new(move |duration| {
        let start = Instant::now();
        loop {
            let lua_get = format!(r#"return _G["{}"]"#, data_var);
            let data: Object = match exec_lua(&lua_get) {
                Ok(d) => d,
                Err(e) => {
                    error!("Error polling autocmd data: {:#?}", e);
                    Object::nil()
                }
            };

            if !data.is_nil() {
                let lua_clear = format!(r#"_G["{}"] = nil"#, data_var);
                let _ = exec_lua(&lua_clear);
                return nvim_object_to_struct(data);
            }

            if start.elapsed() > duration {
                return Err(nvim_oxi::Error::Api(nvim_oxi::api::Error::Other(
                    "Timed out waiting for Autocmd".into(),
                )));
            }
            // The sleep is only used to pump Neovim's event loop. An unrelated
            // error notification (e.g. from a background thread) can interrupt
            // it, so don't treat that as a failure of the wait itself.
            let _ = api::command("sleep 100m");
        }
    })
}
