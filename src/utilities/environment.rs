use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::ffi::CStr;

const NIX_STORE_MARKER: &str = "/nix/store/";

#[cfg(unix)]
static LIBRARY_MARKER: u8 = 0;

pub fn path_is_nix_store(path: &str) -> bool {
    path.contains(NIX_STORE_MARKER)
}

pub fn nix_environment(is_nixos: bool, library_path: Option<&str>) -> bool {
    is_nixos || library_path.is_some_and(path_is_nix_store)
}

#[cfg(unix)]
pub fn is_nixos() -> bool {
    Path::new("/etc/NIXOS").exists()
}

#[cfg(not(unix))]
pub fn is_nixos() -> bool {
    false
}

#[cfg(unix)]
fn loaded_library_path() -> Option<PathBuf> {
    let address = std::ptr::addr_of!(LIBRARY_MARKER).cast::<libc::c_void>();
    // SAFETY: `dladdr` only reads the memory mapping containing `address` and
    // writes the result into `info`. A successful call fully initializes `info`.
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    let found = unsafe { libc::dladdr(address, &mut info) };
    if found == 0 || info.dli_fname.is_null() {
        return None;
    }
    // SAFETY: `dladdr` guarantees `dli_fname` is a valid NUL-terminated string
    // when it returns a non-zero value.
    let path = unsafe { CStr::from_ptr(info.dli_fname) };
    Some(PathBuf::from(path.to_string_lossy().as_ref()))
}

#[cfg(not(unix))]
fn loaded_library_path() -> Option<PathBuf> {
    None
}

pub fn is_nix_environment() -> bool {
    let library_path = loaded_library_path();
    nix_environment(is_nixos(), library_path.as_deref().and_then(Path::to_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nix_environment_true_on_nixos() {
        assert!(nix_environment(
            true,
            Some("/home/user/.local/share/hermes/libhermes.so")
        ));
    }

    #[test]
    fn nix_environment_true_for_nix_store_install() {
        assert!(nix_environment(
            false,
            Some("/nix/store/abc-hermes/lib/libhermes.so")
        ));
    }

    #[test]
    fn nix_environment_false_outside_nix() {
        assert!(!nix_environment(
            false,
            Some("/home/user/.local/share/hermes/libhermes.so")
        ));
    }

    #[test]
    fn nix_environment_false_without_library_path() {
        assert!(!nix_environment(false, None));
    }
}
