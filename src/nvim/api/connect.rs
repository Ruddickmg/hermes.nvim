use crate::{apc::connection::{Assistant, ConnectionDetails, ConnectionManager, Protocol}, nvim::autocommands::ResponseHandler};
use agent_client_protocol::Client;
use nvim_oxi::{
    Dictionary, Function, Object,
    lua::{Error, Poppable, Pushable, ffi::State},
};
use std::{rc::Rc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ConnectionArgs {
    pub agent: Option<Assistant>,
    pub protocol: Option<Protocol>,
}

impl From<ConnectionArgs> for ConnectionDetails {
    fn from(args: ConnectionArgs) -> Self {
        ConnectionDetails {
            agent: args.agent.unwrap_or_default(),
            protocol: args.protocol.unwrap_or_default(),
        }
    }
}

impl Poppable for ConnectionArgs {
    unsafe fn pop(state: *mut State) -> Result<Self, Error> {
        use nvim_oxi::{Object, ObjectKind};

        let table = unsafe { Dictionary::pop(state)? };

        let agent = table
            .get("agent")
            .map(|v: &Object| {
                if v.kind() != ObjectKind::String {
                    return Err(Error::RuntimeError(
                        "Invalid input for \"agent\", must be a string".to_string(),
                    ));
                }
                let s: nvim_oxi::NvimStr = unsafe { v.as_nvim_str_unchecked() };
                Ok(Assistant::from(s.to_string()))
            })
            .transpose()?;

        let protocol = table
            .get("protocol")
            .map(|v: &Object| {
                if v.kind() != ObjectKind::String {
                    return Err(Error::RuntimeError(
                        "Invalid input for \"protocol\", must be a string".to_string(),
                    ));
                }
                let s: nvim_oxi::NvimStr = unsafe { v.as_nvim_str_unchecked() };
                Ok(Protocol::from(s.to_string()))
            })
            .transpose()?;

        Ok(Self { agent, protocol })
    }
}

impl Pushable for ConnectionArgs {
    unsafe fn push(self, state: *mut State) -> Result<i32, Error> {
        let dict = nvim_oxi::Object::from({
            let mut dict = Dictionary::new();

            if let Some(agent) = self.agent {
                dict.insert("agent", agent.to_string());
            }

            if let Some(protocol) = self.protocol {
                dict.insert("protocol", protocol.to_string());
            }

            dict
        });

        // SAFETY: Caller must ensure valid state pointer
        unsafe { dict.push(state) }
    }
}

pub fn connect<H: Client + ResponseHandler + Send + Sync + 'static>(
    connection: Rc<Mutex<ConnectionManager<H>>>,
) -> Object {
    let function: Function<Option<ConnectionArgs>, Result<(), Error>> =
        Function::from_fn(move |arg: Option<ConnectionArgs>| -> Result<(), Error> {
            let details = arg.map(ConnectionDetails::from).unwrap_or_default();
            connection
                .blocking_lock()
                .connect(details.clone())?;
            Ok(())
        });
    function.into()
}
