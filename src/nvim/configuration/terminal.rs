use nvim_oxi::{
    conversion::{Error, FromObject},
    Object,
};

use super::dict_from_object;

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalConfig {
    pub delete: bool,
    pub hidden: bool,
    pub enabled: bool,
    pub buffered: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            enabled: true,
            delete: false,
            hidden: true,
            buffered: true,
        }
    }
}

/// Partial terminal configuration where each field is optional
#[derive(Clone, Debug, Default)]
pub struct TerminalConfigPartial {
    pub delete: Option<bool>,
    pub hidden: Option<bool>,
    pub enabled: Option<bool>,
    pub buffered: Option<bool>,
}

impl TerminalConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut TerminalConfig) {
        if let Some(val) = self.delete {
            config.delete = val;
        }
        if let Some(val) = self.hidden {
            config.hidden = val;
        }
        if let Some(val) = self.enabled {
            config.enabled = val;
        }
        if let Some(val) = self.buffered {
            config.buffered = val;
        }
    }
}

impl FromObject for TerminalConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

        let delete = dict
            .get("delete")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let hidden = dict
            .get("hidden")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let enabled = dict
            .get("enabled")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let buffered = dict
            .get("buffered")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            delete,
            hidden,
            enabled,
            buffered,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_partial_apply_to_updates_specified() {
        let mut config = TerminalConfig::default();
        let partial = TerminalConfigPartial {
            delete: Some(true),
            hidden: None,
            enabled: None,
            buffered: None,
        };
        partial.apply_to(&mut config);
        assert!(config.delete); // changed to true
        assert!(config.hidden); // preserved default
        assert!(config.enabled); // preserved default
        assert!(config.buffered); // preserved default
    }

    #[test]
    fn test_terminal_partial_apply_to_preserves_all_when_none() {
        let mut config = TerminalConfig {
            delete: true,
            hidden: false,
            enabled: false,
            buffered: false,
        };
        let partial = TerminalConfigPartial::default(); // all None
        partial.apply_to(&mut config);
        // All should remain unchanged
        assert!(config.delete);
        assert!(!config.hidden);
        assert!(!config.enabled);
        assert!(!config.buffered);
    }

    #[test]
    fn test_terminal_partial_apply_to_updates_all_fields() {
        let mut config = TerminalConfig::default();
        let partial = TerminalConfigPartial {
            delete: Some(true),
            hidden: Some(false),
            enabled: Some(false),
            buffered: Some(false),
        };
        partial.apply_to(&mut config);
        assert!(config.delete);
        assert!(!config.hidden);
        assert!(!config.enabled);
        assert!(!config.buffered);
    }

    #[test]
    fn test_terminal_partial_from_object_parses_correctly() {
        let mut dict = nvim_oxi::Dictionary::new();
        dict.insert("delete", true);
        dict.insert("hidden", false);
        dict.insert("enabled", true);
        dict.insert("buffered", false);

        let obj = nvim_oxi::Object::from(dict);
        let partial = TerminalConfigPartial::from_object(obj).expect("Should parse");

        assert_eq!(partial.delete, Some(true));
        assert_eq!(partial.hidden, Some(false));
        assert_eq!(partial.enabled, Some(true));
        assert_eq!(partial.buffered, Some(false));
    }

    #[test]
    fn test_terminal_partial_from_object_handles_missing_fields() {
        let mut dict = nvim_oxi::Dictionary::new();
        dict.insert("delete", true);
        // other fields missing

        let obj = nvim_oxi::Object::from(dict);
        let partial = TerminalConfigPartial::from_object(obj).expect("Should parse");

        assert_eq!(partial.delete, Some(true));
        assert_eq!(partial.hidden, None); // missing becomes None
        assert_eq!(partial.enabled, None);
        assert_eq!(partial.buffered, None);
    }

    #[test]
    fn test_terminal_partial_from_object_empty_dict() {
        let dict = nvim_oxi::Dictionary::default();
        let obj = nvim_oxi::Object::from(dict);
        let partial = TerminalConfigPartial::from_object(obj).expect("Should parse");

        assert_eq!(partial.delete, None);
        assert_eq!(partial.hidden, None);
        assert_eq!(partial.enabled, None);
        assert_eq!(partial.buffered, None);
    }
}
