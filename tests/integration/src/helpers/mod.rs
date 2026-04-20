pub mod mock;
pub mod ui;

pub use mock::*;
pub use ui::*;

use std::rc::Rc;
use tokio::runtime::Runtime;

/// Creates a single-threaded Tokio runtime for testing
pub fn mock_runtime() -> Rc<Runtime> {
    Rc::new(
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("Failed to create mock runtime"),
    )
}
