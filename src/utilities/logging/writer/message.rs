use nvim_oxi::api;
use std::io::{self, Write};

#[derive(Debug, Clone, Default)]
pub struct MessageWriter {}

impl Write for MessageWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Convert bytes to string (ignore invalid UTF-8)
        let s = String::from_utf8_lossy(buf);

        // Send the entire content as a single :messages entry
        // echomsg can handle newlines, so we preserve them
        let escaped = s.replace('"', "\\\"");
        let cmd = format!("echomsg \"{}\"", escaped);
        let _ = api::command(&cmd)
            .inspect_err(|e| eprint!("Error trying to route logs to \":messages\": {:?}", e));

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
