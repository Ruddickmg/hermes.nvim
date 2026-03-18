use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::result;
use tokio::sync::oneshot;

use crate::acp::error::Error;
use crate::acp::Result;
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
        sender: oneshot::Sender<Result<(Option<u32>, Option<String>)>>,
    ) -> Result<()> {
        let terminals = self.terminals.borrow();
        let terminal = terminals.get(id);
        if let Some(t) = terminal {
            t.report_exit_to(sender)
        } else {
            Err(Error::Internal(format!(
                "Terminal with id '{}' not found",
                id
            )))
        }
    }

    pub fn kill(&self, id: &str) -> Result<()> {
        let terminals = self.terminals.borrow();
        let terminal = terminals.get(id);
        let result = if let Some(t) = terminal {
            t.stop()
        } else {
            Err(Error::InvalidInput(format!(
                "Terminal with id '{}' was not present when release was called",
                id
            )))
        };
        drop(terminals);
        result
    }

    pub fn release(&self, id: &str) -> Result<()> {
        let mut terminals = self.terminals.borrow_mut();
        let terminal = terminals.remove(id);
        drop(terminals);
        if let Some(t) = terminal {
            t.stop()
        } else {
            Err(Error::InvalidInput(format!(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nvim::terminal::ExitStatus;
    use std::cell::RefCell;
    use std::rc::Rc;
    use uuid::Uuid;

    /// Mock Terminal for testing
    #[derive(Debug, Clone)]
    struct MockTerminal {
        id: Uuid,
        content: String,
        exit_sender: Rc<RefCell<Option<oneshot::Sender<Result<ExitStatus>>>>>,
        killed: Rc<RefCell<bool>>,
    }

    impl MockTerminal {
        fn new(id: &str, content: &str) -> Self {
            Self {
                id: Uuid::parse_str(id).unwrap_or_else(|_| Uuid::new_v4()),
                content: content.to_string(),
                exit_sender: Rc::new(RefCell::new(None)),
                killed: Rc::new(RefCell::new(false)),
            }
        }
    }

    impl Terminal for MockTerminal {
        fn id(&self) -> Uuid {
            self.id
        }

        fn truncated(&self) -> bool {
            false
        }

        fn content(&self) -> String {
            self.content.clone()
        }

        fn report_exit_to(
            &self,
            sender: oneshot::Sender<Result<(Option<u32>, Option<String>)>>,
        ) -> Result<()> {
            *self.exit_sender.borrow_mut() = Some(sender);
            Ok(())
        }

        fn stop(&self) -> Result<()> {
            *self.killed.borrow_mut() = true;
            Ok(())
        }

        fn run(&mut self, _command: String, _args: Vec<String>) -> Result<i64> {
            Ok(1) // Return a mock job ID
        }

        fn from_request(_data: agent_client_protocol::CreateTerminalRequest) -> Self {
            Self::new("550e8400-e29b-41d4-a716-446655440000", "")
        }
    }

    #[test]
    fn terminal_manager_new_creates_empty_manager() {
        let manager: TerminalManager<MockTerminal> = TerminalManager::new();
        assert!(manager.get_terminal("any-id").is_none());
    }

    #[test]
    fn terminal_manager_intitialize_terminal_adds_terminal() {
        let mut manager = TerminalManager::new();
        let terminal = MockTerminal::new("550e8400-e29b-41d4-a716-446655440000", "initial output");
        let terminal_id = terminal.id;
        manager.intitialize_terminal("term-1".to_string(), terminal);

        let retrieved = manager.get_terminal("term-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, terminal_id);
    }

    #[test]
    fn terminal_manager_get_terminal_returns_none_for_missing_id() {
        let manager: TerminalManager<MockTerminal> = TerminalManager::new();
        let result = manager.get_terminal("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn terminal_manager_get_output_returns_terminal_content() {
        let mut manager = TerminalManager::new();
        let terminal = MockTerminal::new("term-1", "test content");
        manager.intitialize_terminal("term-1".to_string(), terminal);

        let output = manager.get_output("term-1");
        assert_eq!(output, Some("test content".to_string()));
    }

    #[test]
    fn terminal_manager_get_output_returns_none_for_missing_terminal() {
        let manager: TerminalManager<MockTerminal> = TerminalManager::new();
        let output = manager.get_output("nonexistent");
        assert!(output.is_none());
    }

    #[test]
    fn terminal_manager_notify_when_finished_registers_sender() {
        let mut manager = TerminalManager::new();
        let terminal = MockTerminal::new("term-1", "");
        manager.intitialize_terminal("term-1".to_string(), terminal.clone());

        let (sender, _receiver) = oneshot::channel();
        let result = manager.notify_when_finished("term-1", sender);
        assert!(result.is_ok());

        // Verify sender was registered
        let sender_opt = terminal.exit_sender.borrow();
        assert!(sender_opt.is_some());
    }

    #[test]
    fn terminal_manager_notify_when_finished_fails_for_missing_terminal() {
        let manager: TerminalManager<MockTerminal> = TerminalManager::new();
        let (sender, _receiver) = oneshot::channel();
        let result = manager.notify_when_finished("nonexistent", sender);
        assert!(result.is_err());
    }

    #[test]
    fn terminal_manager_release_removes_and_closes_terminal() {
        let mut manager = TerminalManager::new();
        let terminal = MockTerminal::new("term-1", "");
        manager.intitialize_terminal("term-1".to_string(), terminal.clone());

        let result = manager.release("term-1");
        assert!(result.is_ok());

        // Verify terminal was removed
        assert!(manager.get_terminal("term-1").is_none());

        // Verify close was called
        assert!(*terminal.killed.borrow());
    }

    #[test]
    fn terminal_manager_release_fails_for_missing_terminal() {
        let mut manager: TerminalManager<MockTerminal> = TerminalManager::new();
        let result = manager.release("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn terminal_manager_default_creates_empty_manager() {
        let manager: TerminalManager<MockTerminal> = TerminalManager::default();
        assert!(manager.get_terminal("any-id").is_none());
    }
}
