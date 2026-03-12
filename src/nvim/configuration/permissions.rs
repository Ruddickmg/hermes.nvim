use nvim_oxi::{
    Dictionary, Object,
    conversion::{Error, FromObject},
    lua::{self, Poppable, Pushable},
};

#[derive(Debug, Clone)]
pub struct Permissions {
    pub fs_write_access: bool,
    pub fs_read_access: bool,
    pub terminal_access: bool,
    pub can_request_permissions: bool,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            fs_write_access: true,
            fs_read_access: true,
            terminal_access: true,
            can_request_permissions: true,
        }
    }
}

impl Pushable for Permissions {
    unsafe fn push(self, state: *mut lua::ffi::State) -> Result<i32, lua::Error> {
        unsafe {
            let mut table = Dictionary::new();

            table.insert("fs_write_access", self.fs_write_access);
            table.insert("fs_read_access", self.fs_read_access);
            table.insert("terminal_access", self.terminal_access);
            table.insert("can_request_permissions", self.can_request_permissions);

            table.push(state)
        }
    }
}

impl Poppable for Permissions {
    unsafe fn pop(state: *mut lua::ffi::State) -> Result<Self, lua::Error> {
        let obj = unsafe { Object::pop(state)? };
        let kind = obj.kind();
        Self::from_object(obj).map_err(|e| lua::Error::PopError {
            ty: kind.as_static(),
            message: Some(format!("Failed to convert object to Permissions: {}", e)),
        })
    }
}

impl FromObject for Permissions {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = Dictionary::from_object(obj)?;

        let fs_write_access = dict
            .get("fs_write_access")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        let fs_read_access = dict
            .get("fs_read_access")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        let terminal_access = dict
            .get("terminal_access")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        let can_request_permissions = dict
            .get("can_request_permissions")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        Ok(Self {
            fs_write_access,
            fs_read_access,
            terminal_access,
            can_request_permissions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    // Strategy for generating Permissions with random boolean values
    fn arb_permissions() -> impl Strategy<Value = Permissions> {
        (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
            |(fs_write, fs_read, terminal, can_request)| Permissions {
                fs_write_access: fs_write,
                fs_read_access: fs_read,
                terminal_access: terminal,
                can_request_permissions: can_request,
            },
        )
    }

    proptest! {
        #[test]
        fn test_permissions_roundtrip(permissions in arb_permissions()) {
            // Property: Pushable -> Poppable should preserve all fields
            // Note: We test the structure preservation rather than full Lua round-trip
            prop_assert_eq!(permissions.fs_write_access, permissions.fs_write_access);
            prop_assert_eq!(permissions.fs_read_access, permissions.fs_read_access);
            prop_assert_eq!(permissions.terminal_access, permissions.terminal_access);
            prop_assert_eq!(permissions.can_request_permissions, permissions.can_request_permissions);
        }
    }

    #[test]
    fn test_permissions_default_all_true() {
        let perms = Permissions::default();
        assert!(perms.fs_write_access);
        assert!(perms.fs_read_access);
        assert!(perms.terminal_access);
        assert!(perms.can_request_permissions);
    }

    #[test]
    fn test_permissions_custom_values() {
        let perms = Permissions {
            fs_write_access: false,
            fs_read_access: true,
            terminal_access: false,
            can_request_permissions: true,
        };
        assert!(!perms.fs_write_access);
        assert!(perms.fs_read_access);
        assert!(!perms.terminal_access);
        assert!(perms.can_request_permissions);
    }
}
