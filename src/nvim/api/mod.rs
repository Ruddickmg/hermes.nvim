pub mod authenticate;
pub mod cancel;
pub mod connect;
pub mod create_session;
pub mod disconnect;
pub mod list_sessions;
pub mod load_session;
pub mod mcp_servers;
pub mod prompt;
pub mod respond;
pub mod set_mode;
pub mod setup;

use std::sync::Arc;

use super::requests::Requests;
use agent_client_protocol::{CancelNotification, ListSessionsRequest};
pub use authenticate::*;
pub use cancel::*;
pub use connect::*;
pub use create_session::*;
pub use disconnect::*;
pub use list_sessions::*;
pub use load_session::*;
use nvim_oxi::{
    Function, Object,
    lua::{Poppable, Pushable},
};
pub use prompt::*;
pub use respond::*;
pub use set_mode::*;
pub use setup::*;
use tracing::trace;

use crate::{
    Handler, PluginState,
    acp::{
        Result,
        connection::{ConnectionDetails, ConnectionManager, Protocol},
        error::Error,
    },
    nvim::requests::RequestHandler,
    utilities::Logger,
};

pub fn create_api_method<A, R, F>(func: F) -> Object
where
    F: Fn(A) -> Result<R> + 'static,
    A: Poppable,
    R: Pushable,
{
    let function: Function<A, Result<()>> = Function::from_fn(move |args: A| -> Result<()> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| func(args)))
            .map(|result| match result {
                Err(e) => eprintln!("ERROR: {}", e),
                Ok(_) => println!("API method executed successfully"),
            })
            .inspect_err(|e| eprintln!("error: {:?}", e))
            .ok();
        Ok(())
    });
    function.into()
}

pub struct Api {
    connection: ConnectionManager,
    logger: Logger,
    state: PluginState,
    response_handler: Arc<Handler>,
    request_handler: Requests,
}

impl Api {
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn new(
        connection: ConnectionManager,
        logger: Logger,
        state: PluginState,
        response_handler: Arc<Handler>,
        request_handler: Requests,
    ) -> Self {
        Self {
            connection,
            logger,
            state,
            response_handler,
            request_handler,
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn cancel(&self, session_id: String) -> Result<()> {
        let connection = self
            .connection
            .get_current_connection()
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        connection.cancel(CancelNotification::new(session_id.clone()))?;

        self.request_handler.cancel_session_requests(session_id)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn list_sessions(&self, maybe_config: Option<ListSessionsConfig>) -> Result<()> {
        let agent_info = self.state.agent_info.clone();

        if !agent_info.can_list_sessions() {
            return Ok(());
        }

        let config = maybe_config.unwrap_or_default();

        let mut request = ListSessionsRequest::new();

        if let Some(cwd) = config.cwd {
            request = request.cwd(cwd);
        }

        if let Some(cursor) = config.cursor {
            request = request.cursor(cursor);
        }

        let connection = self
            .connection
            .get_current_connection()
            .ok_or_else(|| Error::Connection("No connection found".to_string()))?;

        connection.list_sessions(request)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn connect(&self, (agent_name, options): ConnectionArgs) -> Result<()> {
        let mut protocol = Protocol::default();
        if let Some(ref dict) = options
            && let Some(obj) = dict.get("protocol")
        {
            protocol = obj
                .clone()
                .try_into()
                .map(|s: nvim_oxi::String| Protocol::from(s.to_string()))?;
        }
        let agent_name_str = agent_name.to_string();
        let agent = parse_agent_connection(agent_name_str, protocol, options)?;

        self.connection.connect(
            self.response_handler.clone(),
            ConnectionDetails { agent, protocol },
        )?;
        Ok(())
    }
}
