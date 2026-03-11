use crate::acp::Result;
use crate::acp::connection::ConnectionManager;
use crate::nvim::autocommands::AutoCommand;
use crate::nvim::requests::RequestHandler;
use agent_client_protocol::Client;
use nvim_oxi::Object;
use serde::Serialize;
use tokio::sync::Mutex;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::Arc;
use tracing::instrument;

#[async_trait::async_trait(?Send)]
pub trait ResponseHandler {
    async fn schedule_autocommand<T: Debug + ToString, S: Debug + Serialize>(
        &self,
        command: T,
        data: S,
    ) -> Result<()>;
}

#[async_trait::async_trait(?Send)]
impl<R: RequestHandler> ResponseHandler for AutoCommand<R> {
    #[instrument(level = "trace", skip(self))]
    async fn schedule_autocommand<T: Debug + ToString, S: Debug + Serialize>(
        &self,
        command: T,
        data: S,
    ) -> Result<()> {
        self.execute_autocommand(command, data).await
    }
}

