use std::rc::Rc;

use agent_client_protocol::Client;
use nvim_oxi::{Function, Object, conversion::FromObject, lua::Error};
use tracing::{debug, instrument};
use crate::{acp::connection::Assistant, nvim::autocommands::ResponseHandler};

use super::Api;

#[derive(Clone, Debug, Default)]
pub enum DisconnectArgs {
    Multiple(Vec<Assistant>),
    Single(Assistant),
    #[default]
    All,
}

fn parse_assistant_string(
    assistant: nvim_oxi::String,
) -> Result<Assistant, nvim_oxi::conversion::Error> {
    match assistant.to_string().to_lowercase().as_str() {
        "copilot" => Ok(Assistant::Copilot),
        "opencode" => Ok(Assistant::Opencode),
        other => Err(nvim_oxi::conversion::Error::Serialize(
            nvim_oxi::serde::SerializeError {
                msg: format!(
                    "Invalid input found: {}, Agent name must be one of 'copilot' or 'opencode'",
                    other
                ),
            },
        )),
    }
}

const EXPECTED: &str = "Nil, String or Array of Strings";

impl nvim_oxi::conversion::FromObject for DisconnectArgs {
    fn from_object(obj: Object) -> Result<Self, nvim_oxi::conversion::Error> {
        match obj.kind() {
            nvim_oxi::ObjectKind::Nil => Ok(Self::All),
            nvim_oxi::ObjectKind::String => {
                let kind = obj.kind();
                let assistant = unsafe { obj.into_string_unchecked() };
                parse_assistant_string(assistant)
                    .map_err(|_| nvim_oxi::conversion::Error::FromWrongType {
                        expected: EXPECTED,
                        actual: kind.as_static(),
                    })
                    .map(Self::Single)
            }
            nvim_oxi::ObjectKind::Array => {
                let assistants = unsafe { obj.into_array_unchecked() };
                assistants
                    .into_iter()
                    .map(|obj| {
                        if let nvim_oxi::ObjectKind::String = obj.kind() {
                            Ok(unsafe { obj.into_string_unchecked() })
                        } else {
                            Err(nvim_oxi::conversion::Error::FromWrongType {
                                expected: EXPECTED,
                                actual: obj.kind().as_static(),
                            })
                        }
                    })
                    .collect::<Result<Vec<nvim_oxi::String>, nvim_oxi::conversion::Error>>()?
                    .into_iter()
                    .map(parse_assistant_string)
                    .collect::<Result<Vec<Assistant>, nvim_oxi::conversion::Error>>()
                    .map(Self::Multiple)
            }
            other => Err(nvim_oxi::conversion::Error::FromWrongType {
                expected: EXPECTED,
                actual: other.as_static(),
            }),
        }
    }
}

impl nvim_oxi::lua::Poppable for DisconnectArgs {
    unsafe fn pop(lua: *mut nvim_oxi::lua::ffi::State) -> Result<Self, nvim_oxi::lua::Error> {
        let obj = unsafe { Object::pop(lua)? };
        Self::from_object(obj).map_err(|e| match e {
            nvim_oxi::conversion::Error::FromWrongType { actual, .. } => nvim_oxi::lua::Error::PopError {
                ty: actual,
                message: Some(format!("Invalid argument passed to \"disconnect\": {}. Expected Nil, String or Array of Strings", actual)),
            },
            _ => nvim_oxi::lua::Error::RuntimeError(e.to_string()),
        })
    }
}

impl nvim_oxi::lua::Pushable for DisconnectArgs {
    unsafe fn push(self, state: *mut nvim_oxi::lua::ffi::State) -> Result<i32, Error> {
        unsafe {
            match self {
                Self::All => ().push(state),
                Self::Single(s) => s.to_string().push(state),
                Self::Multiple(vec) => vec
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .push(state),
            }
        }
    }
}

impl<H, R> Api<H, R>
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: crate::nvim::requests::RequestHandler + 'static,
{
    #[instrument(level = "trace", skip_all)]
    pub fn disconnect(&self, args: DisconnectArgs) -> Result<(), Error> {
        debug!("Disconnect function called with {:#?}", args);
        self.runtime.block_on(async {
            let mut manager = self.connection.lock().await;
            match args {
                DisconnectArgs::Multiple(agents) => manager.disconnect(agents).await,
                DisconnectArgs::Single(agent) => manager.disconnect(vec![agent.clone()]).await,
                DisconnectArgs::All => manager.close_all().await,
            }?;
            drop(manager);
            Ok(())
        })
    }
}

#[instrument(level = "trace", skip_all)]
pub fn disconnect<H, R>(api: Rc<Api<H, R>>) -> Object
where
    H: Client + ResponseHandler + Send + Sync + 'static,
    R: crate::nvim::requests::RequestHandler + 'static,
{
    let function: Function<DisconnectArgs, Result<(), Error>> =
        Function::from_fn(move |args| api.disconnect(args));
    function.into()
}
