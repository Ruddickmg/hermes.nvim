use agent_client_protocol::Client;
use nvim_oxi::{
    Function, Object, ObjectKind,
    conversion::{self, FromObject},
    lua::{self, Error, Poppable, Pushable},
    serde::SerializeError,
};
use std::rc::Rc;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use crate::{
    acp::connection::{Assistant, ConnectionManager},
    nvim::autocommands::ResponseHandler,
};

#[derive(Clone, Debug, Default)]
pub enum DisconnectArgs {
    Multiple(Vec<Assistant>),
    Single(Assistant),
    #[default]
    All,
}

#[instrument(level = "trace", skip_all)]
fn parse_assistant_string(
    assistant: nvim_oxi::String,
) -> Result<Assistant, nvim_oxi::conversion::Error> {
    match assistant.to_string().to_lowercase().as_str() {
        "copilot" => Ok(Assistant::Copilot),
        "opencode" => Ok(Assistant::Opencode),
        other => Err(nvim_oxi::conversion::Error::Serialize(SerializeError {
            msg: format!(
                "Invalid input found: {}, Agent name must be one of 'copilot' or 'opencode'",
                other
            ),
        })),
    }
}

const EXPECTED: &str = "Nil, String or Array of Strings";

impl FromObject for DisconnectArgs {
    fn from_object(obj: Object) -> Result<Self, nvim_oxi::conversion::Error> {
        match obj.kind() {
            ObjectKind::Nil => Ok(Self::All),
            ObjectKind::String => {
                let kind = obj.kind();
                let assistant = unsafe { obj.into_string_unchecked() };
                parse_assistant_string(assistant)
                    .map_err(|_| nvim_oxi::conversion::Error::FromWrongType {
                        expected: EXPECTED,
                        actual: kind.as_static(),
                    })
                    .map(Self::Single)
            }
            ObjectKind::Array => {
                let assistants = unsafe { obj.into_array_unchecked() };
                assistants
                    .into_iter()
                    .map(|obj| {
                        if let ObjectKind::String = obj.kind() {
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

impl Poppable for DisconnectArgs {
    unsafe fn pop(lua: *mut nvim_oxi::lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(lua)? };
        Self::from_object(obj).map_err(|e| match e {
            conversion::Error::FromWrongType { actual, .. } => lua::Error::PopError {
                ty: actual,
                message: Some(format!("Invalid argument passed to \"disconnect\": {}. Expected Nil, String or Array of Strings", actual)),
            },
            _ => lua::Error::RuntimeError(e.to_string()),
        })
    }
}

impl Pushable for DisconnectArgs {
    unsafe fn push(self, state: *mut lua::ffi::State) -> Result<i32, Error> {
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

#[instrument(level = "trace", skip_all)]
pub fn disconnect<H: Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
    let function: Function<DisconnectArgs, Result<(), Error>> =
        Function::from_fn(move |args: DisconnectArgs| -> Result<(), Error> {
            debug!("Disconnect function called with {:#?}", args);
            let mut manager = connection.blocking_lock();
            match args {
                DisconnectArgs::Multiple(agents) => manager.disconnect(agents),
                DisconnectArgs::Single(agent) => manager.disconnect(vec![agent.clone()]),
                DisconnectArgs::All => manager.close_all(),
            }?;
            drop(manager);
            Ok(())
        });
    function.into()
}
