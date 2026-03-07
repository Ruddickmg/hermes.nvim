use crate::acp::Result;
use crate::nvim::autocommands::AutoCommand;
use serde::Serialize;
use std::fmt::Debug;
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
impl ResponseHandler for AutoCommand {
    #[instrument(level = "trace", skip(self))]
    async fn schedule_autocommand<T: Debug + ToString, S: Debug + Serialize>(
        &self,
        command: T,
        data: S,
    ) -> Result<()> {
        self.execute_autocommand(command, data).await
    }
}
