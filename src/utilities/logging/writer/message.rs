use std::io::{self, Write};
use tracing_subscriber::fmt::writer::MakeWriter;

use crate::utilities::MessageMessenger;

#[derive(Debug, Clone)]
pub struct MessageWriter {
    messenger: MessageMessenger,
}

unsafe impl Send for MessageWriter {}
unsafe impl Sync for MessageWriter {}

impl MessageWriter {
    pub fn new(messenger: MessageMessenger) -> Self {
        Self { messenger }
    }
}

impl Write for MessageWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);

        if s.trim().is_empty() {
            return Ok(buf.len());
        }

        self.messenger.send(s.to_string()).ok();

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for MessageWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_writer_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<MessageWriter>();
    }

    #[test]
    fn test_message_writer_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<MessageWriter>();
    }
}
