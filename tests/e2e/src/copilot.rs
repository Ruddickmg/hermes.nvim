use hermes::acp::connection::Assistant;

use crate::utilities::agent;

#[nvim_oxi::test]
fn test_copilot_prompt() {
    agent::test_agent_prompt(Assistant::Copilot).unwrap();
}

#[nvim_oxi::test]
fn test_copilot_session_creation() {
    agent::test_session_creation(Assistant::Copilot).unwrap();
}
