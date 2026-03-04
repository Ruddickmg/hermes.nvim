use agent_client_protocol::InitializeResponse;
use nvim_oxi::{Function, lua::Error};

#[derive(Clone, Debug)]
pub struct Callbacks {
    initialized: Option<Function<InitializeResponse, Result<(), Error>>>,
}

impl Default for Callbacks {
    fn default() -> Self {
        Self { initialized: None }
    }
}
