use crate::{
    acp::connection::{Assistant, ConnectionDetails, ConnectionManager, Protocol},
    nvim::autocommands::ResponseHandler,
};
use agent_client_protocol::Client;
use nvim_oxi::{lua::Error, Dictionary, Function, Object, ObjectKind};
use std::rc::Rc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

#[instrument(level = "trace", skip_all)]
pub fn connect<H: Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
    let function: Function<(nvim_oxi::String, Option<Dictionary>), Result<(), Error>> =
        Function::from_fn(move |(agent_name, options): (nvim_oxi::String, Option<Dictionary>)| -> Result<(), Error> {
            debug!("Connect function called with agent: {:?}, options: {:?}", agent_name, options);
            
            let agent_name_str = agent_name.to_string();
            
            // Parse options
            let mut protocol = None;
            let mut command = None;
            let mut args = None;
            
            if let Some(dict) = options {
                // Parse protocol
                if let Some(obj) = dict.get("protocol") {
                    let s: nvim_oxi::String = obj.clone().try_into()
                        .map_err(|e| Error::RuntimeError(format!("Invalid protocol: {}", e)))?;
                    protocol = Some(Protocol::from(s.to_string()));
                }
                
                // Parse command
                if let Some(obj) = dict.get("command") {
                    let s: nvim_oxi::String = obj.clone().try_into()
                        .map_err(|e| Error::RuntimeError(format!("Invalid command: {}", e)))?;
                    command = Some(s.to_string());
                }
                
                // Parse args
                if let Some(obj) = dict.get("args") {
                    if obj.kind() == ObjectKind::Array {
                        let arr: nvim_oxi::Array = unsafe { obj.clone().into_array_unchecked() };
                        let parsed_args: Vec<String> = arr
                            .into_iter()
                            .filter_map(|v| v.try_into().ok().map(|s: nvim_oxi::String| s.to_string()))
                            .collect();
                        args = Some(parsed_args);
                    }
                }
            }
            
            // Create agent
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
            
            connection.blocking_lock().connect(details)?;
            Ok(())
        });
    function.into()
}
