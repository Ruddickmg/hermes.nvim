pub mod communication;
pub mod response;

use agent_client_protocol::Meta;
pub use communication::*;
pub use response::*;

use nvim_oxi::Object;
use serde_json::Value;

pub fn json_to_object(value: Value) -> Option<Object> {
    match value {
        Value::Null => Some(Object::nil()),
        Value::Bool(b) => Some(Object::from(b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Object::from(i))
            } else {
                n.as_f64().map(|f| Object::from(f as f32))
            }
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
