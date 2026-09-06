use nvim_oxi::{
    Object,
    conversion::{Error, FromObject},
};

use super::dict_from_object;

#[derive(Debug, Clone, PartialEq)]
pub struct ElicitationPermissions {
    pub form: bool,
    pub url: bool,
    pub reject_unknown_elicitation_values: bool,
}

impl Default for ElicitationPermissions {
    fn default() -> Self {
        Self {
            form: true,
            url: true,
            reject_unknown_elicitation_values: false,
        }
    }
}

impl FromObject for ElicitationPermissions {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

        let form = dict
            .get("form")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        let url = dict
            .get("url")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(true);

        let reject_unknown_elicitation_values = dict
            .get("reject_unknown_elicitation_values")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?
            .unwrap_or(false);

        Ok(Self {
            form,
            url,
            reject_unknown_elicitation_values,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Permissions {
    pub fs_write_access: bool,
    pub fs_read_access: bool,
    pub terminal_access: bool,
    pub request_permissions: bool,
    pub send_notifications: bool,
    pub elicitation: ElicitationPermissions,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            fs_write_access: true,
            fs_read_access: true,
            terminal_access: true,
            request_permissions: true,
            send_notifications: true,
            elicitation: ElicitationPermissions::default(),
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

        let elicitation = dict
            .get("elicitation")
            .map(|o| ElicitationPermissions::from_object(o.clone()))
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            fs_write_access,
            fs_read_access,
            terminal_access,
            request_permissions,
            send_notifications,
            elicitation,
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
            any::<bool>(),
        )
            .prop_map(
                |(
                    fs_write,
                    fs_read,
                    terminal,
                    can_request,
                    allow_notif,
                    elic_form,
                    elic_url,
                    reject,
                )| {
                    Permissions {
                        fs_write_access: fs_write,
                        fs_read_access: fs_read,
                        terminal_access: terminal,
                        request_permissions: can_request,
                        send_notifications: allow_notif,
                        elicitation: ElicitationPermissions {
                            form: elic_form,
                            url: elic_url,
                            reject_unknown_elicitation_values: reject,
                        },
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
            let mut elicitation = Dictionary::new();
            elicitation.insert("form", permissions.elicitation.form);
            elicitation.insert("url", permissions.elicitation.url);
            elicitation.insert(
                "reject_unknown_elicitation_values",
                permissions.elicitation.reject_unknown_elicitation_values,
            );
            dict.insert("elicitation", elicitation);

            let obj = Object::from(dict);
            let parsed = Permissions::from_object(obj).expect("Permissions::from_object failed");

            prop_assert_eq!(parsed.fs_write_access, permissions.fs_write_access);
            prop_assert_eq!(parsed.fs_read_access, permissions.fs_read_access);
            prop_assert_eq!(parsed.terminal_access, permissions.terminal_access);
            prop_assert_eq!(parsed.request_permissions, permissions.request_permissions);
            prop_assert_eq!(parsed.send_notifications, permissions.send_notifications);
            prop_assert_eq!(parsed.elicitation, permissions.elicitation);
        }
    }

    #[test]
    fn test_permissions_elicitation_defaults_to_all_true() {
        let perms = Permissions::default();
        assert_eq!(
            perms.elicitation,
            ElicitationPermissions {
                form: true,
                url: true,
                reject_unknown_elicitation_values: false,
            }
        );
    }

    #[test]
    fn test_permissions_elicitation_from_object_parses_nested() {
        let mut elicitation = Dictionary::new();
        elicitation.insert("form", false);
        elicitation.insert("url", true);
        elicitation.insert("reject_unknown_elicitation_values", true);
        let mut dict = Dictionary::new();
        dict.insert("elicitation", elicitation);

        let parsed =
            Permissions::from_object(Object::from(dict)).expect("Permissions::from_object failed");

        assert_eq!(
            parsed.elicitation,
            ElicitationPermissions {
                form: false,
                url: true,
                reject_unknown_elicitation_values: true,
            }
        );
    }

    #[test]
    fn test_permissions_custom_values() {
        let perms = Permissions {
            fs_write_access: false,
            fs_read_access: true,
            terminal_access: false,
            request_permissions: true,
            send_notifications: false,
            elicitation: ElicitationPermissions {
                form: false,
                url: true,
                reject_unknown_elicitation_values: true,
            },
        };
        assert_eq!(
            perms.elicitation,
            ElicitationPermissions {
                form: false,
                url: true,
                reject_unknown_elicitation_values: true,
            }
        );
    }

    #[test]
    fn test_elicitation_permissions_default_all_true() {
        let elic = ElicitationPermissions::default();
        assert_eq!(
            elic,
            ElicitationPermissions {
                form: true,
                url: true,
                reject_unknown_elicitation_values: false,
            }
        );
    }

    #[test]
    fn test_elicitation_permissions_from_object_parses_nested() {
        let mut dict = Dictionary::new();
        dict.insert("form", true);
        dict.insert("url", false);
        dict.insert("reject_unknown_elicitation_values", true);

        let parsed = ElicitationPermissions::from_object(Object::from(dict))
            .expect("ElicitationPermissions::from_object failed");

        assert_eq!(
            parsed,
            ElicitationPermissions {
                form: true,
                url: false,
                reject_unknown_elicitation_values: true,
            }
        );
    }

    #[test]
    fn test_elicitation_permissions_partial_from_object_parses_nested() {
        let mut dict = Dictionary::new();
        dict.insert("form", false);
        dict.insert("url", false);
        dict.insert("reject_unknown_elicitation_values", true);

        let parsed = ElicitationPermissionsPartial::from_object(Object::from(dict))
            .expect("ElicitationPermissionsPartial::from_object failed");

        assert_eq!(
            parsed,
            ElicitationPermissionsPartial {
                form: Some(false),
                url: Some(false),
                reject_unknown_elicitation_values: Some(true),
            }
        );
    }

    #[test]
    fn test_permissions_partial_elicitation_apply_to_nested() {
        let mut perms = Permissions::default();
        let partial = PermissionsPartial {
            elicitation: Some(ElicitationPermissionsPartial {
                form: Some(false),
                url: None,
                reject_unknown_elicitation_values: Some(true),
            }),
            ..Default::default()
        };

        partial.apply_to(&mut perms);

        assert_eq!(
            perms.elicitation,
            ElicitationPermissions {
                form: false,
                url: true,
                reject_unknown_elicitation_values: true,
            }
        );
    }
}

/// Partial permissions configuration where each field is optional
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ElicitationPermissionsPartial {
    pub form: Option<bool>,
    pub url: Option<bool>,
    pub reject_unknown_elicitation_values: Option<bool>,
}

impl FromObject for ElicitationPermissionsPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

        let form = dict
            .get("form")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let url = dict
            .get("url")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let reject_unknown_elicitation_values = dict
            .get("reject_unknown_elicitation_values")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            form,
            url,
            reject_unknown_elicitation_values,
        })
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
    pub elicitation: Option<ElicitationPermissionsPartial>,
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
        if let Some(elicitation) = self.elicitation {
            if let Some(val) = elicitation.form {
                permissions.elicitation.form = val;
            }
            if let Some(val) = elicitation.url {
                permissions.elicitation.url = val;
            }
            if let Some(val) = elicitation.reject_unknown_elicitation_values {
                permissions.elicitation.reject_unknown_elicitation_values = val;
            }
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
        let elicitation = dict
            .get("elicitation")
            .map(|o| ElicitationPermissionsPartial::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            fs_write_access,
            fs_read_access,
            terminal_access,
            request_permissions,
            send_notifications,
            elicitation,
        })
    }
}
