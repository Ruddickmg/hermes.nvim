mod permissions;

pub use permissions::Permissions;

#[derive(Clone, Debug, Default)]
pub struct ClientConfig {
    pub permissions: Permissions,
}
