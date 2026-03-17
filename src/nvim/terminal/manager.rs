use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tokio::sync::oneshot;

use crate::acp::Result;
use crate::acp::error::Error;
use crate::nvim::terminal::Terminal;

/// Manages all terminal (job) instances for a session
#[derive(Debug, Clone)]
pub struct TerminalManager<T: Terminal + Clone> {
    terminals: Rc<RefCell<HashMap<String, T>>>,
}

impl<T: Terminal + Clone> TerminalManager<T> {
    /// Create a new TerminalManager
    pub fn new() -> Self {
        Self {
            terminals: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn intitialize_terminal(&mut self, id: String, info: T) {
        self.terminals.borrow_mut().insert(id, info);
    }

    pub fn get_terminal(&self, id: &str) -> Option<T> {
        self.terminals.borrow().get(id).cloned()
    }

    pub fn get_output(&self, id: &str) -> Option<String> {
        self.terminals.borrow().get(id).map(|info| info.content())
    }

    pub fn notify_when_finished(
        &self,
        id: &str,
        sender: oneshot::Sender<(u32, String)>,
    ) -> Result<()> {
        self.terminals
            .borrow_mut()
            .get(id)
            .map(|terminal| terminal.report_exit_to(sender))
            .transpose()
            .map(|_| ())
    }

    pub fn release(&mut self, id: &str) -> Result<()> {
        let mut terminals = self.terminals.borrow_mut();
        let terminal = terminals.remove(id);
        drop(terminals);
        if let Some(t) = terminal {
            t.close()
        } else {
            Err(Error::Internal(format!(
                "Terminal with id '{}' was not present when release was called",
                id
            )))
        }
    }
}

impl<T: Terminal + Clone> Default for TerminalManager<T> {
    fn default() -> Self {
        Self::new()
    }
}
