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
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tracing::{debug, error};

use crate::utilities::Logger;
use crate::{
    Handler, PluginState,
    acp::{Result, connection::ConnectionManager},
};

pub struct HermesRuntime {
    api: Rc<RefCell<Api>>,
    runtime: Rc<Runtime>,
}

impl HermesRuntime {
    pub fn new(runtime: Rc<Runtime>, api: Rc<RefCell<Api>>) -> Result<Self> {
        Ok(Self { api, runtime })
    }

    fn create_api_method<A, R, F, Fut>(&self, func: F) -> Object
    where
        F: Fn(A) -> Fut + 'static,
        Fut: Future<Output = Result<R>>,
        A: Poppable,
        R: Pushable,
    {
        let runtime = self.runtime.clone();
        let function: Function<A, Result<()>> = Function::from_fn(move |args: A| -> Result<()> {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(tokio::task::LocalSet::new().run_until(func(args)))
            }))
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

    fn create_cancel_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |session_id: String| {
            let methods = api.clone();
            async move { methods.try_borrow()?.cancel(session_id).await }
        })
    }

    fn create_connect_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |args: ConnectionArgs| {
            let methods = api.clone();
            async move { methods.try_borrow_mut()?.connect(args).await }
        })
    }

    fn create_create_session_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |args: CreateSessionArgs| {
            let methods = api.clone();
            async move { methods.try_borrow()?.create_session(args).await }
        })
    }

    fn create_disconnect_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |args: DisconnectArgs| {
            let methods = api.clone();
            async move { methods.try_borrow_mut()?.disconnect(args).await }
        })
    }

    fn create_list_sessions_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |args: Option<ListSessionsConfig>| {
            let methods = api.clone();
            async move { methods.try_borrow()?.list_sessions(args).await }
        })
    }

    fn create_load_session_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |args: LoadSessionArgs| {
            let methods = api.clone();
            async move { methods.try_borrow()?.load_session(args).await }
        })
    }

    fn create_authenticate_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |id: String| {
            let methods = api.clone();
            async move { methods.try_borrow()?.authenticate(id).await }
        })
    }

    fn create_set_mode_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |args: SetModeArgs| {
            let methods = api.clone();
            async move { methods.try_borrow()?.set_mode(args).await }
        })
    }

    fn create_setup_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |args: SetupArgs| {
            let methods = api.clone();
            async move { methods.try_borrow()?.setup(args).await }
        })
    }

    fn create_prompt_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |args: PromptArgs| {
            let methods = api.clone();
            async move { methods.try_borrow()?.prompt(args).await }
        })
    }

    fn create_respond_method(&self, api: Rc<RefCell<Api>>) -> Object {
        self.create_api_method(move |args: RespondArgs| {
            let methods = api.clone();
            async move { methods.try_borrow()?.respond(args).await }
        })
    }
}

impl From<HermesRuntime> for Dictionary {
    fn from(runtime: HermesRuntime) -> Dictionary {
        let api = runtime.api.clone();
        Dictionary::from_iter([
            ("cancel", runtime.create_cancel_method(api.clone())),
            ("connect", runtime.create_connect_method(api.clone())),
            (
                "create_session",
                runtime.create_create_session_method(api.clone()),
            ),
            ("disconnect", runtime.create_disconnect_method(api.clone())),
            (
                "list_sessions",
                runtime.create_list_sessions_method(api.clone()),
            ),
            (
                "load_session",
                runtime.create_load_session_method(api.clone()),
            ),
            (
                "authenticate",
                runtime.create_authenticate_method(api.clone()),
            ),
            ("set_mode", runtime.create_set_mode_method(api.clone())),
            ("setup", runtime.create_setup_method(api.clone())),
            ("prompt", runtime.create_prompt_method(api.clone())),
            ("respond", runtime.create_respond_method(api.clone())),
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
