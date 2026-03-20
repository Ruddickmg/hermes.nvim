use nvim_oxi::{
    conversion::{Error, FromObject},
    Dictionary, Object,
};

#[derive(Clone, Debug)]
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
    pub delete_on_end: Option<bool>,
    pub hidden: Option<bool>,
    pub buffered: Option<bool>,
    pub enabled: Option<bool>,
}

impl TerminalConfigPartial {
    /// Apply only Some() values to existing config
    pub fn apply_to(self, config: &mut TerminalConfig) {
        if let Some(val) = self.delete_on_end {
            config.delete = val;
        }
        if let Some(val) = self.hidden {
            config.hidden = val;
        }
    }
}

impl FromObject for TerminalConfigPartial {
    fn from_object(obj: Object) -> Result<Self, Error> {
        let dict = Dictionary::from_object(obj)?;

        let delete_on_end = dict
            .get("delete_on_end")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;
        let hidden = dict
            .get("hidden")
            .map(|o| bool::from_object(o.clone()))
            .transpose()?;

        Ok(Self {
            delete_on_end,
            hidden,
        })
    }
}
