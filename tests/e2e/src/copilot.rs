use hermes::acp::connection::Assistant;

use crate::utilities::agent;

#[nvim_oxi::test]
fn test_copilot_prompt() {
    agent::test_agent_prompt(
        Assistant::CustomStdio {
            name: "github-copilot-cli".to_string(),
            command: "copilot".to_string(),
            args: vec!["--acp".to_string()],
        },
        None,
    )
    .unwrap();
}

#[nvim_oxi::test]
fn test_copilot_session_creation() {
    agent::test_session_creation(
        Assistant::CustomStdio {
            name: "github-copilot-cli".to_string(),
            command: "copilot".to_string(),
            args: vec!["--acp".to_string()],
        },
        None,
    )
    .unwrap();
}
