pub mod agents;
pub mod authenticate;
pub mod cancel;
pub mod close_session;
pub mod config_options;
pub mod configure_model;
pub mod connect;
pub mod create_session;
pub mod delete_session;
pub mod disconnect;
pub mod list_sessions;
pub mod load_session;
pub mod logout;
pub mod mcp_servers;
pub mod model_configurations;
pub mod models;
pub mod modes;
pub mod prompt;
pub mod respond;
pub mod resume_session;
pub mod set_config_option;
pub mod set_mode;
pub mod set_model;
pub mod set_thought_level;
pub mod setup;
pub mod thought_levels;

use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};

use super::requests::Requests;
pub use agents::*;
use async_lock::Mutex;
pub use config_options::*;
pub use configure_model::*;
pub use connect::*;
pub use create_session::*;
pub use delete_session::*;
pub use disconnect::*;
pub use list_sessions::*;
pub use load_session::*;
pub use logout::*;
pub use model_configurations::*;
pub use models::*;
pub use modes::*;
use nvim_oxi::{
    Dictionary, Function, Object,
    lua::{Poppable, Pushable},
};
pub use prompt::*;
pub use respond::*;
pub use resume_session::*;
pub use set_config_option::*;
pub use set_mode::*;
pub use set_model::*;
pub use set_thought_level::*;
pub use setup::*;
pub use thought_levels::*;
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
        let function: Function<A, Option<R>> = Function::from_fn(move |args: A| -> Option<R> {
            let api = api.clone();
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                nvim_runtime.run(func(api, args))
            }))
            .map(|result| match result {
                Some(Err(e)) => {
                    error!("An error occurred while executing api method: {:?}", e);
                    None
                }
                Some(Ok(value)) => {
                    debug!("API method executed successfully");
                    Some(value)
                }
                None => {
                    debug!("API method scheduled (re-entrant call)");
                    None
                }
            })
            .inspect_err(|e| error!("A panic occurred while executing api method: {:?}", e))
            .unwrap_or(None)
        });
        function.into()
    }

    fn cancel_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, session_id: String| async move {
            api.try_borrow()?.cancel(session_id).await
        })
    }

    fn close_session_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, session_id: String| async move {
            api.try_borrow()?.close_session(session_id).await
        })
    }

    fn delete_session_method(&self) -> Object {
        self.api_method(
            |api: Rc<RefCell<Api>>, args: DeleteSessionArgs| async move {
                api.try_borrow()?.delete_session(args).await
            },
        )
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

    fn resume_session_method(&self) -> Object {
        self.api_method(
            |api: Rc<RefCell<Api>>, args: ResumeSessionArgs| async move {
                api.try_borrow()?.resume_session(args).await
            },
        )
    }

    fn logout_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: LogoutArgs| async move {
            api.try_borrow()?.logout(args).await
        })
    }

    fn agents_method(&self) -> Object {
        self.api_method(
            |api: Rc<RefCell<Api>>, args: Option<AgentsConfig>| async move {
                api.try_borrow()?.agents(args).await
            },
        )
    }

    fn authenticate_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, id: String| async move {
            api.try_borrow()?.authenticate(id).await
        })
    }

    fn config_options_method(&self) -> Object {
        self.api_method(
            |api: Rc<RefCell<Api>>, session_id: ConfigOptionsArgs| async move {
                api.try_borrow()?.config_options(session_id).await
            },
        )
    }

    fn set_config_option_method(&self) -> Object {
        self.api_method(
            |api: Rc<RefCell<Api>>, args: SetConfigOptionArgs| async move {
                api.try_borrow()?.set_config_option(args).await
            },
        )
    }

    fn modes_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, session_id: String| async move {
            api.try_borrow()?.modes(session_id).await
        })
    }

    fn models_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, session_id: String| async move {
            api.try_borrow()?.models(session_id).await
        })
    }

    fn model_configurations_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, session_id: String| async move {
            api.try_borrow()?.model_configurations(session_id).await
        })
    }

    fn configure_model_method(&self) -> Object {
        self.api_method(
            |api: Rc<RefCell<Api>>, args: ConfigureModelArgs| async move {
                api.try_borrow()?.configure_model(args).await
            },
        )
    }

    fn set_mode_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: SetModeArgs| async move {
            api.try_borrow()?.set_mode(args).await
        })
    }

    fn set_model_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, args: SetModelArgs| async move {
            api.try_borrow()?.set_model(args).await
        })
    }

    fn thought_levels_method(&self) -> Object {
        self.api_method(|api: Rc<RefCell<Api>>, session_id: String| async move {
            api.try_borrow()?.thought_levels(session_id).await
        })
    }

    fn set_thought_level_method(&self) -> Object {
        self.api_method(
            |api: Rc<RefCell<Api>>, args: SetThoughtLevelArgs| async move {
                api.try_borrow()?.set_thought_level(args).await
            },
        )
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
            ("agents", hermes.agents_method()),
            ("cancel", hermes.cancel_method()),
            ("close_session", hermes.close_session_method()),
            ("delete_session", hermes.delete_session_method()),
            ("connect", hermes.connect_method()),
            ("create_session", hermes.create_session_method()),
            ("disconnect", hermes.disconnect_method()),
            ("list_sessions", hermes.list_sessions_method()),
            ("load_session", hermes.load_session_method()),
            ("logout", hermes.logout_method()),
            ("resume_session", hermes.resume_session_method()),
            ("config_options", hermes.config_options_method()),
            ("set_config_option", hermes.set_config_option_method()),
            ("modes", hermes.modes_method()),
            ("models", hermes.models_method()),
            ("model_configurations", hermes.model_configurations_method()),
            ("configure_model", hermes.configure_model_method()),
            ("authenticate", hermes.authenticate_method()),
            ("set_mode", hermes.set_mode_method()),
            ("set_model", hermes.set_model_method()),
            ("thought_levels", hermes.thought_levels_method()),
            ("set_thought_level", hermes.set_thought_level_method()),
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
