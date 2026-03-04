use core::fmt;
use std::fmt::{Debug, Display};

use nvim_oxi::{
    Object,
    api::opts::{EchoOpts, ExecAutocmdsOpts},
};

mod event;
mod response;

pub use response::*;

#[derive(Clone)]
pub struct AutoCommands {
    group: String,
}

impl AutoCommands {
    pub fn new(group: String) -> Self {
        Self { group }
    }

    async fn schedule_autocommand<T: ToString>(&self, command: T, data: Object) {
        let group = self.group.clone();
        let command = command.to_string();

        println!("command: {:?}", command.to_string());
        nvim_oxi::schedule(move |_| {
            println!("scheduling autocommand '{}'", command);
            let opts = ExecAutocmdsOpts::builder().data(data).group(group).build();
            let echo_opts = EchoOpts::default();

            if let Err(err) = nvim_oxi::api::echo([("Hello from nvim-oxi!", None::<String>)], true, &echo_opts) {
                eprintln!("Error echoing: {}", err);
            }

            if let Err(err) = nvim_oxi::api::exec_autocmds([command.as_str()], &opts) {
            //     eprintln!("Error executing autocommand '{}': {:?}", command, err);
            }
        });
    }
}

impl Default for AutoCommands {
    fn default() -> Self {
        Self {
            group: "hermes".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum Commands {
    AgentConnectionInitialized,
    CreatedSession,
}

impl Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}
