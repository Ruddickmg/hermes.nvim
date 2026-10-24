use hermes::acp::connection::Assistant;
use nvim_oxi::Dictionary;

use crate::utilities::agent;

fn binary_cache_setup_config() -> Option<Dictionary> {
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir().join(format!("hermes-e2e-opencode-{}", pid));
    let mut binary_dict = Dictionary::new();
    binary_dict.insert("path", temp_dir.to_string_lossy().to_string());
    let mut dist_dict = Dictionary::new();
    dist_dict.insert("binary", binary_dict);
    let mut config = Dictionary::new();
    config.insert("distributions", dist_dict);
    Some(config)
}

#[nvim_oxi::test]
#[ignore = "No provider available"]
fn test_opencode_prompt() {
    agent::test_agent_prompt(Assistant::from("opencode"), binary_cache_setup_config()).unwrap();
}

#[nvim_oxi::test]
#[ignore = "No provider available"]
fn test_opencode_session_creation() {
    agent::test_session_creation(Assistant::from("opencode"), binary_cache_setup_config()).unwrap();
}
