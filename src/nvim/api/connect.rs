use crate::{
    acp::connection::{Assistant, ConnectionDetails, ConnectionManager, Protocol},
    nvim::autocommands::ResponseHandler,
};
use agent_client_protocol::Client;
use nvim_oxi::{Dictionary, Function, Object, ObjectKind, lua::Error};
use std::rc::Rc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

pub type ConnectionArgs = (nvim_oxi::String, Option<Dictionary>);

#[instrument(level = "trace", skip_all)]
pub fn connect<H: Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
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
                    let s: nvim_oxi::String = obj
                        .clone()
                        .try_into()
                        .map_err(|e| Error::RuntimeError(format!("Invalid protocol: {}", e)))?;
                    protocol = Some(Protocol::from(s.to_string()));
                }

                // Parse command
                if let Some(obj) = dict.get("command") {
                    let s: nvim_oxi::String = obj
                        .clone()
                        .try_into()
                        .map_err(|e| Error::RuntimeError(format!("Invalid command: {}", e)))?;
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
            let mut agent = if let Some(ref cmd) = command {
                Assistant::Custom {
                    name: agent_name_str,
                    command: cmd.clone(),
                    args: args.clone().unwrap_or_default(),
                }
            } else {
                Assistant::from(agent_name_str.clone())
            };

            // Validate that unknown agents without an explicit command don't result in an empty command
            if let Assistant::Custom { ref command, .. } = agent {
                if command.is_empty() {
                    return Err(Error::RuntimeError(
                        "Unknown agent name; please provide 'command' (and optionally 'args') when connecting to a custom assistant".into(),
                    ));
                }
            }
            let details = ConnectionDetails {
                agent,
                protocol: protocol.unwrap_or_default(),
            };
            connection.blocking_lock().connect(details)?;
            Ok(())
        },
    );
    function.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
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
