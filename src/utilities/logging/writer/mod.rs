mod file;
mod message;
mod notification;

pub use file::*;
pub use message::*;
pub use notification::*;

use std::io::{self, Write};
use tracing::Metadata;
use tracing_subscriber::fmt::writer::MakeWriter;

use crate::utilities::LogLevel;

/// A writer that conditionally discards output based on log level
pub struct MaybeWriter<W> {
    inner: Option<W>,
}

impl<W: Write> Write for MaybeWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &mut self.inner {
            Some(w) => w.write(buf),
            None => Ok(buf.len()), // Pretend success, discard data
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &mut self.inner {
            Some(w) => w.flush(),
            None => Ok(()),
        }
    }
}

/// A wrapper that filters writes based on log level
///
/// This wraps any writer that implements Clone + Write, and implements MakeWriter
/// by cloning the inner writer for each write operation.
#[derive(Clone)]
pub struct LevelFilterWriter<W> {
    inner: W,
    level: tracing::Level,
}

impl<W> LevelFilterWriter<W> {
    pub fn new(inner: W, level: tracing::Level) -> Self {
        Self { inner, level }
    }
}

impl<W> Write for LevelFilterWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W> MakeWriter<'_> for LevelFilterWriter<W>
where
    W: Write + Clone + 'static,
{
    type Writer = MaybeWriter<Self>;

    fn make_writer_for(&self, meta: &Metadata<'_>) -> Self::Writer {
        if *meta.level() <= self.level {
            MaybeWriter {
                inner: Some(self.clone()),
            }
        } else {
            MaybeWriter { inner: None }
        }
    }

    fn make_writer(&self) -> Self::Writer {
        MaybeWriter {
            inner: Some(self.clone()),
        }
    }
}

/// Trait to add filtering capability to writers
///
/// This trait is implemented for types that implement both Write and Clone.
/// It wraps them in a LevelFilterWriter that filters based on log level.
pub trait Filtered: Clone + Write + 'static {
    /// Wrap this writer with a LevelFilterWriter
    fn filtered(self, level: LogLevel) -> LevelFilterWriter<Self> {
        LevelFilterWriter::new(self, level.into())
    }
}

// Implement Filtered for any type that is Clone + Write + 'static
impl<T> Filtered for T where T: Clone + Write + 'static {}
