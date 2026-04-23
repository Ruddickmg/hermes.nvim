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
use std::{cell::RefCell, rc::Rc};

use super::requests::Requests;
pub use connect::*;
pub use create_session::*;
pub use disconnect::*;
pub use list_sessions::*;
pub use load_session::*;
use nvim_oxi::{
    Dictionary, Function, Object,
    lua::{Poppable, Pushable},
};
pub use prompt::*;
pub use respond::*;
pub use set_mode::*;
pub use setup::*;
use async_lock::Mutex;
use tracing::{debug, error};

use crate::utilities::{Logger, NvimRuntime};
use crate::{
    Handler, PluginState,
    acp::{Result, connection::ConnectionManager},
};

pub struct Hermes {
    api: Rc<RefCell<Api>>,
    nvim_runtime: NvimRuntime,
}

impl Hermes {
    pub fn new(nvim_runtime: NvimRuntime, api: Rc<RefCell<Api>>) -> Result<Self> {
        Ok(Self { api, nvim_runtime })
    }

    fn api_method<A, R, F, Fut>(&self, func: F) -> Object
    where
        F: Fn(Rc<RefCell<Api>>, A) -> Fut + 'static,
        Fut: Future<Output = Result<R>> + 'static,
        A: Poppable,
        R: Pushable + 'static,
    {
        let nvim_runtime = self.nvim_runtime.clone();
        let api = self.api.clone();
        let function: Function<A, Result<()>> = Function::from_fn(move |args: A| -> Result<()> {
            let api = api.clone();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                nvim_runtime.run(func(api, args))
            }))
            .map(|result| match result {
                Some(Err(e)) => error!("An error occurred while executing api method: {:?}", e),
                Some(Ok(_)) => debug!("API method executed successfully"),
                None => debug!("API method scheduled (re-entrant call)"),
            })
            .inspect_err(|e| error!("A panic occurred while executing api method: {:?}", e))
            .ok();
            Ok(())
        });
        function.into()
    }

    fn cancel_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, session_id: String| async move {
            api.try_borrow()?.cancel(session_id).await
        })
    }

    fn connect_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: ConnectionArgs| async move {
            api.try_borrow_mut()?.connect(args).await
        })
    }

    fn create_session_method(&self) -> Object {
        self.api_method(
            |api: Rc<RefCell<Api>>, args: CreateSessionArgs| async move {
                api.try_borrow()?.create_session(args).await
            },
        )
    }

    fn disconnect_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: DisconnectArgs| async move {
            api.try_borrow_mut()?.disconnect(args).await
        })
    }

    fn list_sessions_method(&self) -> Object {
        self.api_method(
            |api: Rc<RefCell<Api>>, args: Option<ListSessionsConfig>| async move {
                api.try_borrow()?.list_sessions(args).await
            },
        )
    }

    fn load_session_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: LoadSessionArgs| async move {
            api.try_borrow()?.load_session(args).await
        })
    }

    fn authenticate_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, id: String| async move {
            api.try_borrow()?.authenticate(id).await
        })
    }

    fn set_mode_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: SetModeArgs| async move {
            api.try_borrow()?.set_mode(args).await
        })
    }

    fn setup_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: SetupArgs| async move {
            api.try_borrow()?.setup(args).await
        })
    }

    fn prompt_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: PromptArgs| async move {
            api.try_borrow()?.prompt(args).await
        })
    }

    fn respond_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: RespondArgs| async move {
            api.try_borrow()?.respond(args).await
        })
    }
}

impl From<Hermes> for Dictionary {
    fn from(hermes: Hermes) -> Dictionary {
        Dictionary::from_iter([
            ("cancel", hermes.cancel_method()),
            ("connect", hermes.connect_method()),
            ("create_session", hermes.create_session_method()),
            ("disconnect", hermes.disconnect_method()),
            ("list_sessions", hermes.list_sessions_method()),
            ("load_session", hermes.load_session_method()),
            ("authenticate", hermes.authenticate_method()),
            ("set_mode", hermes.set_mode_method()),
            ("setup", hermes.setup_method()),
            ("prompt", hermes.prompt_method()),
            ("respond", hermes.respond_method()),
        ])
    }
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
}
