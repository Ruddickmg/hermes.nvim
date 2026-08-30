use nvim_oxi::{
    Object,
    conversion::{Error, FromObject},
};

use super::dict_from_object;

#[derive(Debug, Clone, PartialEq)]
pub struct Permissions {
    pub fs_write_access: bool,
    pub fs_read_access: bool,
    pub terminal_access: bool,
    pub request_permissions: bool,
    pub send_notifications: bool,
    pub elicitation_form: bool,
    pub elicitation_url: bool,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            fs_write_access: true,
            fs_read_access: true,
            terminal_access: true,
            request_permissions: true,
            send_notifications: true,
            elicitation_form: true,
            elicitation_url: true,
        }
    }
}

impl FromObject for Permissions {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

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

        let request_permissions = dict
            .get("request_permissions")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        let send_notifications = dict
            .get("send_notifications")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        let elicitation_form = dict
            .get("elicitation_form")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        let elicitation_url = dict
            .get("elicitation_url")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        Ok(Self {
            fs_write_access,
            fs_read_access,
            terminal_access,
            request_permissions,
            send_notifications,
            elicitation_form,
            elicitation_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvim_oxi::Dictionary;
    use proptest::prelude::*;

    // Strategy for generating Permissions with random boolean values
    fn arb_permissions() -> impl Strategy<Value = Permissions> {
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(
                |(fs_write, fs_read, terminal, can_request, allow_notif, elic_form, elic_url)| {
                    Permissions {
                        fs_write_access: fs_write,
                        fs_read_access: fs_read,
                        terminal_access: terminal,
                        request_permissions: can_request,
                        send_notifications: allow_notif,
                        elicitation_form: elic_form,
                        elicitation_url: elic_url,
                    }
                },
            )
    }

    proptest! {
        #[test]
        fn test_permissions_roundtrip(permissions in arb_permissions()) {
            // Build a Dictionary/Object and ensure Permissions::from_object
            // reconstructs the original Permissions value.
            let mut dict = Dictionary::new();
            dict.insert("fs_write_access", permissions.fs_write_access);
            dict.insert("fs_read_access", permissions.fs_read_access);
            dict.insert("terminal_access", permissions.terminal_access);
            dict.insert("request_permissions", permissions.request_permissions);
            dict.insert("send_notifications", permissions.send_notifications);
            dict.insert("elicitation_form", permissions.elicitation_form);
            dict.insert("elicitation_url", permissions.elicitation_url);

            let obj = Object::from(dict);
            let parsed = Permissions::from_object(obj).expect("Permissions::from_object failed");

            prop_assert_eq!(parsed.fs_write_access, permissions.fs_write_access);
            prop_assert_eq!(parsed.fs_read_access, permissions.fs_read_access);
            prop_assert_eq!(parsed.terminal_access, permissions.terminal_access);
            prop_assert_eq!(parsed.request_permissions, permissions.request_permissions);
            prop_assert_eq!(parsed.send_notifications, permissions.send_notifications);
            prop_assert_eq!(parsed.elicitation_form, permissions.elicitation_form);
            prop_assert_eq!(parsed.elicitation_url, permissions.elicitation_url);
        }
    }

    #[test]
    fn test_permissions_default_all_true() {
        let perms = Permissions::default();
        assert!(perms.fs_write_access);
        assert!(perms.fs_read_access);
        assert!(perms.terminal_access);
        assert!(perms.request_permissions);
        assert!(perms.send_notifications);
        assert!(perms.elicitation_form);
        assert!(perms.elicitation_url);
    }

    #[test]
    fn test_permissions_custom_values() {
        let perms = Permissions {
            fs_write_access: false,
            fs_read_access: true,
            terminal_access: false,
            request_permissions: true,
            send_notifications: false,
            elicitation_form: false,
            elicitation_url: true,
        };
        assert!(!perms.fs_write_access);
        assert!(perms.fs_read_access);
        assert!(!perms.terminal_access);
        assert!(perms.request_permissions);
        assert!(!perms.send_notifications);
        assert!(!perms.elicitation_form);
        assert!(perms.elicitation_url);
    }
}

/// Partial permissions configuration where each field is optional
#[derive(Clone, Debug, Default)]
pub struct PermissionsPartial {
    pub fs_write_access: Option<bool>,
    pub fs_read_access: Option<bool>,
    pub terminal_access: Option<bool>,
    pub request_permissions: Option<bool>,
    pub send_notifications: Option<bool>,
    pub elicitation_form: Option<bool>,
    pub elicitation_url: Option<bool>,
}

impl PermissionsPartial {
    /// Apply only Some() values to existing permissions
    pub fn apply_to(self, permissions: &mut Permissions) {
        if let Some(val) = self.fs_write_access {
            permissions.fs_write_access = val;
        }
        if let Some(val) = self.fs_read_access {
            permissions.fs_read_access = val;
        }
        if let Some(val) = self.terminal_access {
            permissions.terminal_access = val;
        }
        if let Some(val) = self.request_permissions {
            permissions.request_permissions = val;
        }
        if let Some(val) = self.send_notifications {
            permissions.send_notifications = val;
        }
        if let Some(val) = self.elicitation_form {
            permissions.elicitation_form = val;
        }
        if let Some(val) = self.elicitation_url {
            permissions.elicitation_url = val;
        }
    }
}

impl FromObject for PermissionsPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

        let fs_write_access = dict
            .get("fs_write_access")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let fs_read_access = dict
            .get("fs_read_access")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let terminal_access = dict
            .get("terminal_access")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let request_permissions = dict
            .get("request_permissions")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let send_notifications = dict
            .get("send_notifications")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let elicitation_form = dict
            .get("elicitation_form")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let elicitation_url = dict
            .get("elicitation_url")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            fs_write_access,
            fs_read_access,
            terminal_access,
            request_permissions,
            send_notifications,
            elicitation_form,
            elicitation_url,
        })
    }
}
