use nvim_oxi::{
    Object,
    conversion::{Error, FromObject},
};

use super::dict_from_object;
use super::permissions::{Permissions, PermissionsPartial};

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

/// Returns whether `partial` changes the connect-time elicitation settings
/// (`form`/`url`) relative to `old`. Those settings are only advertised at connect
/// time, so any change requires a reconnect to take effect. The
/// `reject_unknown_elicitation_values` toggle is intentionally excluded: it is
/// applied at response-handling time and takes effect immediately.
pub fn elicitation_changed(partial: Option<&PermissionsPartial>, old: &Permissions) -> bool {
    let Some(elicitation) = partial.and_then(|p| p.elicitation.as_ref()) else {
        return false;
    };
    elicitation.form.is_some_and(|v| v != old.elicitation.form)
        || elicitation.url.is_some_and(|v| v != old.elicitation.url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvim_oxi::Dictionary;

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
    fn elicitation_changed_returns_true_when_form_enabled() {
        let old = Permissions {
            elicitation: ElicitationPermissions {
                form: false,
                url: true,
                reject_unknown_elicitation_values: false,
            },
            ..Default::default()
        };
        let partial = PermissionsPartial {
            elicitation: Some(ElicitationPermissionsPartial {
                form: Some(true),
                url: None,
                reject_unknown_elicitation_values: None,
            }),
            ..Default::default()
        };

        assert!(elicitation_changed(Some(&partial), &old));
    }

    #[test]
    fn elicitation_changed_returns_true_when_form_disabled() {
        let old = Permissions {
            elicitation: ElicitationPermissions {
                form: true,
                url: true,
                reject_unknown_elicitation_values: false,
            },
            ..Default::default()
        };
        let partial = PermissionsPartial {
            elicitation: Some(ElicitationPermissionsPartial {
                form: Some(false),
                url: None,
                reject_unknown_elicitation_values: None,
            }),
            ..Default::default()
        };

        assert!(elicitation_changed(Some(&partial), &old));
    }

    #[test]
    fn elicitation_changed_returns_true_when_url_enabled() {
        let old = Permissions {
            elicitation: ElicitationPermissions {
                form: true,
                url: false,
                reject_unknown_elicitation_values: false,
            },
            ..Default::default()
        };
        let partial = PermissionsPartial {
            elicitation: Some(ElicitationPermissionsPartial {
                form: None,
                url: Some(true),
                reject_unknown_elicitation_values: None,
            }),
            ..Default::default()
        };

        assert!(elicitation_changed(Some(&partial), &old));
    }

    #[test]
    fn elicitation_changed_returns_false_when_settings_unchanged() {
        let old = Permissions {
            elicitation: ElicitationPermissions {
                form: true,
                url: true,
                reject_unknown_elicitation_values: false,
            },
            ..Default::default()
        };
        let partial = PermissionsPartial {
            elicitation: Some(ElicitationPermissionsPartial {
                form: Some(true),
                url: Some(true),
                reject_unknown_elicitation_values: None,
            }),
            ..Default::default()
        };

        assert!(!elicitation_changed(Some(&partial), &old));
    }

    #[test]
    fn elicitation_changed_returns_false_without_elicitation_partial() {
        let old = Permissions::default();

        assert!(!elicitation_changed(None, &old));
    }
}
