use nvim_oxi::api;
use std::io::{self, Write};
use tracing_subscriber::fmt::writer::MakeWriter;

#[derive(Debug, Clone, Default)]
pub struct MessageWriter;

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

impl<'a> MakeWriter<'a> for MessageWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
