use nvim_oxi::{
    conversion::{Error, FromObject},
    lua::{self, Poppable, Pushable},
    Dictionary, Object,
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
