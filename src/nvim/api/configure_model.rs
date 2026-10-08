use nvim_oxi::{
    Dictionary, Object,
    conversion::{Error as ConversionError, FromObject},
    lua::{Poppable, Pushable},
};
use tracing::error;

use crate::{
    acp::{Result, error::Error as AcpError},
    api::Api,
};

/// Tuple for two positional arguments: (session_id, config)
pub type ConfigureModelArgs = (String, ConfigureModelConfig);

/// Table with `id` and `value` keys for configuring a model option.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConfigureModelConfig {
    pub id: String,
    pub value: String,
}

impl FromObject for ConfigureModelConfig {
    fn from_object(obj: Object) -> std::result::Result<Self, ConversionError> {
        let dict: Dictionary = obj.try_into()?;

        let id: String = dict
            .get("id")
            .and_then(|o| o.clone().try_into().ok())
            .map(|s: nvim_oxi::String| s.to_string())
            .ok_or_else(|| {
                ConversionError::Other("Missing or invalid 'id' field in config table".to_string())
            })?;

        let value: String = dict
            .get("value")
            .and_then(|o| o.clone().try_into().ok())
            .map(|s: nvim_oxi::String| s.to_string())
            .ok_or_else(|| {
                ConversionError::Other(
                    "Missing or invalid 'value' field in config table".to_string(),
                )
            })?;

        Ok(Self { id, value })
    }
}

impl Poppable for ConfigureModelConfig {
    unsafe fn pop(
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> std::result::Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua_state)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| error!("{:?}", e))
            .unwrap_or_default())
    }
}

impl Pushable for ConfigureModelConfig {
    unsafe fn push(
        self,
        lua_state: *mut nvim_oxi::lua::ffi::State,
    ) -> std::result::Result<i32, nvim_oxi::lua::Error> {
        let mut dict = Dictionary::new();
        dict.insert("id", self.id);
        dict.insert("value", self.value);
        unsafe { Object::from(dict).push(lua_state) }
    }
}

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn configure_model(&self, (session_id, config): ConfigureModelArgs) -> Result<()> {
        if config.id.is_empty() || config.value.is_empty() {
            return Err(AcpError::Internal(format!(
                "Invalid configure_model argument: 'id' and 'value' must be non-empty strings, got id='{}' and value='{}'",
                config.id, config.value
            )));
        }

        let state = self.state.lock().await;
        let has_config = state
            .session_info
            .get(&session_id)
            .ok_or_else(|| AcpError::SessionNotFound(session_id.clone()))?
            .model_config_options()
            .iter()
            .any(|mc| mc.id == config.id);
        drop(state);

        if !has_config {
            return Err(AcpError::Internal(format!(
                "Model configuration '{}' not found for session: {}",
                config.id, session_id
            )));
        }

        let connection = self
            .connection_manager
            .get_current_connection()
            .await
            .ok_or_else(|| AcpError::Connection("No connection found".to_string()))?;

        connection
            .set_config_option(
                agent_client_protocol::schema::v1::SetSessionConfigOptionRequest::new(
                    session_id,
                    config.id,
                    agent_client_protocol::schema::v1::SessionConfigOptionValue::value_id(
                        config.value,
                    ),
                ),
            )
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn configure_model_config_from_object_valid() {
        let mut dict = Dictionary::new();
        dict.insert("id", "model-opt");
        dict.insert("value", "new-value");
        let obj = Object::from(dict);

        let result = ConfigureModelConfig::from_object(obj);

        assert_eq!(
            result,
            Ok(ConfigureModelConfig {
                id: "model-opt".to_string(),
                value: "new-value".to_string(),
            })
        );
    }

    #[test]
    fn configure_model_config_from_object_missing_id() {
        let mut dict = Dictionary::new();
        dict.insert("value", "new-value");
        let obj = Object::from(dict);

        let result = ConfigureModelConfig::from_object(obj);

        assert!(result.is_err());
    }

    #[test]
    fn configure_model_config_from_object_missing_value() {
        let mut dict = Dictionary::new();
        dict.insert("id", "model-opt");
        let obj = Object::from(dict);

        let result = ConfigureModelConfig::from_object(obj);

        assert!(result.is_err());
    }
}
