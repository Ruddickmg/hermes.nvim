pub mod authenticate;
pub mod cancel;
pub mod connect;
pub mod create_session;
pub mod disconnect;
pub mod prompt;
pub mod respond;
pub mod set_mode;

pub use authenticate::*;
pub use cancel::*;
pub use connect::*;
pub use create_session::*;
pub use disconnect::*;
pub use prompt::*;
pub use respond::*;
pub use set_mode::*;

use agent_client_protocol::Client;
use std::{rc::Rc, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    acp::connection::ConnectionManager,
    nvim::{autocommands::ResponseHandler, requests::RequestHandler},
};

pub struct Api<H, R>
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: RequestHandler + 'static,
{
    connection: Rc<Mutex<ConnectionManager<H>>>,
    request_handler: Arc<R>,
    runtime: tokio::runtime::Runtime,
}

impl<H, R> Api<H, R>
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: RequestHandler + 'static,
{
    pub fn new(
        connection: Rc<Mutex<ConnectionManager<H>>>,
        request_handler: Arc<R>,
        runtime: tokio::runtime::Runtime,
    ) -> Self {
        Self {
            connection,
            request_handler,
            runtime,
        }
    }
}
