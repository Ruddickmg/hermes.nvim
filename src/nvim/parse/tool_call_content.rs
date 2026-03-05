use crate::nvim::parse;
use agent_client_protocol::{Error, Result, ToolCallContent};
use nvim_oxi::Dictionary;
use std::fs;

pub fn parse_tool_call_content(content: ToolCallContent) -> Result<Dictionary> {
    match content {
        ToolCallContent::Content(container) => {
            Ok(Dictionary::new())
        }
        ToolCallContent::Terminal(terminal) => {
            let mut dict = Dictionary::new();
            dict.insert("id", terminal.terminal_id.to_string());
            dict.insert("type", "terminal");
            Ok(dict)
        }
        ToolCallContent::Diff(diff) => fs::read_to_string(diff.path.clone())
            .map_err(Error::into_internal_error)
            .map(|path| {
                let mut dict = Dictionary::new();
                dict.insert("type", "diff");
                dict.insert("path", path);
                dict.insert("new_text", diff.new_text.clone());
                if let Some(old_text) = diff.old_text.clone() {
                    dict.insert("old_text", old_text);
                }
                dict
            }),
        _ => Err(Error::method_not_found()),
    }
}
