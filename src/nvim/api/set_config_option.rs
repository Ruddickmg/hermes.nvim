use agent_client_protocol::schema::v1::SessionConfigOptionValue;
use nvim_oxi::{
    Dictionary, Object, ObjectKind,
    conversion::{Error as ConversionError, FromObject},
    lua::Poppable,
};
use tracing::error;

use crate::{
    acp::{Result, error::Error as AcpError},
    api::Api,
};

/// Tuple for two positional arguments: (session_id, config)
pub type SetConfigOptionArgs = (String, SetConfigOptionConfig);

/// The value carried by a config option.
///
/// ACP models these as two shapes: a string `valueId` for select options and a
/// boolean for boolean options. Lua supplies whichever matches the option's
/// `type`, and we forward it verbatim.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigOptionValue {
    ValueId(String),
    Boolean(bool),
}

impl From<ConfigOptionValue> for SessionConfigOptionValue {
    fn from(value: ConfigOptionValue) -> Self {
        match value {
            ConfigOptionValue::ValueId(id) => Self::value_id(id),
            ConfigOptionValue::Boolean(boolean) => Self::boolean(boolean),
        }
    }
}

/// Table with `config_id` and `value` keys for setting any config option.
#[derive(Debug, Clone, PartialEq)]
pub struct SetConfigOptionConfig {
    pub config_id: String,
    pub value: ConfigOptionValue,
}

impl FromObject for SetConfigOptionConfig {
    fn from_object(obj: Object) -> std::result::Result<Self, ConversionError> {
        let dict: Dictionary = obj.try_into()?;

        let config_id: String = dict
            .get("config_id")
            .and_then(|o| o.clone().try_into().ok())
            .map(|s: nvim_oxi::String| s.to_string())
            .ok_or_else(|| {
                ConversionError::Other(
                    "Missing or invalid 'config_id' field in config table".to_string(),
                )
            })?;

        let value_obj = dict.get("value").cloned().ok_or_else(|| {
            ConversionError::Other("Missing 'value' field in config table".to_string())
        })?;

        let value = match value_obj.kind() {
            ObjectKind::Boolean => {
                ConfigOptionValue::Boolean(unsafe { value_obj.as_boolean_unchecked() })
            }
            ObjectKind::String => {
                let s: nvim_oxi::String = value_obj.try_into().map_err(|_| {
                    ConversionError::Other("Invalid string value in config table".to_string())
                })?;
                ConfigOptionValue::ValueId(s.to_string())
            }
            _ => {
                return Err(ConversionError::Other(
                    "'value' must be a string or boolean".to_string(),
                ));
            }
        };

        Ok(Self { config_id, value })
    }
}

impl Poppable for SetConfigOptionConfig {
    unsafe fn pop(
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> std::result::Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| error!("{:?}", e))
            .unwrap_or_default())
    }
}

impl nvim_oxi::lua::Pushable for SetConfigOptionConfig {
    unsafe fn push(
        self,
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> std::result::Result<i32, nvim_oxi::lua::Error> {
        let mut dict = Dictionary::new();
        dict.insert("config_id", self.config_id);
        dict.insert(
            "value",
            match self.value {
                ConfigOptionValue::ValueId(id) => Object::from(id),
                ConfigOptionValue::Boolean(boolean) => Object::from(boolean),
            },
        );
        unsafe { Object::from(dict).push(lua_state) }
    }
}

impl Default for SetConfigOptionConfig {
    fn default() -> Self {
        Self {
            config_id: String::new(),
            value: ConfigOptionValue::ValueId(String::new()),
        }
    }
}

impl Api {
    /// Sets any config option by id, regardless of category or value type.
    ///
    /// This is the primitive underneath the convenience setters like
    /// `set_mode` and `set_model`; use it when the option's `config_id` comes
    /// from `config_options()`, which is the only way to reach boolean options
    /// since their ids are chosen by the agent.
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn set_config_option(&self, (session_id, config): SetConfigOptionArgs) -> Result<()> {
        if config.config_id.is_empty() {
            return Err(AcpError::Internal(
                "Invalid set_config_option argument: 'config_id' must be a non-empty string"
                    .to_string(),
            ));
        }

        let state = self.state.lock().await;
        let details = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| AcpError::SessionNotFound(session_id.clone()))?;
        let is_known = details.has_config_option(&config.config_id);
        drop(state);

        if !is_known {
            return Err(AcpError::InvalidInput(format!(
                "Unknown config_id '{}' for session: {}",
                config.config_id, session_id
            )));
        }

        let connection = self
            .connection
            .get_current_connection()
            .await
            .ok_or_else(|| AcpError::Connection("No connection found".to_string()))?;

        connection
            .set_config_option(
                agent_client_protocol::schema::v1::SetSessionConfigOptionRequest::new(
                    session_id,
                    config.config_id,
                    SessionConfigOptionValue::from(config.value),
                ),
            )
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::SessionConfigKind;
    use pretty_assertions::assert_eq;

    #[test]
    fn config_option_value_from_string_is_value_id() {
        let mut dict = Dictionary::new();
        dict.insert("config_id", "mode");
        dict.insert("value", "plan");
        let obj = Object::from(dict);

        let result = SetConfigOptionConfig::from_object(obj);

        assert_eq!(
            result,
            Ok(SetConfigOptionConfig {
                config_id: "mode".to_string(),
                value: ConfigOptionValue::ValueId("plan".to_string()),
            })
        );
    }

    #[test]
    fn config_option_value_from_boolean_is_boolean() {
        let mut dict = Dictionary::new();
        dict.insert("config_id", "brave_mode");
        dict.insert("value", true);
        let obj = Object::from(dict);

        let result = SetConfigOptionConfig::from_object(obj);

        assert_eq!(
            result,
            Ok(SetConfigOptionConfig {
                config_id: "brave_mode".to_string(),
                value: ConfigOptionValue::Boolean(true),
            })
        );
    }

    #[test]
    fn config_option_value_from_number_is_rejected() {
        let mut dict = Dictionary::new();
        dict.insert("config_id", "brave_mode");
        dict.insert("value", 1);
        let obj = Object::from(dict);

        let result = SetConfigOptionConfig::from_object(obj);

        assert!(result.is_err());
    }

    #[test]
    fn set_config_option_missing_config_id_is_rejected() {
        let mut dict = Dictionary::new();
        dict.insert("value", true);
        let obj = Object::from(dict);

        let result = SetConfigOptionConfig::from_object(obj);

        assert!(result.is_err());
    }

    #[test]
    fn set_config_option_missing_value_is_rejected() {
        let mut dict = Dictionary::new();
        dict.insert("config_id", "brave_mode");
        let obj = Object::from(dict);

        let result = SetConfigOptionConfig::from_object(obj);

        assert!(result.is_err());
    }

    #[test]
    fn string_value_converts_to_value_id() {
        let value: SessionConfigOptionValue = ConfigOptionValue::ValueId("plan".to_string()).into();

        assert_eq!(value, SessionConfigOptionValue::value_id("plan"));
    }

    #[test]
    fn boolean_value_converts_to_boolean() {
        let value: SessionConfigOptionValue = ConfigOptionValue::Boolean(true).into();

        assert_eq!(value, SessionConfigOptionValue::boolean(true));
    }

    #[test]
    fn boolean_value_serializes_with_boolean_type_tag() {
        let value = serde_json::to_value(SessionConfigOptionValue::boolean(true))
            .expect("serialization should succeed");

        assert_eq!(value["type"], serde_json::json!("boolean"));
    }

    #[test]
    fn value_id_value_serializes_as_string() {
        let value = serde_json::to_value(SessionConfigOptionValue::value_id("plan"))
            .expect("serialization should succeed");

        assert_eq!(value["value"], serde_json::json!("plan"));
    }

    /// The `type` discriminator is flattened into the option object itself,
    /// so Lua receives `{ id, name, type = "boolean", currentValue }`.
    #[test]
    fn boolean_option_serializes_with_flattened_type_tag() {
        let option = agent_client_protocol::schema::v1::SessionConfigOption::new(
            "brave_mode",
            "Brave Mode",
            SessionConfigKind::Boolean(
                agent_client_protocol::schema::v1::SessionConfigBoolean::new(true),
            ),
        );
        let json = serde_json::to_value(&option).expect("serialization should succeed");

        assert_eq!(json["type"], serde_json::json!("boolean"));
    }

    #[test]
    fn boolean_option_serializes_current_value_as_bool() {
        let option = agent_client_protocol::schema::v1::SessionConfigOption::new(
            "brave_mode",
            "Brave Mode",
            SessionConfigKind::Boolean(
                agent_client_protocol::schema::v1::SessionConfigBoolean::new(false),
            ),
        );
        let json = serde_json::to_value(&option).expect("serialization should succeed");

        assert_eq!(json["currentValue"], serde_json::json!(false));
    }
}
