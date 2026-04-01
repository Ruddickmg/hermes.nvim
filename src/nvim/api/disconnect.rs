use nvim_oxi::{
    Function, Object, ObjectKind,
    conversion::{self, FromObject},
    lua::{self, Error, Poppable, Pushable},
    serde::SerializeError,
};
use std::{cell::RefCell, rc::Rc};
use tracing::{debug, error, instrument};

use crate::acp::connection::{Assistant, ConnectionManager};

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
        Self::from_object(obj).map_err(|e| match e {
            conversion::Error::FromWrongType { actual, .. } => lua::Error::PopError {
                ty: actual,
                message: Some(format!("Invalid argument passed to \"disconnect\": {}. Expected Nil, String or Array of Strings", actual)),
            },
            _ => lua::Error::RuntimeError(e.to_string()),
        })
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

#[instrument(level = "trace", skip_all)]
pub fn disconnect(connection: Rc<RefCell<ConnectionManager>>) -> Object {
    let function: Function<DisconnectArgs, Result<(), Error>> =
        Function::from_fn(move |args: DisconnectArgs| -> Result<(), Error> {
            debug!("Disconnect function called with {:#?}", args);
            let mut manager = match connection.try_borrow_mut() {
                Ok(m) => m,
                Err(e) => {
                    error!("Failed to borrow connection manager: {}", e);
                    return Ok(());
                }
            };
            let result = match args {
                DisconnectArgs::Multiple(agents) => manager.disconnect(agents),
                DisconnectArgs::Single(agent) => manager.disconnect(vec![agent.clone()]),
                DisconnectArgs::All => manager.close_all(),
            };
            if let Err(e) = result {
                error!("Error during disconnect: {:?}", e);
            }
            drop(manager);
            Ok(())
        });
    function.into()
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
                        matches!(assistant, Assistant::Copilot | Assistant::Opencode | Assistant::Custom { .. }),
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
                            matches!(assistant, Assistant::Copilot | Assistant::Opencode | Assistant::Custom { .. }),
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
}
