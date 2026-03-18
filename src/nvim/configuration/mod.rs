mod permissions;
mod terminal;

pub use permissions::Permissions;
pub use terminal::TerminalConfig;

#[derive(Clone, Debug, Default)]
pub struct ClientConfig {
    pub permissions: Permissions,
    pub terminal: TerminalConfig,
}
