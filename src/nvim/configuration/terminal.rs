pub struct TerminalConfig {
    pub shell: String,
    pub hidden: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
            hidden: true,
        }
    }
}
