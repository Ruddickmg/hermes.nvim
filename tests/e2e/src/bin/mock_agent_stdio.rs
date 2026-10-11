//! E2E mock agent over stdio.
//!
//! Hermes spawns this executable when an e2e test connects with
//! `protocol = "stdio"`. It reuses the in-process `MockAgent` from the e2e
//! library but serves a single ACP connection over this process's
//! stdin/stdout, so only tests that need a command-carrying agent (terminal
//! authentication) have to use it — every other e2e test continues to use the
//! TCP mock.
//!
//! Configuration comes from argv so each spawned process (including the fresh
//! ones hermes starts on reconnect) advertises the right `initialize` response:
//!
//! - `terminal [arg...]`: advertise a terminal auth method whose args are the
//!   remaining argv (e.g. `terminal --device-code`).
//! - anything else (default): advertise an agent auth method.

use agent_client_protocol::schema::v1::{AuthMethod, AuthMethodAgent, AuthMethodTerminal};
use hermes_e2e::utilities::mock_agent::MockAgent;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let method = match args.first().map(String::as_str) {
        Some("terminal") => AuthMethod::Terminal(
            AuthMethodTerminal::new("tui-auth".to_string(), "Terminal Auth".to_string())
                .args(args[1..].to_vec()),
        ),
        _ => AuthMethod::Agent(AuthMethodAgent::new(
            "mock-auth".to_string(),
            "Mock Auth".to_string(),
        )),
    };

    let agent = MockAgent::new();
    {
        let mut config = agent.config().lock().unwrap();
        config.initialize_response = config
            .initialize_response
            .clone()
            .auth_methods(vec![method]);
    }

    let code = match smol::block_on(agent.run_stdio()) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("mock-agent-stdio connection error: {}", e);
            1
        }
    };
    std::process::exit(code);
}
