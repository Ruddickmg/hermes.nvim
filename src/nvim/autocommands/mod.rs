use agent_client_protocol::Error;
use nvim_oxi::{Object, api::opts::ExecAutocmdsOpts};

mod event;
mod response;

#[derive(Clone)]
pub struct AutoCommands {
    group: String,
}

impl AutoCommands {
    pub fn new(group: String) -> Self {
        Self { group }
    }

    fn schedule_autocommand<T: ToString>(&self, command: T, data: Object) {
        let group = self.group.clone();
        let command = command.to_string();
        let opts = ExecAutocmdsOpts::builder().data(data).group(group).build();
        nvim_oxi::schedule(move |_| {
            nvim_oxi::api::exec_autocmds([command.as_str()], &opts)
                .map_err(Error::into_internal_error)
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
