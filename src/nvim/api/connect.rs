use crate::{
    acp::connection::{Assistant, ConnectionDetails, ConnectionManager, Protocol},
    api,
    nvim::autocommands::ResponseHandler,
};
use agent_client_protocol::Client;
use nvim_oxi::{Dictionary, Function, Object, ObjectKind, lua::Error};
use std::rc::Rc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::Api;

impl<H, R> Api<H, R>
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: crate::nvim::requests::RequestHandler + 'static,
{
    #[instrument(level = "trace", skip_all)]
    pub fn connect(
        &self,
        agent_name: nvim_oxi::String,
        options: Option<Dictionary>,
    ) -> Result<(), Error> {
        self.runtime.block_on(async {
            debug!(
                "Connect function called with agent: {:?}, options: {:?}",
                agent_name, options
            );

            let agent_name_str = agent_name.to_string();

            let mut protocol = None;
            let mut command = None;
            let mut args = None;

            if let Some(dict) = options {
                if let Some(obj) = dict.get("protocol") {
                    let s: nvim_oxi::String = obj
                        .clone()
                        .try_into()
                        .map_err(|e| Error::RuntimeError(format!("Invalid protocol: {}", e)))?;
                    protocol = Some(Protocol::from(s.to_string()));
                }

                if let Some(obj) = dict.get("command") {
                    let s: nvim_oxi::String = obj
                        .clone()
                        .try_into()
                        .map_err(|e| Error::RuntimeError(format!("Invalid command: {}", e)))?;
                    command = Some(s.to_string());
                }

                if let Some(obj) = dict.get("args")
                    && obj.kind() == ObjectKind::Array
                {
                    let arr: nvim_oxi::Array = unsafe { obj.clone().into_array_unchecked() };
                    let parsed_args: Vec<String> = arr
                        .into_iter()
                        .filter_map(|v| v.try_into().ok().map(|s: nvim_oxi::String| s.to_string()))
                        .collect();
                    args = Some(parsed_args);
                }
            }
            let agent = if let Some(ref cmd) = command {
                Assistant::Custom {
                    name: agent_name_str,
                    command: cmd.clone(),
                    args: args.clone().unwrap_or_default(),
                }
            } else {
                Assistant::from(agent_name_str)
            };
            let details = ConnectionDetails {
                agent,
                protocol: protocol.unwrap_or_default(),
            };
            let mut connection = self.connection.lock().await;
            connection.connect(details).await.map_err(|e| {
                Error::RuntimeError(format!(
                    "Failed to connect to agent '{}': {}",
                    agent_name, e
                ))
            })?;
            drop(connection);
            Ok(())
        })
    }
}

#[instrument(level = "trace", skip_all)]
pub fn connect<H, R>(api: Rc<Api<H, R>>) -> Object
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: crate::nvim::requests::RequestHandler + 'static,
{
    let function: Function<(nvim_oxi::String, Option<Dictionary>), Result<(), Error>> =
        Function::from_fn(move |(agent_name, options)| api.connect(agent_name, options));
    function.into()
}
