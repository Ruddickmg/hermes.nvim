mod file;
mod message;
mod notification;

pub use file::*;
pub use message::*;
pub use notification::*;

use tracing_subscriber::fmt::writer::MakeWriter;
use tracing::Metadata;

use crate::utilities::LogLevel;

#[derive(Clone)]
pub struct LevelFilterWriter<W> {
    inner: W,
    level: tracing::Level,
}

impl<W> LevelFilterWriter<W> {
    pub fn new(inner: W, level: tracing::Level) -> Self {
        Self {
            inner,
            level
        }
    }
}

impl<'a, W> MakeWriter<'a> for LevelFilterWriter<W>
where
    W: MakeWriter<'a> + Clone,
{
    type Writer = W::Writer;

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
        if *meta.level() <= self.level {
            self.inner.make_writer()
        } else {
            std::io::sink()
        }
    }

    fn make_writer(&'a self) -> Self::Writer {
        self.inner.make_writer()
    }
}

pub trait Filtered {
    /// Wrap this writer with a LevelFilterWriter
    fn filtered(self, level: LogLevel) -> LevelFilterWriter<impl Fn() -> Self>
    where
        Self: Sized + Clone + 'static,
    {
        LevelFilterWriter::new(move || self.clone(), level.into())
    }
}
