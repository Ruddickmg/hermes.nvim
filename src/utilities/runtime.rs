use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;
use tokio::runtime::Runtime;
use tokio::task::LocalSet;
use tracing::debug;

/// Manages async execution on the Neovim main thread.
///
/// Wraps a shared [`Runtime`] and [`LocalSet`] to safely handle re-entrant
/// calls. Re-entrancy occurs when a Neovim autocommand listener (triggered
/// by [`exec_autocmds`] inside a [`block_on`]) calls back into a Hermes API
/// method that itself needs [`block_on`].
///
/// Instead of nesting [`block_on`] (which panics on a `current_thread`
/// runtime), re-entrant calls are spawned onto the shared [`LocalSet`] and
/// driven by the outer [`block_on`].
#[derive(Clone, Debug)]
pub struct NvimRuntime {
    runtime: Rc<Runtime>,
    local_set: Rc<LocalSet>,
    /// Whether we are currently inside a [`NvimRuntime::run`] call.
    running: Rc<Cell<bool>>,
}

impl NvimRuntime {
    pub fn new(runtime: Rc<Runtime>) -> Self {
        Self {
            runtime,
            local_set: Rc::new(LocalSet::new()),
            running: Rc::new(Cell::new(false)),
        }
    }

    /// Run an async future, handling re-entrant calls safely.
    ///
    /// - **Primary call** (no active `block_on`): Drives the future
    ///   synchronously via `local_set.block_on()` and returns `Some(result)`.
    /// - **Re-entrant call** (inside an active `block_on`): Spawns the future
    ///   onto the shared [`LocalSet`] and returns `None`. The outer `block_on`
    ///   drives the spawned task to completion once the synchronous re-entrant
    ///   call returns (e.g., after a Lua autocommand callback finishes).
    ///
    /// Requires `'static` because the re-entrant path uses [`spawn_local`].
    pub fn run<F, R>(&self, future: F) -> Option<R>
    where
        F: Future<Output = R> + 'static,
        R: 'static,
    {
        if self.running.get() {
            debug!("Re-entrant call detected, spawning onto existing LocalSet");
            self.local_set.spawn_local(async move {
                let _ = future.await;
            });
            None
        } else {
            // Use a guard to ensure the flag is reset even if the future panics,
            // so subsequent calls still work after a catch_unwind.
            let _guard = RunningGuard(&self.running);
            self.running.set(true);
            // Run the main future and then drain any tasks that were spawned
            // onto the LocalSet during execution (e.g., from re-entrant calls
            // via spawn_local). We yield after the main future completes to
            // give the executor a chance to poll spawned tasks. Without this,
            // spawned tasks could sit idle until the next block_on call.
            let result = self.local_set.block_on(&self.runtime, async {
                let result = future.await;
                // Yield to allow the LocalSet to poll any tasks that were
                // spawned during execution of the main future (e.g., from
                // re-entrant calls that used spawn_local).
                tokio::task::yield_now().await;
                result
            });
            Some(result)
        }
    }

    /// Run an async future as a primary (non-re-entrant) entry point.
    ///
    /// This method does not require `'static` because it always drives the
    /// future synchronously via `block_on`. Use this for code that is
    /// guaranteed to be the outermost async entry point (e.g., the
    /// [`NvimMessenger`] callback, which is always triggered from libuv's
    /// event loop and never from within another `block_on`).
    ///
    /// # Panics
    ///
    /// Panics if called while another `run` or `block_on_primary` is active
    /// on this `NvimRuntime` (i.e., re-entrant use is not supported).
    pub fn block_on_primary<F, R>(&self, future: F) -> R
    where
        F: Future<Output = R>,
    {
        assert!(
            !self.running.get(),
            "block_on_primary called re-entrantly; use run() for re-entrant calls"
        );
        let _guard = RunningGuard(&self.running);
        self.running.set(true);
        self.local_set.block_on(&self.runtime, future)
    }
}

/// RAII guard that resets the `running` flag when dropped, ensuring
/// it is cleared even if the future panics inside `block_on`.
struct RunningGuard<'a>(&'a Rc<Cell<bool>>);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_runtime() -> Rc<Runtime> {
        Rc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
        )
    }

    #[test]
    fn primary_call_returns_some_with_result() {
        let nvim_rt = NvimRuntime::new(create_runtime());
        let result = nvim_rt.run(async { 42 });
        assert_eq!(result, Some(42));
    }

    #[test]
    fn running_flag_is_false_after_primary_call() {
        let nvim_rt = NvimRuntime::new(create_runtime());
        nvim_rt.run(async {});
        assert!(!nvim_rt.running.get());
    }

    #[test]
    fn reentrant_call_returns_none() {
        let nvim_rt = NvimRuntime::new(create_runtime());
        let inner_rt = nvim_rt.clone();

        let result = nvim_rt.run(async move {
            // Simulate re-entrant call (as if Lua called back into Hermes)
            let inner_result = inner_rt.run(async { 99 });
            inner_result
        });

        // Outer call completes
        assert!(result.is_some());
        // Inner (re-entrant) call returned None
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn spawned_task_runs_to_completion() {
        let nvim_rt = NvimRuntime::new(create_runtime());
        let inner_rt = nvim_rt.clone();
        let flag = Rc::new(Cell::new(false));
        let flag_clone = flag.clone();

        nvim_rt.run(async move {
            // Re-entrant call spawns work onto the LocalSet
            inner_rt.run(async move {
                flag_clone.set(true);
            });
        });

        // The spawned task should have been driven to completion
        // by the outer block_on before it returned
        assert!(flag.get());
    }

    #[test]
    fn running_flag_resets_on_panic() {
        let nvim_rt = NvimRuntime::new(create_runtime());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            nvim_rt.run(async { panic!("test panic") });
        }));
        assert!(result.is_err());
        assert!(!nvim_rt.running.get());
    }
}
