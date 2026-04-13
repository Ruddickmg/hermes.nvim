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

pub use authenticate::*;
pub use cancel::*;
pub use connect::*;
pub use create_session::*;
pub use disconnect::*;
pub use list_sessions::*;
pub use load_session::*;
use nvim_oxi::{
    Function, Object,
    lua::{Poppable, Pushable},
};
pub use prompt::*;
pub use respond::*;
pub use set_mode::*;
pub use setup::*;

use crate::acp::Result;

pub fn create_api_method<A, R, F>(func: F) -> Object
where
    F: Fn(A) -> Result<R> + 'static,
    A: Poppable,
    R: Pushable,
{
    let function: Function<A, Result<()>> = Function::from_fn(move |args: A| -> Result<()> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| func(args)))
            .map(|result| match result {
                Err(e) => eprintln!("ERROR: {}", e),
                Ok(_) => println!("API method executed successfully"),
            })
            .inspect_err(|e| eprintln!("error: {:?}", e))
            .ok();
        Ok(())
    });
    function.into()
}
