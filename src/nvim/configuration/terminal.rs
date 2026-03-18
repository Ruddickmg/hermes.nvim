#[derive(Clone, Debug)]
pub struct TerminalConfig {
    pub delete_on_end: bool,
    pub hidden: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            delete_on_end: false,
            hidden: true,
        }
    }
}
