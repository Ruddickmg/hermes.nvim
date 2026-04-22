pub mod mock;
pub mod ui;

pub use mock::*;
pub use ui::*;

use hermes::utilities::NvimRuntime;
use std::rc::Rc;

/// Creates a single-threaded Tokio runtime for testing
pub fn mock_runtime() -> NvimRuntime {
    NvimRuntime::new(Rc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create mock runtime"),
    ))
}
