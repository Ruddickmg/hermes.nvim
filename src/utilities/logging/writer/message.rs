use std::io::{self, Write};
use nvim_oxi::api;
use crate::utilities::{writer::{Filtered}};

#[derive(Debug, Clone, Default)]
pub struct MessageWriter;

impl Filtered for MessageWriter {}

impl Write for MessageWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = String::from_utf8_lossy(buf);
        let escaped = s.replace('"', "\\\"");
        let cmd = format!("echomsg \"{}\"", escaped);
        api::command(&cmd).map_err(|e| {
            std::io::Error::other(format!("Error sending log to \":messages\": {:?}", e))
        })?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
