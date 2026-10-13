//! Undo/redo history system with command pattern

use std::fmt;
use std::sync::{Arc, Mutex};

use plugin_editor_api::undo::lock;
use plugin_editor_api::SharedHistory;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Command undo failed: {0}")]
    UndoFailed(String),
}

pub type CommandResult<T> = Result<T, CommandError>;

/// Command trait for undo/redo operations
/// 
/// All actions that modify the document must implement this trait.
/// Commands must be self-contained and store both before and after state.
pub trait Command: Send + Sync + fmt::Debug {
    /// Execute the command (apply changes)
    fn execute(&mut self) -> CommandResult<()>;
    
    /// Undo the command (revert changes)
    fn undo(&mut self) -> CommandResult<()>;
    
    /// Redo is typically the same as execute
    fn redo(&mut self) -> CommandResult<()> {
        self.execute()
    }
    
    /// Get a human-readable description of this command
    fn description(&self) -> &str;
}

/// One history entry: a command already applied to the document.
#[derive(Debug)]
pub struct MatterEdit(Box<dyn Command>);

impl plugin_editor_api::UndoEntry for MatterEdit {
    fn label(&self) -> String {
        self.0.description().to_string()
    }
}

/// The document's undo history: the host's shared history type
/// (`plugin_editor_api::undo`, Pulsar-Native#931), registered with the
/// host's undo service by [`Self::register`] so the Edit menu and the
/// shared Undo/Redo actions reach it.
pub struct History {
    shared: SharedHistory<MatterEdit>,
}

impl History {
    /// Create a new history manager
    pub fn new(max_size: usize) -> Self {
        Self {
            shared: Arc::new(Mutex::new(plugin_editor_api::History::with_capacity(max_size))),
        }
    }

    /// The history as the host and the Edit menu see it.
    pub fn shared(&self) -> &SharedHistory<MatterEdit> {
        &self.shared
    }

    /// Register this history with the host's undo service under the
    /// document at `path`.
    pub fn register(&self, service: &plugin_editor_api::UndoService, path: &std::path::Path) {
        service.adopt(&plugin_editor_api::DocumentId::for_path(path), &self.shared);
    }

    /// Execute a command and add it to history
    pub fn execute(&mut self, mut command: Box<dyn Command>) -> CommandResult<()> {
        command.execute()?;
        self.push_executed(command);
        Ok(())
    }

    /// Undo the last command. A command that fails to undo is dropped
    /// (with the redo stack), as the shared history does.
    pub fn undo(&mut self) -> CommandResult<()> {
        let mut failed = None;
        let undone = lock(&self.shared).undo(|edit| match edit.0.undo() {
            Ok(()) => true,
            Err(error) => {
                failed = Some(error);
                false
            }
        });
        match (undone, failed) {
            (Some(_), _) => Ok(()),
            (None, Some(error)) => Err(error),
            (None, None) => Err(CommandError::UndoFailed("Nothing to undo".to_string())),
        }
    }

    /// Redo the last undone command
    pub fn redo(&mut self) -> CommandResult<()> {
        let mut failed = None;
        let redone = lock(&self.shared).redo(|edit| match edit.0.redo() {
            Ok(()) => true,
            Err(error) => {
                failed = Some(error);
                false
            }
        });
        match (redone, failed) {
            (Some(_), _) => Ok(()),
            (None, Some(error)) => Err(error),
            (None, None) => Err(CommandError::ExecutionFailed("Nothing to redo".to_string())),
        }
    }

    /// Check if undo stack is empty
    pub fn is_empty_undo(&self) -> bool {
        !self.can_undo()
    }

    /// Check if redo stack is empty
    pub fn is_empty_redo(&self) -> bool {
        !self.can_redo()
    }

    /// Check if there are commands to undo
    pub fn can_undo(&self) -> bool {
        lock(&self.shared).can_undo()
    }

    /// Check if there are commands to redo
    pub fn can_redo(&self) -> bool {
        lock(&self.shared).can_redo()
    }

    /// Get the description of the next undo command
    pub fn undo_description(&self) -> Option<String> {
        lock(&self.shared).undo_label()
    }

    /// Get the description of the next redo command
    pub fn redo_description(&self) -> Option<String> {
        lock(&self.shared).redo_label()
    }

    /// Push a command that has *already been applied* to the document.
    /// Skips calling `execute()`; only registers it for undo.
    pub fn push_executed(&mut self, command: Box<dyn Command>) {
        let mut history = lock(&self.shared);
        history.push(MatterEdit(command));
        history.seal();
    }

    /// Clear all history
    pub fn clear(&mut self) {
        lock(&self.shared).clear();
    }

    /// Get the number of commands in undo stack
    pub fn undo_count(&self) -> usize {
        lock(&self.shared).len()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new(50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug)]
    struct TestCommand {
        value: i32,
        executed: bool,
    }
    
    impl TestCommand {
        fn new(value: i32) -> Self {
            Self { value, executed: false }
        }
    }
    
    impl Command for TestCommand {
        fn execute(&mut self) -> CommandResult<()> {
            self.executed = true;
            Ok(())
        }
        
        fn undo(&mut self) -> CommandResult<()> {
            self.executed = false;
            Ok(())
        }
        
        fn description(&self) -> &str {
            "Test Command"
        }
    }
    
    #[test]
    fn test_execute_and_undo() {
        let mut history = History::new(10);
        let cmd = Box::new(TestCommand::new(42));
        
        history.execute(cmd).unwrap();
        assert!(history.can_undo());
        assert!(!history.can_redo());
        
        history.undo().unwrap();
        assert!(!history.can_undo());
        assert!(history.can_redo());
    }
    
    #[test]
    fn test_redo() {
        let mut history = History::new(10);
        history.execute(Box::new(TestCommand::new(1))).unwrap();
        history.undo().unwrap();
        
        assert!(history.can_redo());
        history.redo().unwrap();
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }
    
    #[test]
    fn test_max_size() {
        let mut history = History::new(3);
        
        for i in 0..5 {
            history.execute(Box::new(TestCommand::new(i))).unwrap();
        }
        
        assert_eq!(history.undo_count(), 3);
    }
}
