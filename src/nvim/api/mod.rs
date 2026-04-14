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

use std::{cell::RefCell, rc::Rc};
use std::sync::Arc;

use super::requests::Requests;
pub use connect::*;
pub use create_session::*;
pub use disconnect::*;
pub use list_sessions::*;
pub use load_session::*;
use nvim_oxi::{
    Dictionary, Function, Object, lua::{Poppable, Pushable}
};
pub use prompt::*;
pub use respond::*;
pub use set_mode::*;
pub use setup::*;
use tokio::sync::Mutex;
use tracing::{debug, error};

use crate::utilities::Logger;
use crate::{
    Handler, PluginState,
    acp::{Result, connection::ConnectionManager},
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
                Err(e) => error!("An error occurred while executing api method: {:?}", e),
                Ok(_) => debug!("API method executed successfully"),
            })
            .inspect_err(|e| error!("A panic occurred while executing api method: {:?}", e))
            .ok();
        Ok(())
    });
    function.into()
}

pub struct Api {
    state: Arc<Mutex<PluginState>>,
    logger: &'static Logger,
    connection: ConnectionManager,
    response_handler: Arc<Handler>,
    request_handler: Rc<Requests>,
}

impl Api {
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn new(
        state: Arc<Mutex<PluginState>>,
        logger: &'static Logger,
        response_handler: Arc<Handler>,
        request_handler: Rc<Requests>,
    ) -> Self {
        Self {
            connection: ConnectionManager::new(state.clone()),
            response_handler,
            request_handler,
            logger,
            state,
        }
    }

    fn create_cancel_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |session_id: String| api.try_borrow()?.cancel(session_id))
    }

    fn create_connect_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |args: ConnectionArgs| api.try_borrow_mut()?.connect(args))
    }

    fn create_create_session_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |args: CreateSessionArgs| api.try_borrow()?.create_session(args))
    }

    fn create_disconnect_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |args: DisconnectArgs| api.try_borrow_mut()?.disconnect(args))
    }

    fn create_list_sessions_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |args: Option<ListSessionsConfig>| api.try_borrow()?.list_sessions(args))
    }

    fn create_load_session_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |args: LoadSessionArgs| api.try_borrow()?.load_session(args))
    }

    fn create_authenticate_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |id: String| api.try_borrow()?.authenticate(id))
    }
    
    fn create_set_mode_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |args: SetModeArgs| api.try_borrow()?.set_mode(args))
    }

    fn ceate_setup_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |args: SetupArgs| api.try_borrow()?.setup(args))
    }

    fn create_prompt_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |args: PromptArgs| api.try_borrow()?.prompt(args))
    }

    fn create_respond_method(api: Rc<RefCell<Self>>) -> Object {
        create_api_method(move |args: RespondArgs| api.try_borrow()?.respond(args))
    }
}

impl Into<Dictionary> for Api {
    fn into(self) -> Dictionary {
        let api = Rc::new(RefCell::new(self));
        Dictionary::from_iter([
            ("cancel", Self::create_cancel_method(api.clone())),
            ("connect", Self::create_connect_method(api.clone())),
            ("authenticate", Self::create_authenticate_method(api.clone())),
            ("disconnect", Self::create_disconnect_method(api.clone())),
            ("create_session", Self::create_create_session_method(api.clone())),
            ("load_session", Self::create_load_session_method(api.clone())),
            ("list_sessions", Self::create_list_sessions_method(api.clone())),
            ("prompt", Self::create_prompt_method(api.clone())),
            ("set_mode", Self::create_set_mode_method(api.clone())),
            ("respond", Self::create_respond_method(api.clone())),
            ("setup", Self::ceate_setup_method(api.clone())),
        ])
    }
}
