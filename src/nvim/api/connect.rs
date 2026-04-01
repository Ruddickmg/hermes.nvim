use crate::{
    Handler,
    acp::connection::{Assistant, ConnectionDetails, ConnectionManager, Protocol},
};
use nvim_oxi::{Dictionary, Function, Object, ObjectKind, lua::Error};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use tracing::{debug, error, instrument};

pub type ConnectionArgs = (nvim_oxi::String, Option<Dictionary>);

#[instrument(level = "trace", skip_all)]
pub fn connect(connection: Rc<RefCell<ConnectionManager>>, handler: Arc<Handler>) -> Object {
    let function: Function<ConnectionArgs, Result<(), Error>> = Function::from_fn(
        move |(agent_name, options): ConnectionArgs| -> Result<(), Error> {
            debug!(
                "Connect function called with agent: {:?}, options: {:?}",
                agent_name, options
            );

            let agent_name_str = agent_name.to_string();

            // Parse options
            let mut protocol = None;
            let mut command = None;
            let mut args = None;

            if let Some(dict) = options {
                // Parse protocol
                if let Some(obj) = dict.get("protocol") {
                    let s: nvim_oxi::String = match obj.clone().try_into() {
                        Ok(v) => v,
                        Err(e) => {
                            error!("Invalid protocol: {}", e);
                            return Ok(());
                        }
                    };
                    protocol = Some(Protocol::from(s.to_string()));
                }

                // Parse command
                if let Some(obj) = dict.get("command") {
                    let s: nvim_oxi::String = match obj.clone().try_into() {
                        Ok(v) => v,
                        Err(e) => {
                            error!("Invalid command: {}", e);
                            return Ok(());
                        }
                    };
                    command = Some(s.to_string());
                }

                // Parse args
                if let Some(obj) = dict.get("args")
                    && obj.kind() == ObjectKind::Array
                {
                    let arr: nvim_oxi::Array = unsafe { obj.clone().into_array_unchecked() };
                    let parsed_args: Vec<String> = arr
                        .into_iter()
                        .filter_map(|v| v.try_into().ok().map(|s: nvim_oxi::String| s.to_string()))
                        .collect();
                    args = Some(parsed_args);
                }
            }
            let agent = if let Some(ref cmd) = command {
                Assistant::Custom {
                    name: agent_name_str,
                    command: cmd.clone(),
                    args: args.clone().unwrap_or_default(),
                }
            } else {
                Assistant::from(agent_name_str.clone())
            };

            // Validate that unknown agents without an explicit command don't result in an empty command
            if let Assistant::Custom { ref command, .. } = agent
                && command.is_empty()
            {
                error!(
                    "Unknown agent name; please provide 'command' (and optionally 'args') when connecting to a custom assistant"
                );
                return Ok(());
            }
            let details = ConnectionDetails {
                agent,
                protocol: protocol.unwrap_or_default(),
            };

            let mut conn = match connection.try_borrow_mut() {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to borrow connection manager: {}", e);
                    return Ok(());
                }
            };

            if let Err(e) = conn.connect(handler.clone(), details) {
                error!("Error connecting: {:?}", e);
            }

            drop(conn);
            Ok(())
        },
    );
    function.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating agent names
    fn arb_agent_name() -> impl Strategy<Value = String> {
        prop_oneof!(
            Just("copilot".to_string()),
            Just("opencode".to_string()),
            Just("COPILOT".to_string()),
            Just("OPENCODE".to_string()),
            Just("Copilot".to_string()),
            Just("Opencode".to_string()),
            "[a-zA-Z][a-zA-Z0-9_-]*".prop_map(|s| s.to_string())
        )
    }

    proptest! {
        #[test]
        fn test_assistant_from_str_never_panics(name in arb_agent_name()) {
            // Property: converting any string to Assistant should never panic
            let _ = Assistant::from(name.as_str());
        }

        #[test]
        fn test_known_agents_parsed_correctly(name in prop_oneof!(
            Just("copilot"),
            Just("opencode"),
            Just("COPILOT"),
            Just("OPENCODE")
        )) {
            // Property: Known agents should parse to their respective variants
            let assistant = Assistant::from(name);
            let name_lower = name.to_lowercase();

            if name_lower == "copilot" {
                prop_assert!(matches!(assistant, Assistant::Copilot), "Expected Copilot variant");
            } else if name_lower == "opencode" {
                prop_assert!(matches!(assistant, Assistant::Opencode), "Expected Opencode variant");
            }
        }
    }

    #[test]
    fn test_assistant_from_str_copilot() {
        assert!(matches!(Assistant::from("copilot"), Assistant::Copilot));
    }

    #[test]
    fn test_assistant_from_str_opencode() {
        assert!(matches!(Assistant::from("opencode"), Assistant::Opencode));
    }

    #[test]
    fn test_assistant_from_str_case_insensitive() {
        assert!(matches!(Assistant::from("COPILOT"), Assistant::Copilot));
        assert!(matches!(Assistant::from("OpEnCoDe"), Assistant::Opencode));
    }

    #[test]
    fn test_assistant_custom_with_command() {
        let assistant = Assistant::from("custom-agent");
        assert!(matches!(assistant, Assistant::Custom { name, .. } if name == "custom-agent"));
    }
}
