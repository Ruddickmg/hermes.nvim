#[cfg(test)]
use std::collections::BTreeMap;

use agent_client_protocol::schema::v1::{
    CreateElicitationRequest, CreateElicitationResponse, ElicitationAcceptAction,
    ElicitationAction, ElicitationContentValue, ElicitationMode, ElicitationPropertySchema,
    MultiSelectItems, StringPropertySchema,
};
#[cfg(test)]
use agent_client_protocol::schema::v1::{
    EnumOption, OtherMultiSelectItems, StringMultiSelectItems, TitledMultiSelectItems,
};
use nvim_oxi::conversion::FromObject;

use crate::acp::Result;
use crate::acp::error::Error;
use crate::nvim::configuration::dict_from_object;

use super::request::Request;

impl Request {
    pub(super) fn parse_content_value(data: nvim_oxi::Object) -> Result<ElicitationContentValue> {
        if let Ok(s) = String::from_object(data.clone()) {
            return Ok(ElicitationContentValue::String(s));
        }
        if let Ok(i) = i64::from_object(data.clone()) {
            return Ok(ElicitationContentValue::Integer(i));
        }
        if let Ok(n) = f64::from_object(data.clone()) {
            return Ok(ElicitationContentValue::Number(n));
        }
        if let Ok(b) = bool::from_object(data.clone()) {
            return Ok(ElicitationContentValue::Boolean(b));
        }
        if let Ok(arr) = <Vec<String>>::from_object(data.clone()) {
            return Ok(ElicitationContentValue::StringArray(arr));
        }
        Err(Error::InvalidInput(
            "Unsupported content value in elicitation response".to_string(),
        ))
    }

    pub(super) fn allowed_string_values(schema: &StringPropertySchema) -> Option<Vec<String>> {
        if let Some(values) = &schema.enum_values {
            return Some(values.clone());
        }
        if let Some(options) = &schema.one_of {
            return Some(options.iter().map(|o| o.value.clone()).collect());
        }
        None
    }

    pub(super) fn allowed_array_values(items: &MultiSelectItems) -> Option<Vec<String>> {
        match items {
            MultiSelectItems::String(s) => Some(s.values.clone()),
            MultiSelectItems::Titled(t) => {
                Some(t.options.iter().map(|o| o.value.clone()).collect())
            }
            MultiSelectItems::Other(_) => None,
            _ => None,
        }
    }

    pub(super) async fn validate_content_value(
        &self,
        value: nvim_oxi::Object,
        schema: &ElicitationPropertySchema,
    ) -> Result<ElicitationContentValue> {
        let reject_unknown = self
            .state
            .lock()
            .await
            .config
            .permissions
            .elicitation
            .reject_unknown_elicitation_values;
        match schema {
            ElicitationPropertySchema::String(prop) => {
                let s = String::from_object(value)
                    .map_err(|_| Error::InvalidInput("Expected string value".to_string()))?;
                if let Some(allowed) = Self::allowed_string_values(prop) {
                    if !allowed.contains(&s) {
                        return Err(Error::InvalidInput(format!(
                            "Value '{}' is not an allowed enum option",
                            s
                        )));
                    }
                }
                Ok(ElicitationContentValue::String(s))
            }
            ElicitationPropertySchema::Integer(_) => {
                if let Ok(i) = i64::from_object(value.clone()) {
                    return Ok(ElicitationContentValue::Integer(i));
                }
                if let Ok(n) = f64::from_object(value.clone())
                    && n.fract() == 0.0
                    && n >= i64::MIN as f64
                    && n <= i64::MAX as f64
                {
                    return Ok(ElicitationContentValue::Integer(n as i64));
                }
                Err(Error::InvalidInput("Expected integer value".to_string()))
            }
            ElicitationPropertySchema::Number(_) => {
                if let Ok(n) = f64::from_object(value.clone()) {
                    return Ok(ElicitationContentValue::Number(n));
                }
                if let Ok(i) = i64::from_object(value.clone()) {
                    return Ok(ElicitationContentValue::Integer(i));
                }
                Err(Error::InvalidInput("Expected number value".to_string()))
            }
            ElicitationPropertySchema::Boolean(_) => {
                let b = bool::from_object(value)
                    .map_err(|_| Error::InvalidInput("Expected boolean value".to_string()))?;
                Ok(ElicitationContentValue::Boolean(b))
            }
            ElicitationPropertySchema::Array(prop) => {
                let arr: Vec<String> = <Vec<String>>::from_object(value)
                    .map_err(|_| Error::InvalidInput("Expected array of strings".to_string()))?;
                if let Some(allowed) = Self::allowed_array_values(&prop.items) {
                    for s in &arr {
                        if !allowed.contains(s) {
                            return Err(Error::InvalidInput(format!(
                                "Value '{}' is not an allowed enum option",
                                s
                            )));
                        }
                    }
                }
                Ok(ElicitationContentValue::StringArray(arr))
            }
            _ => {
                if reject_unknown {
                    return Err(Error::InvalidInput(
                        "Unknown elicitation property type was rejected".to_string(),
                    ));
                }
                Self::parse_content_value(value)
            }
        }
    }

    pub(super) async fn parse_elicitation_response(
        &self,
        request: &CreateElicitationRequest,
        data: nvim_oxi::Object,
    ) -> Result<CreateElicitationResponse> {
        let dict = dict_from_object(data).map_err(|e| Error::InvalidInput(e.to_string()))?;

        let action = dict
            .get("action")
            .cloned()
            .ok_or(Error::InvalidInput(
                "Missing 'action' field in elicitation response".to_string(),
            ))
            .and_then(|o| String::from_object(o).map_err(|e| Error::InvalidInput(e.to_string())))?;

        match action.as_str() {
            "accept" => {
                let accept = match &request.mode {
                    ElicitationMode::Form(form_mode) => {
                        let content_dict = match dict.get("content").cloned() {
                            Some(content_obj) => dict_from_object(content_obj)
                                .map_err(|e| Error::InvalidInput(e.to_string()))?,
                            None => nvim_oxi::Dictionary::default(),
                        };
                        let schema = &form_mode.requested_schema;
                        let mut content = std::collections::BTreeMap::new();
                        for (key, value) in content_dict {
                            let key: String = key.to_string();
                            if let Some(prop_schema) = schema.properties.get(&key) {
                                let parsed = self
                                    .validate_content_value(value, prop_schema)
                                    .await
                                    .map_err(|e| Error::InvalidInput(e.to_string()))?;
                                content.insert(key, parsed);
                            }
                        }
                        if let Some(required) = &schema.required {
                            for field in required {
                                if !content.contains_key(field) {
                                    return Err(Error::InvalidInput(format!(
                                        "Missing required elicitation field '{}'",
                                        field
                                    )));
                                }
                            }
                        }
                        // Only include the content map if it's non-empty. An empty
                        // content should be represented as `None` on the accept
                        // action so callers can distinguish between "no content"
                        // and "empty content".
                        if content.is_empty() {
                            ElicitationAcceptAction::new()
                        } else {
                            ElicitationAcceptAction::new().content(content)
                        }
                    }
                    _ => ElicitationAcceptAction::new(),
                };
                Ok(CreateElicitationResponse::new(ElicitationAction::Accept(
                    accept,
                )))
            }
            "decline" => Ok(CreateElicitationResponse::new(ElicitationAction::Decline)),
            "cancel" => Ok(CreateElicitationResponse::new(ElicitationAction::Cancel)),
            _ => Err(Error::InvalidInput(format!(
                "Unknown elicitation action: '{}'",
                action
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvim_oxi::Object;

    #[test]
    fn parse_content_value_accepts_string() {
        let result = Request::parse_content_value(Object::from("hello"));
        assert_eq!(
            result.unwrap(),
            ElicitationContentValue::String("hello".to_string())
        );
    }

    #[test]
    fn parse_content_value_accepts_integer() {
        let result = Request::parse_content_value(Object::from(42i64));
        assert_eq!(result.unwrap(), ElicitationContentValue::Integer(42));
    }

    #[test]
    fn parse_content_value_accepts_boolean() {
        let result = Request::parse_content_value(Object::from(true));
        assert_eq!(result.unwrap(), ElicitationContentValue::Boolean(true));
    }

    #[test]
    fn parse_content_value_accepts_number() {
        let result = Request::parse_content_value(Object::from(3.14f64));
        assert_eq!(result.unwrap(), ElicitationContentValue::Number(3.14));
    }

    #[test]
    fn parse_content_value_accepts_string_array() {
        let arr = nvim_oxi::Array::from_iter(vec![Object::from("a"), Object::from("b")]);
        let result = Request::parse_content_value(Object::from(arr));
        assert_eq!(
            result.unwrap(),
            ElicitationContentValue::StringArray(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn parse_content_value_rejects_unsupported_type() {
        let mut nested = nvim_oxi::Dictionary::default();
        nested.insert("field", Object::from("x"));
        let result = Request::parse_content_value(Object::from(nested));
        assert!(result.is_err());
    }

    #[test]
    fn allowed_string_values_returns_enum_values_when_present() {
        let schema =
            StringPropertySchema::new().enum_values(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            Request::allowed_string_values(&schema),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn allowed_string_values_prefers_enum_values_over_one_of() {
        let schema = StringPropertySchema::new()
            .enum_values(vec!["a".to_string()])
            .one_of(vec![EnumOption::new("b", "B")]);
        assert_eq!(
            Request::allowed_string_values(&schema),
            Some(vec!["a".to_string()])
        );
    }

    #[test]
    fn allowed_string_values_returns_one_of_values_when_present() {
        let schema = StringPropertySchema::new().one_of(vec![
            EnumOption::new("us", "United States"),
            EnumOption::new("gb", "United Kingdom"),
        ]);
        assert_eq!(
            Request::allowed_string_values(&schema),
            Some(vec!["us".to_string(), "gb".to_string()])
        );
    }

    #[test]
    fn allowed_string_values_returns_none_without_restrictions() {
        let schema = StringPropertySchema::new();
        assert_eq!(Request::allowed_string_values(&schema), None);
    }

    #[test]
    fn allowed_array_values_returns_string_values() {
        let items = MultiSelectItems::String(StringMultiSelectItems::new(vec![
            "a".to_string(),
            "b".to_string(),
        ]));
        assert_eq!(
            Request::allowed_array_values(&items),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn allowed_array_values_returns_titled_option_values() {
        let items = MultiSelectItems::Titled(TitledMultiSelectItems::new(vec![EnumOption::new(
            "us",
            "United States",
        )]));
        assert_eq!(
            Request::allowed_array_values(&items),
            Some(vec!["us".to_string()])
        );
    }

    #[test]
    fn allowed_array_values_returns_none_for_unknown_items() {
        let items = MultiSelectItems::Other(OtherMultiSelectItems::new("custom", BTreeMap::new()));
        assert_eq!(Request::allowed_array_values(&items), None);
    }
}
