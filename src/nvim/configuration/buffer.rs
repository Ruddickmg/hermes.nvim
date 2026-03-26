use nvim_oxi::{
    Object,
    conversion::{Error, FromObject},
};

use super::dict_from_object;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BufferConfig {
    pub auto_save: bool,
}

/// Partial buffer configuration where each field is optional
#[derive(Clone, Debug, Default)]
pub struct BufferConfigPartial {
    pub auto_save: Option<bool>,
}

impl BufferConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut BufferConfig) {
        if let Some(val) = self.auto_save {
            config.auto_save = val;
        }
    }
}

impl FromObject for BufferConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = dict_from_object(obj)?;
        let auto_save = dict
            .get("auto_save")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        Ok(Self { auto_save })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_partial_apply_to_updates_specified() {
        let mut config = BufferConfig::default(); // auto_save=false
        let partial = BufferConfigPartial {
            auto_save: Some(true),
        };
        partial.apply_to(&mut config);
        assert!(config.auto_save); // changed
    }

    #[test]
    fn test_buffer_partial_apply_to_preserves_when_none() {
        let mut config = BufferConfig { auto_save: true };
        let partial = BufferConfigPartial::default(); // None
        partial.apply_to(&mut config);
        assert!(config.auto_save); // preserved
    }

    #[test]
    fn test_buffer_partial_from_object_parses_correctly() {
        let mut dict = nvim_oxi::Dictionary::new();
        dict.insert("auto_save", true);

        let obj = nvim_oxi::Object::from(dict);
        let partial = BufferConfigPartial::from_object(obj).expect("Should parse");

        assert_eq!(partial.auto_save, Some(true));
    }

    #[test]
    fn test_buffer_partial_from_object_empty_dict() {
        let dict = nvim_oxi::Dictionary::default();
        let obj = nvim_oxi::Object::from(dict);
        let partial = BufferConfigPartial::from_object(obj).expect("Should parse");

        assert_eq!(partial.auto_save, None);
    }
}
