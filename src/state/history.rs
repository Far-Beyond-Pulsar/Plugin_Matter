//! Undo/redo history system with command pattern

use std::fmt;
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

/// History manager with undo/redo stacks
pub struct History {
    /// Stack of executed commands that can be undone
    undo_stack: Vec<Box<dyn Command>>,
    
    /// Stack of undone commands that can be redone
    redo_stack: Vec<Box<dyn Command>>,
    
    /// Maximum number of commands to keep in history
    max_size: usize,
}

impl History {
    /// Create a new history manager
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size,
        }
    }
    
    /// Execute a command and add it to history
    pub fn execute(&mut self, mut command: Box<dyn Command>) -> CommandResult<()> {
        command.execute()?;
        
        // Clear redo stack when new command is executed
        self.redo_stack.clear();
        
        // Add to undo stack
        self.undo_stack.push(command);
        
        // Limit stack size
        if self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
        
        Ok(())
    }
    
    /// Undo the last command
    pub fn undo(&mut self) -> CommandResult<()> {
        if let Some(mut command) = self.undo_stack.pop() {
            command.undo()?;
            self.redo_stack.push(command);
            Ok(())
        } else {
            Err(CommandError::UndoFailed("Nothing to undo".to_string()))
        }
    }
    
    /// Redo the last undone command
    pub fn redo(&mut self) -> CommandResult<()> {
        if let Some(mut command) = self.redo_stack.pop() {
            command.redo()?;
            self.undo_stack.push(command);
            Ok(())
        } else {
            Err(CommandError::ExecutionFailed("Nothing to redo".to_string()))
        }
    }
    
    /// Check if undo stack is empty
    pub fn is_empty_undo(&self) -> bool {
        self.undo_stack.is_empty()
    }
    
    /// Check if redo stack is empty
    pub fn is_empty_redo(&self) -> bool {
        self.redo_stack.is_empty()
    }
    
    /// Check if there are commands to undo
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }
    
    /// Check if there are commands to redo
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
    
    /// Get the description of the next undo command
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|cmd| cmd.description())
    }
    
    /// Get the description of the next redo command
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|cmd| cmd.description())
    }
    
    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
    
    /// Get the number of commands in undo stack
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }
    
    /// Get the number of commands in redo stack
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
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
