pub mod annotations;
pub mod available_commands;
pub mod communication;
pub mod config_option;
pub mod current_mode;
pub mod plan;
pub mod response;
pub mod tool_call;
pub mod tool_call_content;
pub mod tool_call_update;

use agent_client_protocol::Meta;
pub use annotations::*;
pub use available_commands::*;
pub use communication::*;
pub use config_option::*;
pub use current_mode::*;
pub use plan::*;
pub use response::*;
pub use tool_call::*;
pub use tool_call_update::*;

use nvim_oxi::Object;
use serde_json::Value;

pub fn json_to_object(value: Value) -> Option<Object> {
    match value {
        Value::Null => Some(Object::nil()),
        Value::Bool(b) => Some(Object::from(b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Object::from(i))
            } else { n.as_f64().map(|f| Object::from(f as f32)) }
        }
        Value::String(s) => Some(Object::from(s.as_str())),
        Value::Array(arr) => {
            let mut array = nvim_oxi::Array::new();
            for item in arr {
                if let Some(obj) = json_to_object(item) {
                    array.push(obj);
                }
            }
            Some(Object::from(array))
        }
        Value::Object(map) => {
            let mut dict = nvim_oxi::Dictionary::new();
            for (k, v) in map {
                if let Some(obj) = json_to_object(v) {
                    dict.insert(k.as_str(), obj);
                }
            }
            Some(Object::from(dict))
        }
    }
}

pub fn convert_metadata_to_lua_object(value: Option<Meta>) -> Option<Object> {
    if let Some(meta) = value {
        serde_json::to_value(meta).ok().and_then(json_to_object)
    } else {
        None
    }
}
