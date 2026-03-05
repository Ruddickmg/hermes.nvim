mod callbacks;
mod permissions;

// pub use callbacks::Callbacks;
pub use permissions::Permissions;

#[derive(Clone, Debug, Default)]
pub struct ClientConfig {
    pub permissions: Permissions,
    // pub callbacks: Callbacks,
}
