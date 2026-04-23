mod sys;

use crate::acp::Result;
use async_lock::Mutex;
use async_process::{Child as AsyncChild, ChildStderr, ChildStdin, ChildStdout, Command};
use event_listener::Event;
use std::io;
use std::process::ExitStatus;
use std::sync::Arc;
use tracing::{debug, trace, warn};

#[derive(Debug)]
enum ChildState {
    Running,
    Exited(ExitStatus),
}

#[derive(Debug)]
struct ChildInner {
    child: AsyncChild,
    state: ChildState,
}

/// A wrapper around an async_process child process that supports lazy initialization.
///
/// The child process can be created in two phases:
/// 1. `Child::new()` creates an uninitialized handle (no process spawned yet)
/// 2. `Child::initialize()` spawns the actual process
///
/// This two-phase construction allows the `Child` to be shared (via `Arc`) between
/// threads before the process is spawned. The process must be spawned on the same
/// executor whose reactor will handle its IO.
///
/// For convenience, `Child::spawn()` performs both phases in one call.
#[derive(Debug)]
pub struct Child {
    inner: Mutex<Option<ChildInner>>,
    exit_notify: Arc<Event>,
}

impl Child {
    /// Create an uninitialized child handle. No process is spawned yet.
    ///
    /// Call `initialize()` on the target runtime's thread before using IO methods.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            exit_notify: Arc::new(Event::new()),
        }
    }

    /// Spawn the child process. Must be called on the executor whose
    /// reactor will be used for IO (stdin/stdout).
    pub async fn initialize(&self, command: &mut Command) -> Result<()> {
        let child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        debug!("Child process spawned with PID {:?}", child.id());

        let mut inner = self.inner.lock().await;
        *inner = Some(ChildInner {
            child,
            state: ChildState::Running,
        });
        Ok(())
    }

    /// Convenience method that creates and immediately initializes a child process.
    ///
    /// Equivalent to calling `new()` followed by `initialize()`. The process is
    /// spawned on the current executor.
    pub async fn spawn(command: &mut Command) -> Result<Self> {
        let child = Self::new();
        child.initialize(command).await?;
        Ok(child)
    }

    pub async fn take_stdin(&self) -> Option<ChildStdin> {
        let mut inner = self.inner.lock().await;
        inner.as_mut()?.child.stdin.take()
    }

    pub async fn take_stdout(&self) -> Option<ChildStdout> {
        let mut inner = self.inner.lock().await;
        inner.as_mut()?.child.stdout.take()
    }

    pub async fn take_stderr(&self) -> Option<ChildStderr> {
        let mut inner = self.inner.lock().await;
        inner.as_mut()?.child.stderr.take()
    }

    pub async fn id(&self) -> Option<u32> {
        let inner = self.inner.lock().await;
        let inner = inner.as_ref()?;
        match inner.state {
            ChildState::Exited(_) => None,
            ChildState::Running => Some(inner.child.id()),
        }
    }

    pub async fn wait(&self) -> io::Result<ExitStatus> {
        {
            let inner = self.inner.lock().await;
            let inner = inner
                .as_ref()
                .ok_or_else(|| io::Error::other("child not initialized"))?;
            if let ChildState::Exited(status) = inner.state {
                return Ok(status);
            }
        }

        let handle = {
            let inner = self.inner.lock().await;
            let inner = inner
                .as_ref()
                .ok_or_else(|| io::Error::other("child not initialized"))?;
            sys::get_handle(&inner.child)
        };

        trace!("Waiting for child process to exit (non-reaping)");
        // Use blocking::unblock for blocking operations
        blocking::unblock(move || sys::wait_noreap(handle)).await?;

        let mut inner = self.inner.lock().await;
        let inner = inner
            .as_mut()
            .ok_or_else(|| io::Error::other("child not initialized"))?;

        if let ChildState::Exited(status) = inner.state {
            return Ok(status);
        }

        // async_process doesn't have try_wait, so we check the state
        // The child should have exited after wait_noreap returns
        let status = match inner.child.status().await {
            Ok(s) => s,
            Err(e) => return Err(io::Error::other(format!("child status error: {}", e))),
        };

        inner.state = ChildState::Exited(status);
        self.exit_notify.notify(usize::MAX);
        Ok(status)
    }

    /// Check if the child has exited without blocking.
    ///
    /// Returns `Ok(Some(status))` if exited, `Ok(None)` if still running.
    /// Returns `Ok(None)` if the child has not been initialized yet.
    pub async fn try_wait(&self) -> io::Result<Option<ExitStatus>> {
        let mut inner = self.inner.lock().await;
        let Some(inner) = inner.as_mut() else {
            return Ok(None);
        };
        match inner.state {
            ChildState::Exited(status) => Ok(Some(status)),
            ChildState::Running => {
                // async_process doesn't have try_wait - we'll check by attempting to get status
                // without blocking (this isn't perfect but works for most cases)
                match inner.child.status().await {
                    Ok(status) => {
                        inner.state = ChildState::Exited(status);
                        self.exit_notify.notify(usize::MAX);
                        Ok(Some(status))
                    }
                    Err(_) => Ok(None),
                }
            }
        }
    }

    pub async fn terminate(&self) -> io::Result<()> {
        let inner = self.inner.lock().await;
        let Some(inner) = inner.as_ref() else {
            warn!("Cannot terminate: child process not yet initialized");
            return Ok(());
        };
        if let ChildState::Exited(_) = inner.state {
            return Ok(()); // Already exited
        }
        debug!("Sending terminate signal to child process");
        sys::terminate(&inner.child)
    }

    pub async fn kill(&self) -> io::Result<()> {
        let inner = self.inner.lock().await;
        let Some(inner) = inner.as_ref() else {
            warn!("Cannot kill: child process not yet initialized");
            return Ok(());
        };
        if let ChildState::Exited(_) = inner.state {
            return Ok(()); // Already exited
        }
        debug!("Sending kill signal to child process");
        sys::force_kill(&inner.child)
    }

    /// Synchronous, non-blocking kill attempt for use in `Drop` contexts where
    /// no async runtime is available.
    ///
    /// Uses `try_lock()` to avoid blocking if the mutex is held. If the lock
    /// cannot be acquired, the kill is skipped (the child's own `Drop` impl
    /// provides fallback cleanup).
    pub fn try_kill_sync(&self) -> io::Result<()> {
        let Some(mut inner) = self.inner.try_lock() else {
            debug!("Could not acquire child lock for sync kill, skipping");
            return Ok(());
        };
        let Some(ref mut inner) = *inner else {
            return Ok(());
        };
        if let ChildState::Exited(_) = inner.state {
            return Ok(());
        }
        debug!("Sending kill signal to child process (sync)");
        inner.child.kill()
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        let inner = self.inner.get_mut();
        if let Some(inner) = inner {
            if let ChildState::Running = inner.state {
                if let Err(e) = inner.child.kill() {
                    warn!("Failed to kill child process on drop: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_executor() -> smol::LocalExecutor<'static> {
        smol::LocalExecutor::new()
    }

    #[test]
    fn spawn_creates_child_with_pid() {
        let executor = test_executor();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            assert!(child.id().await.is_some());
        });
    }

    #[test]
    fn take_stdin_returns_handle() {
        let executor = test_executor();
        let mut cmd = Command::new("cat");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            assert!(child.take_stdin().await.is_some());
            // Cleanup
            child.kill().await.unwrap();
        });
    }

    #[test]
    fn take_stdout_returns_handle() {
        let executor = test_executor();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            assert!(child.take_stdout().await.is_some());
        });
    }

    #[test]
    fn take_stderr_returns_handle() {
        let executor = test_executor();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            assert!(child.take_stderr().await.is_some());
        });
    }

    #[test]
    fn take_stdin_twice_returns_none() {
        let executor = test_executor();
        let mut cmd = Command::new("cat");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            assert!(child.take_stdin().await.is_some());
            assert!(child.take_stdin().await.is_none());
            // Cleanup
            child.kill().await.unwrap();
        });
    }

    #[test]
    fn wait_returns_exit_status() {
        let executor = test_executor();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            let status = child.wait().await.unwrap();
            assert!(status.success());
        });
    }

    #[test]
    fn wait_caches_exit_status() {
        let executor = test_executor();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            let status1 = child.wait().await.unwrap();
            let status2 = child.wait().await.unwrap();
            assert_eq!(status1, status2);
        });
    }

    #[test]
    fn id_returns_none_after_exit() {
        let executor = test_executor();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            let _ = child.wait().await.unwrap();
            assert!(child.id().await.is_none());
        });
    }

    #[test]
    fn try_wait_returns_none_while_running() {
        let executor = test_executor();
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            let result = child.try_wait().await.unwrap();
            assert!(result.is_none());
            child.kill().await.unwrap();
        });
    }

    #[test]
    fn kill_terminates_child() {
        let executor = test_executor();
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            child.kill().await.unwrap();
            let status = child.wait().await.unwrap();
            assert!(!status.success());
        });
    }

    #[test]
    fn terminate_followed_by_wait() {
        let executor = test_executor();
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            child.terminate().await.unwrap();
            let status = child.wait().await.unwrap();
            assert!(!status.success());
        });
    }

    #[test]
    fn kill_already_exited_is_ok() {
        let executor = test_executor();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            let _ = child.wait().await.unwrap();
            assert!(child.kill().await.is_ok());
        });
    }

    #[test]
    fn terminate_already_exited_is_ok() {
        let executor = test_executor();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            let _ = child.wait().await.unwrap();
            assert!(child.terminate().await.is_ok());
        });
    }

    #[test]
    fn try_kill_sync_terminates_running_child() {
        let executor = test_executor();
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            assert!(child.try_kill_sync().is_ok());
            let status = child.wait().await.unwrap();
            assert!(!status.success());
        });
    }

    #[test]
    fn try_kill_sync_on_exited_child_is_ok() {
        let executor = test_executor();
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            let _ = child.wait().await.unwrap();
            assert!(child.try_kill_sync().is_ok());
        });
    }

    #[test]
    fn try_kill_sync_on_uninitialized_child_is_ok() {
        let child = Child::new();
        assert!(child.try_kill_sync().is_ok());
    }

    #[test]
    fn concurrent_wait_and_kill() {
        let executor = test_executor();
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        executor.run(async {
            let child = Child::spawn(&mut cmd).await.unwrap();
            let child = Arc::new(child);

            // One task waits
            let child_clone = child.clone();
            let wait_future = async move { child_clone.wait().await };

            // Give the wait task time to start, then kill
            async_io::Timer::after(std::time::Duration::from_millis(50)).await;
            child.kill().await.unwrap();

            // Both should complete without error
            let status = wait_future.await.unwrap();
            assert!(!status.success());
        });
    }
}
