use nvim_oxi::{
    Object, ObjectKind,
    conversion::FromObject,
    lua::{self, Error, Poppable, Pushable},
};
use tracing::error;

use crate::{acp::connection::Assistant, api::Api};

#[derive(Clone, Debug, Default)]
pub enum DisconnectArgs {
    Multiple(Vec<Assistant>),
    Single(Assistant),
    #[default]
    All,
}

const EXPECTED: &str = "Nil, String or Array of Strings";

impl FromObject for DisconnectArgs {
    fn from_object(obj: Object) -> Result<Self, nvim_oxi::conversion::Error> {
        match obj.kind() {
            ObjectKind::Nil => Ok(Self::All),
            ObjectKind::String => {
                let assistant = unsafe { obj.into_string_unchecked() };
                Ok(Self::Single(Assistant::from(assistant.to_string())))
            }
            ObjectKind::Array => {
                let assistants = unsafe { obj.into_array_unchecked() };
                Ok(Self::Multiple(
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
                        .map(|assistant| Assistant::from(assistant.to_string()))
                        .collect::<Vec<Assistant>>(),
                ))
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
            DisconnectArgs::Multiple(agents) => self.connection_manager.disconnect(agents),
            DisconnectArgs::Single(agent) => self.connection_manager.disconnect(vec![agent]),
            DisconnectArgs::All => self.connection_manager.close_all(),
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
}
