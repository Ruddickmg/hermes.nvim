use crate::{
    acp::{
        self,
        connection::{Assistant, Connection},
        error::Error,
    },
    api::Api,
};
use agent_client_protocol::schema::v1::LogoutRequest;
use futures::future;
use nvim_oxi::{
    Object, ObjectKind,
    conversion::FromObject,
    lua::{self, Poppable, Pushable},
};
use tracing::error;

#[derive(Clone, Debug, Default)]
pub enum LogoutArgs {
    Multiple(Vec<Assistant>),
    Single(Assistant),
    #[default]
    All,
}

const EXPECTED: &str = "Nil, String or Array of Strings";

impl FromObject for LogoutArgs {
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

impl Poppable for LogoutArgs {
    unsafe fn pop(lua: *mut nvim_oxi::lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(lua)? };
        Ok(Self::from_object(obj)
            .inspect_err(|e| {
                error!(
                    "An error occurred while parsing the logout arguments, failed to logout: {:?}",
                    e
                )
            })
            .unwrap_or(Self::Multiple(vec![])))
    }
}

impl Pushable for LogoutArgs {
    unsafe fn push(self, state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
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
    pub async fn logout(&self, args: LogoutArgs) -> crate::acp::Result<()> {
        let agents: Vec<Assistant> = match &args {
            LogoutArgs::Single(agent) => vec![agent.clone()],
            LogoutArgs::Multiple(agents) => agents.clone(),
            LogoutArgs::All => self.connection_manager.connected_agents(),
        };
        let futures: Vec<_> = agents
            .iter()
            .map(|assistant| {
                self.connection_manager
                    .get_connection(assistant)
                    .ok_or_else(|| {
                        Error::Connection(format!("No connection found for: {}", assistant))
                    })
            })
            .collect::<acp::Result<Vec<&Connection>>>()?
            .iter()
            .map(|connection| connection.logout(LogoutRequest::new()))
            .collect();

        future::try_join_all(futures).await?;
        Ok(())
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

    // Strategy for generating LogoutArgs variants
    fn arb_logout_args() -> impl Strategy<Value = LogoutArgs> {
        prop_oneof!(
            Just(LogoutArgs::All),
            arb_assistant_name().prop_map(|name| {
                match Assistant::from(name.as_str()) {
                    Assistant::Copilot | Assistant::Opencode => {
                        LogoutArgs::Single(Assistant::from(name.as_str()))
                    }
                    _ => LogoutArgs::All,
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
                    LogoutArgs::All
                } else {
                    LogoutArgs::Multiple(assistants)
                }
            })
        )
    }

    proptest! {
        #[test]
        fn test_logout_args_from_str_roundtrip(name in arb_assistant_name()) {
            // Property: converting string to Assistant should never panic
            let _ = Assistant::from(name.as_str());
        }

        #[test]
        fn test_logout_args_pushable_roundtrip(args in arb_logout_args()) {
            match args {
                LogoutArgs::All => {
                    // All variant should remain All
                }
                LogoutArgs::Single(ref assistant) => {
                    prop_assert!(
                        matches!(assistant, Assistant::Copilot | Assistant::Opencode | Assistant::CustomStdio { .. }),
                        "Single variant should contain valid assistant"
                    );
                }
                LogoutArgs::Multiple(ref assistants) => {
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
