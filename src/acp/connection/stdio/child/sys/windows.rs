//! Windows implementation of child process management.
//!
//! Uses `WaitForSingleObject()` on the process handle to wait without reaping,
//! and `TerminateProcess()` for both graceful and forced termination (Windows has
//! no SIGTERM equivalent).
//!
//! Windows prevents PID reuse while process handles are open, so concurrent
//! wait/kill is inherently safe as long as we hold a handle.

use std::io;
use std::os::windows::io::{AsRawHandle, RawHandle};
use tokio::process::Child;

/// A handle to a child process on Windows. Uses the raw OS handle.
#[derive(Copy, Clone)]
pub struct Handle(RawHandle);

// SAFETY: RawHandle is just a pointer-sized integer on Windows.
// The handle is valid as long as the Child exists, and we only use it
// in spawn_blocking before the Child is dropped.
unsafe impl Send for Handle {}

#[link(name = "kernel32")]
#[allow(non_snake_case)]
unsafe extern "system" {
    fn WaitForSingleObject(hHandle: *mut std::ffi::c_void, dwMilliseconds: u32) -> u32;
    fn OpenProcess(
        dwDesiredAccess: u32,
        bInheritHandle: i32,
        dwProcessId: u32,
    ) -> *mut std::ffi::c_void;
    fn TerminateProcess(hProcess: *mut std::ffi::c_void, uExitCode: u32) -> i32;
    fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
}

const INFINITE: u32 = 0xFFFFFFFF;
const WAIT_OBJECT_0: u32 = 0;
const PROCESS_TERMINATE: u32 = 0x0001;

/// Extract a platform handle from a tokio Child.
///
/// # Panics
/// Panics if the child has already been reaped (id() returns None).
pub fn get_handle(child: &Child) -> Handle {
    Handle(child.raw_handle())
}

/// Block until the child exits, **without reaping it**.
///
/// On Windows, `WaitForSingleObject` does not "reap" the process - the handle
/// remains valid until explicitly closed. This makes concurrent wait/kill safe.
///
/// This function is meant to be called via `tokio::task::spawn_blocking`.
pub fn wait_noreap(handle: Handle) -> io::Result<()> {
    // SAFETY: We call WaitForSingleObject with a valid process handle.
    // INFINITE means we block until the process exits.
    let ret = unsafe { WaitForSingleObject(handle.0 as *mut std::ffi::c_void, INFINITE) };
    if ret == WAIT_OBJECT_0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Terminate the child process.
///
/// Windows has no equivalent of Unix SIGTERM. We use `TerminateProcess` for both
/// graceful and forced termination.
fn terminate_process(child: &Child) -> io::Result<()> {
    let pid = child
        .id()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "child already reaped"))?;

    // SAFETY: We open the process by PID with PROCESS_TERMINATE access,
    // call TerminateProcess, then close the handle.
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }

    let result = unsafe { TerminateProcess(handle, 1) };
    unsafe {
        CloseHandle(handle);
    }

    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Send graceful termination signal. On Windows, this is the same as force_kill.
pub fn terminate(child: &Child) -> io::Result<()> {
    terminate_process(child)
}

/// Forcefully kill the child process.
pub fn force_kill(child: &Child) -> io::Result<()> {
    terminate_process(child)
}
