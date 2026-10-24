use nvim_oxi::{
    Object,
    conversion::{Error, FromObject},
};
use serde::{Deserialize, Serialize};

use crate::utilities::is_nix_environment;

use super::dict_from_object;

#[derive(Serialize, Deserialize, std::hash::Hash, Debug, Clone, PartialEq, Eq)]
pub struct BinaryConfig {
    pub path: String,
    pub enabled: bool,
}

impl Default for BinaryConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            enabled: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, std::hash::Hash, PartialEq, Eq)]
pub struct DistributionsConfig {
    pub binary: BinaryConfig,
    pub uvx: bool,
    pub npx: bool,
}

impl DistributionsConfig {
    pub(crate) fn defaults(is_nix: bool) -> Self {
        Self {
            binary: BinaryConfig {
                enabled: !is_nix,
                ..BinaryConfig::default()
            },
            uvx: !is_nix,
            npx: !is_nix,
        }
    }
}

impl Default for DistributionsConfig {
    fn default() -> Self {
        Self::defaults(is_nix_environment())
    }
}

#[derive(Clone, Debug, Default)]
pub struct BinaryConfigPartial {
    pub path: Option<String>,
    pub enabled: Option<bool>,
}

impl BinaryConfigPartial {
    pub fn apply_to(self, config: &mut BinaryConfig) {
        if let Some(val) = self.path {
            config.path = val;
        }
        if let Some(val) = self.enabled {
            config.enabled = val;
        }
    }
}

impl FromObject for BinaryConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;
        let path = dict
            .get("path")
            .map(|o| String::from_object(o.clone()))
            .transpose()?;
        let enabled = dict
            .get("enabled")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        Ok(Self { path, enabled })
    }
}

#[derive(Clone, Debug, Default)]
pub struct DistributionsConfigPartial {
    pub binary: Option<BinaryConfigPartial>,
    pub uvx: Option<bool>,
    pub npx: Option<bool>,
}

impl DistributionsConfigPartial {
    pub fn apply_to(self, config: &mut DistributionsConfig) {
        if let Some(val) = self.binary {
            val.apply_to(&mut config.binary);
        }
        if let Some(val) = self.uvx {
            config.uvx = val;
        }
        if let Some(val) = self.npx {
            config.npx = val;
        }
    }
}

impl FromObject for DistributionsConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;

        let binary = dict
            .get("binary")
            .map(|o| BinaryConfigPartial::from_object(o.clone()))
            .transpose()?;

        let uvx = dict
            .get("uvx")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;

        let npx = dict
            .get("npx")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;

        Ok(Self { binary, uvx, npx })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvim_oxi::Dictionary;
    use pretty_assertions::assert_eq;

    #[test]
    fn binary_config_default_path_empty() {
        let config = BinaryConfig::default();
        assert!(config.path.is_empty());
        assert!(config.enabled);
    }

    #[test]
    fn distributions_config_defaults_outside_nix() {
        let config = DistributionsConfig::defaults(false);
        assert_eq!(
            (config.binary.enabled, config.uvx, config.npx),
            (true, true, true)
        );
    }

    #[test]
    fn distributions_config_defaults_in_nix() {
        let config = DistributionsConfig::defaults(true);
        assert_eq!(
            (config.binary.enabled, config.uvx, config.npx),
            (false, false, false)
        );
    }

    #[test]
    fn binary_partial_apply_to_path() {
        let mut config = BinaryConfig::default();
        let partial = BinaryConfigPartial {
            path: Some("/custom/path".to_string()),
            enabled: None,
        };
        partial.apply_to(&mut config);
        assert_eq!(config.path, "/custom/path");
        assert!(config.enabled);
    }

    #[test]
    fn binary_partial_apply_to_enabled() {
        let mut config = BinaryConfig::default();
        let partial = BinaryConfigPartial {
            path: None,
            enabled: Some(false),
        };
        partial.apply_to(&mut config);
        assert!(config.path.is_empty());
        assert!(!config.enabled);
    }

    #[test]
    fn distributions_partial_apply_to_all_fields() {
        let mut config = DistributionsConfig::default();
        let partial = DistributionsConfigPartial {
            binary: Some(BinaryConfigPartial {
                path: Some("/p".to_string()),
                enabled: Some(false),
            }),
            uvx: Some(false),
            npx: Some(false),
        };
        partial.apply_to(&mut config);
        assert_eq!(config.binary.path, "/p");
        assert!(!config.binary.enabled);
        assert!(!config.uvx);
        assert!(!config.npx);
    }

    #[test]
    fn distributions_partial_apply_to_preserves_unspecified() {
        let mut config = DistributionsConfig {
            binary: BinaryConfig {
                path: "/original".to_string(),
                enabled: false,
            },
            uvx: false,
            npx: false,
        };
        let partial = DistributionsConfigPartial::default();
        partial.apply_to(&mut config);
        assert_eq!(config.binary.path, "/original");
        assert!(!config.binary.enabled);
        assert!(!config.uvx);
        assert!(!config.npx);
    }

    #[test]
    fn binary_partial_from_object_with_path() {
        let mut dict = Dictionary::new();
        dict.insert("path", "/custom");
        let obj = Object::from(dict);
        let partial = BinaryConfigPartial::from_object(obj).expect("should parse");
        assert_eq!(partial.path, Some("/custom".to_string()));
        assert_eq!(partial.enabled, None);
    }

    #[test]
    fn binary_partial_from_object_with_enabled() {
        let mut dict = Dictionary::new();
        dict.insert("enabled", false);
        let obj = Object::from(dict);
        let partial = BinaryConfigPartial::from_object(obj).expect("should parse");
        assert_eq!(partial.path, None);
        assert_eq!(partial.enabled, Some(false));
    }

    #[test]
    fn binary_partial_from_object_empty_dict() {
        let dict = Dictionary::new();
        let obj = Object::from(dict);
        let partial = BinaryConfigPartial::from_object(obj).expect("should parse");
        assert_eq!(partial.path, None);
        assert_eq!(partial.enabled, None);
    }

    #[test]
    fn distributions_partial_from_object_all_fields() {
        let mut binary_dict = Dictionary::new();
        binary_dict.insert("path", "/p");
        binary_dict.insert("enabled", false);

        let mut dict = Dictionary::new();
        dict.insert("binary", binary_dict);
        dict.insert("uvx", false);
        dict.insert("npx", true);

        let obj = Object::from(dict);
        let partial = DistributionsConfigPartial::from_object(obj).expect("should parse");
        assert!(partial.binary.is_some());
        assert_eq!(partial.binary.as_ref().unwrap().path.as_deref(), Some("/p"));
        assert_eq!(partial.binary.as_ref().unwrap().enabled, Some(false));
        assert_eq!(partial.uvx, Some(false));
        assert_eq!(partial.npx, Some(true));
    }

    #[test]
    fn distributions_partial_from_object_empty_dict() {
        let dict = Dictionary::new();
        let obj = Object::from(dict);
        let partial = DistributionsConfigPartial::from_object(obj).expect("should parse");
        assert!(partial.binary.is_none());
        assert_eq!(partial.uvx, None);
        assert_eq!(partial.npx, None);
    }
}
