#[derive(Debug, Clone)]
pub struct BufferConfig {
    pub auto_save: bool,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self { auto_save: false }
    }
}
