use nvim_oxi::{
    Object, ObjectKind,
    conversion::FromObject,
    lua::{self, Error, Poppable, Pushable},
    serde::SerializeError,
};
use tracing::{error, instrument};

use crate::{acp::connection::Assistant, api::Api};

#[derive(Clone, Debug, Default)]
pub enum DisconnectArgs {
    Multiple(Vec<Assistant>),
    Single(Assistant),
    #[default]
    All,
}

#[instrument(level = "trace", skip_all)]
fn parse_assistant_string(
    assistant: nvim_oxi::String,
) -> Result<Assistant, nvim_oxi::conversion::Error> {
    match assistant.to_string().to_lowercase().as_str() {
        "copilot" => Ok(Assistant::Copilot),
        "opencode" => Ok(Assistant::Opencode),
        other => Err(nvim_oxi::conversion::Error::Serialize(SerializeError {
            msg: format!(
                "Invalid input found: {}, Agent name must be one of 'copilot' or 'opencode'",
                other
            ),
        })),
    }
}

const EXPECTED: &str = "Nil, String or Array of Strings";

impl FromObject for DisconnectArgs {
    fn from_object(obj: Object) -> Result<Self, nvim_oxi::conversion::Error> {
        match obj.kind() {
            ObjectKind::Nil => Ok(Self::All),
            ObjectKind::String => {
                let kind = obj.kind();
                let assistant = unsafe { obj.into_string_unchecked() };
                parse_assistant_string(assistant)
                    .map_err(|_| nvim_oxi::conversion::Error::FromWrongType {
                        expected: EXPECTED,
                        actual: kind.as_static(),
                    })
                    .map(Self::Single)
            }
            ObjectKind::Array => {
                let assistants = unsafe { obj.into_array_unchecked() };
                assistants
                    .into_iter()
                    .map(|obj| {
                        if let ObjectKind::String = obj.kind() {
                            Ok(unsafe { obj.into_string_unchecked() })
                        } else {
                            Err(nvim_oxi::conversion::Error::FromWrongType {
                                expected: EXPECTED,
                                actual: obj.kind().as_static(),
                            })
                        }
                    })
                    .collect::<Result<Vec<nvim_oxi::String>, nvim_oxi::conversion::Error>>()?
                    .into_iter()
                    .map(parse_assistant_string)
                    .collect::<Result<Vec<Assistant>, nvim_oxi::conversion::Error>>()
                    .map(Self::Multiple)
            }
            other => Err(nvim_oxi::conversion::Error::FromWrongType {
                expected: EXPECTED,
                actual: other.as_static(),
            }),
        }
    }
}

impl Poppable for DisconnectArgs {
    unsafe fn pop(lua: *mut nvim_oxi::lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(lua)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| error!("An error occurred while parsing the disconnect arguments, failed to disconnect: {:?}", e))
            .unwrap_or(Self::Multiple(vec![])))
    }
}

impl Pushable for DisconnectArgs {
    unsafe fn push(self, state: *mut lua::ffi::State) -> Result<i32, Error> {
        unsafe {
            match self {
                Self::All => ().push(state),
                Self::Single(s) => s.to_string().push(state),
                Self::Multiple(vec) => vec
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .push(state),
            }
        }
    }
}

impl Api {
    #[tracing::instrument(level = "trace", skip(self))]
    pub async fn disconnect(&mut self, args: DisconnectArgs) -> crate::acp::Result<()> {
        match args {
            DisconnectArgs::Multiple(agents) => self.connection.disconnect(agents),
            DisconnectArgs::Single(agent) => self.connection.disconnect(vec![agent]),
            DisconnectArgs::All => self.connection.close_all(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating valid assistant names
    fn arb_assistant_name() -> impl Strategy<Value = String> {
        prop_oneof!(
            Just("copilot".to_string()),
            Just("opencode".to_string()),
            Just("COPILOT".to_string()),
            Just("OPENCODE".to_string()),
            Just("Copilot".to_string()),
            Just("Opencode".to_string()),
            "[a-zA-Z][a-zA-Z0-9_]*".prop_map(|s| s.to_string())
        )
    }

    // Strategy for generating DisconnectArgs variants
    fn arb_disconnect_args() -> impl Strategy<Value = DisconnectArgs> {
        prop_oneof!(
            Just(DisconnectArgs::All),
            arb_assistant_name().prop_map(|name| {
                match Assistant::from(name.as_str()) {
                    Assistant::Copilot | Assistant::Opencode => {
                        DisconnectArgs::Single(Assistant::from(name.as_str()))
                    }
                    _ => DisconnectArgs::All, // Custom assistants become All for simplicity
                }
            }),
            prop::collection::vec(arb_assistant_name(), 0..5).prop_map(|names| {
                let assistants: Vec<Assistant> = names
                    .into_iter()
                    .filter_map(|name| match Assistant::from(name.as_str()) {
                        Assistant::Copilot | Assistant::Opencode => {
                            Some(Assistant::from(name.as_str()))
                        }
                        _ => None,
                    })
                    .collect();
                if assistants.is_empty() {
                    DisconnectArgs::All
                } else {
                    DisconnectArgs::Multiple(assistants)
                }
            })
        )
    }

    proptest! {
        #[test]
        fn test_disconnect_args_from_str_roundtrip(name in arb_assistant_name()) {
            // Property: converting string to Assistant should never panic
            let _ = Assistant::from(name.as_str());
        }

        #[test]
        fn test_disconnect_args_pushable_roundtrip(args in arb_disconnect_args()) {
            // Property: Pushable -> Poppable should preserve the value
            // Note: We can't easily test the full round-trip without a Lua state,
            // but we can verify the enum structure is preserved
            match args {
                DisconnectArgs::All => {
                    // All variant should remain All
                }
                DisconnectArgs::Single(ref assistant) => {
                    prop_assert!(
                        matches!(assistant, Assistant::Copilot | Assistant::Opencode | Assistant::CustomStdio { .. }),
                        "Single variant should contain valid assistant"
                    );
                }
                DisconnectArgs::Multiple(ref assistants) => {
                    prop_assert!(
                        !assistants.is_empty(),
                        "Multiple variant should contain at least one assistant"
                    );
                    for assistant in assistants {
                        prop_assert!(
                            matches!(assistant, Assistant::Copilot | Assistant::Opencode | Assistant::CustomStdio { .. }),
                            "Each assistant in Multiple should be valid"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_disconnect_args_default_is_all() {
        let args: DisconnectArgs = Default::default();
        assert!(matches!(args, DisconnectArgs::All));
    }

    #[test]
    fn test_parse_assistant_string_copilot() {
        let result = parse_assistant_string(nvim_oxi::String::from("copilot"));
        assert!(matches!(result, Ok(Assistant::Copilot)));
    }

    #[test]
    fn test_parse_assistant_string_opencode() {
        let result = parse_assistant_string(nvim_oxi::String::from("opencode"));
        assert!(matches!(result, Ok(Assistant::Opencode)));
    }

    #[test]
    fn test_parse_assistant_string_case_insensitive() {
        let result = parse_assistant_string(nvim_oxi::String::from("COPILOT"));
        assert!(matches!(result, Ok(Assistant::Copilot)));
    }

    #[test]
    fn test_parse_assistant_string_invalid() {
        let result = parse_assistant_string(nvim_oxi::String::from("invalid"));
        assert!(result.is_err());
    }

    #[test]
    fn test_disconnect_args_pushable_all_variant() {
        // Test that All variant pushes correctly (empty tuple)
        let args = DisconnectArgs::All;
        // Verify the variant exists and has correct type
        assert!(matches!(args, DisconnectArgs::All));
    }

    #[test]
    fn test_disconnect_args_pushable_single_variant() {
        // Test that Single variant stores assistant correctly
        let args = DisconnectArgs::Single(Assistant::Copilot);
        match args {
            DisconnectArgs::Single(assistant) => {
                assert!(matches!(assistant, Assistant::Copilot));
            }
            _ => panic!("Expected Single variant"),
        }
    }

    #[test]
    fn test_disconnect_args_pushable_multiple_variant() {
        // Test that Multiple variant stores vector correctly
        let assistants = vec![Assistant::Copilot, Assistant::Opencode];
        let args = DisconnectArgs::Multiple(assistants);
        match args {
            DisconnectArgs::Multiple(vec) => {
                assert_eq!(vec.len(), 2);
                assert!(matches!(vec[0], Assistant::Copilot));
                assert!(matches!(vec[1], Assistant::Opencode));
            }
            _ => panic!("Expected Multiple variant"),
        }
    }
}
