use async_process::Child;
use std::io;
use std::mem::MaybeUninit;

/// A handle to a child process on Unix. Just the PID, since `waitid()` takes a PID.
#[derive(Copy, Clone)]
pub struct Handle(u32);

/// Extract a platform handle from an async_process Child.
///
/// # Panics
/// Panics if the child has already been reaped (id() returns None).
pub fn get_handle(child: &Child) -> Handle {
    Handle(child.id())
}

/// Block until the child exits, **without reaping it**.
///
/// Uses `waitid(P_PID, WEXITED | WNOWAIT)` which waits for the child to change state
/// but does not remove it from the process table. This keeps the PID valid for concurrent
/// signal delivery via `kill()`.
///
/// This function is meant to be called via `blocking::unblock`.
pub fn wait_noreap(handle: Handle) -> io::Result<()> {
    loop {
        let mut siginfo = MaybeUninit::zeroed();
        // SAFETY: We call waitid with a valid PID and properly initialized siginfo.
        // WNOWAIT ensures the child is not reaped, so the PID remains valid.
        let ret = unsafe {
            libc::waitid(
                libc::P_PID,
                handle.0 as libc::id_t,
                siginfo.as_mut_ptr(),
                libc::WEXITED | libc::WNOWAIT,
            )
        };
        if ret == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
        // EINTR: we were interrupted by a signal. Retry.
    }
}

/// Send SIGTERM to the child process (graceful termination request).
pub fn terminate(child: &Child) -> io::Result<()> {
    let pid = child.id();

    // SAFETY: Sending a standard POSIX signal to a process we spawned.
    let ret = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Send SIGKILL to the child process (forced termination).
pub fn force_kill(child: &Child) -> io::Result<()> {
    let pid = child.id();

    // SAFETY: Sending SIGKILL to a process we spawned. Cannot be caught or ignored.
    let ret = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}
