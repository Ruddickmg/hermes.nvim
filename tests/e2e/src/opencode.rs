use hermes::acp::connection::Assistant;

use crate::utilities::agent;

#[nvim_oxi::test]
fn test_opencode_prompt() {
    agent::test_agent_prompt(Assistant::Opencode).unwrap();
}

#[nvim_oxi::test]
fn test_opencode_session_creation() {
    agent::test_session_creation(Assistant::Opencode).unwrap();
}
