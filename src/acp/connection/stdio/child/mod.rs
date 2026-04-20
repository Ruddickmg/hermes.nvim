mod sys;

use crate::acp::Result;
use std::io;
use std::process::ExitStatus;
use std::sync::Arc;
use tokio::process::{Child as TokioChild, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tracing::{debug, trace, warn};

#[derive(Debug)]
enum ChildState {
    Running,
    Exited(ExitStatus),
}

#[derive(Debug)]
struct ChildInner {
    child: TokioChild,
    state: ChildState,
}

/// A wrapper around a tokio child process that supports lazy initialization.
///
/// The child process can be created in two phases:
/// 1. `Child::new()` creates an uninitialized handle (no process spawned yet)
/// 2. `Child::initialize()` spawns the actual process
///
/// This two-phase construction allows the `Child` to be shared (via `Arc`) between
/// threads before the process is spawned. The process must be spawned on the same
/// tokio runtime whose reactor will handle its IO, since tokio IO handles are
/// reactor-bound.
///
/// For convenience, `Child::spawn()` performs both phases in one call.
#[derive(Debug)]
pub struct Child {
    inner: Mutex<Option<ChildInner>>,
    exit_notify: Arc<tokio::sync::Notify>,
}

impl Child {
    /// Create an uninitialized child handle. No process is spawned yet.
    ///
    /// Call `initialize()` on the target runtime's thread before using IO methods.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            exit_notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Spawn the child process. Must be called on the tokio runtime whose
    /// reactor will be used for IO (stdin/stdout).
    pub async fn initialize(&self, command: &mut Command) -> Result<()> {
        let child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
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
    /// spawned on the current tokio runtime.
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
            ChildState::Running => inner.child.id(),
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
        tokio::task::spawn_blocking(move || sys::wait_noreap(handle)).await??;

        let mut inner = self.inner.lock().await;
        let inner = inner
            .as_mut()
            .ok_or_else(|| io::Error::other("child not initialized"))?;

        if let ChildState::Exited(status) = inner.state {
            return Ok(status);
        }

        let status = inner
            .child
            .try_wait()?
            .ok_or_else(|| io::Error::other("child not exited after wait"))?;

        inner.state = ChildState::Exited(status);
        self.exit_notify.notify_waiters();
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
                if let Some(status) = inner.child.try_wait()? {
                    inner.state = ChildState::Exited(status);
                    self.exit_notify.notify_waiters();
                    Ok(Some(status))
                } else {
                    Ok(None)
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
}

impl Drop for Child {
    fn drop(&mut self) {
        let inner = self.inner.get_mut();
        if let Some(inner) = inner {
            if let ChildState::Running = inner.state {
                if let Err(e) = inner.child.start_kill() {
                    warn!("Failed to kill child process on drop: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spawn_creates_child_with_pid() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let child = Child::spawn(&mut cmd).await.unwrap();
        assert!(child.id().await.is_some());
    }

    #[tokio::test]
    async fn take_stdin_returns_handle() {
        let mut cmd = Command::new("cat");
        let child = Child::spawn(&mut cmd).await.unwrap();
        assert!(child.take_stdin().await.is_some());
        // Cleanup
        child.kill().await.unwrap();
    }

    #[tokio::test]
    async fn take_stdout_returns_handle() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let child = Child::spawn(&mut cmd).await.unwrap();
        assert!(child.take_stdout().await.is_some());
    }

    #[tokio::test]
    async fn take_stderr_returns_handle() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let child = Child::spawn(&mut cmd).await.unwrap();
        assert!(child.take_stderr().await.is_some());
    }

    #[tokio::test]
    async fn take_stdin_twice_returns_none() {
        let mut cmd = Command::new("cat");
        let child = Child::spawn(&mut cmd).await.unwrap();
        assert!(child.take_stdin().await.is_some());
        assert!(child.take_stdin().await.is_none());
        // Cleanup
        child.kill().await.unwrap();
    }

    #[tokio::test]
    async fn wait_returns_exit_status() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let child = Child::spawn(&mut cmd).await.unwrap();
        let status = child.wait().await.unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn wait_caches_exit_status() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let child = Child::spawn(&mut cmd).await.unwrap();
        let status1 = child.wait().await.unwrap();
        let status2 = child.wait().await.unwrap();
        assert_eq!(status1, status2);
    }

    #[tokio::test]
    async fn id_returns_none_after_exit() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let child = Child::spawn(&mut cmd).await.unwrap();
        let _ = child.wait().await.unwrap();
        assert!(child.id().await.is_none());
    }

    #[tokio::test]
    async fn try_wait_returns_none_while_running() {
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        let child = Child::spawn(&mut cmd).await.unwrap();
        let result = child.try_wait().await.unwrap();
        assert!(result.is_none());
        child.kill().await.unwrap();
    }

    #[tokio::test]
    async fn kill_terminates_child() {
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        let child = Child::spawn(&mut cmd).await.unwrap();
        child.kill().await.unwrap();
        let status = child.wait().await.unwrap();
        assert!(!status.success());
    }

    #[tokio::test]
    async fn terminate_followed_by_wait() {
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        let child = Child::spawn(&mut cmd).await.unwrap();
        child.terminate().await.unwrap();
        let status = child.wait().await.unwrap();
        assert!(!status.success());
    }

    #[tokio::test]
    async fn kill_already_exited_is_ok() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let child = Child::spawn(&mut cmd).await.unwrap();
        let _ = child.wait().await.unwrap();
        assert!(child.kill().await.is_ok());
    }

    #[tokio::test]
    async fn terminate_already_exited_is_ok() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let child = Child::spawn(&mut cmd).await.unwrap();
        let _ = child.wait().await.unwrap();
        assert!(child.terminate().await.is_ok());
    }

    #[tokio::test]
    async fn concurrent_wait_and_kill() {
        let mut cmd = Command::new("sleep");
        cmd.arg("60");
        let child = Child::spawn(&mut cmd).await.unwrap();
        let child = Arc::new(child);

        // One task waits
        let child_clone = child.clone();
        let wait_handle = tokio::spawn(async move { child_clone.wait().await });

        // Give the wait task time to start, then kill
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        child.kill().await.unwrap();

        // Both should complete without error
        let status = wait_handle.await.unwrap().unwrap();
        assert!(!status.success());
    }
}
