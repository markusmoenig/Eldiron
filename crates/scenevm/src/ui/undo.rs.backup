use crate::SceneVM;

/// Trait for undoable/redoable commands
///
/// Implement this trait for your application-specific commands.
/// The generic parameter `T` is your application context (e.g., your main app struct).
///
/// # Example
/// ```ignore
/// #[derive(Debug)]
/// struct MyCommand { /* ... */ }
///
/// impl UndoCommand<MyApp> for MyCommand {
///     fn execute(&mut self, vm: &mut SceneVM, context: &mut MyApp, is_new: bool) {
///         // Modify your app state and VM
///     }
///
///     fn undo(&mut self, vm: &mut SceneVM, context: &mut MyApp) {
///         // Restore previous state
///     }
///
///     fn description(&self) -> &str {
///         "My Custom Command"
///     }
/// }
/// ```
pub trait UndoCommand<T>: std::fmt::Debug {
    /// Execute the command
    /// `is_new` is true when the command is first executed, false when redoing
    /// This prevents re-applying UI actions that were just performed
    fn execute(&mut self, vm: &mut SceneVM, context: &mut T, is_new: bool);

    /// Reverse the command (for undo)
    fn undo(&mut self, vm: &mut SceneVM, context: &mut T);

    /// Optional: merge with next command if they're related (e.g., consecutive slider drags)
    /// Returns true if merge was successful
    fn try_merge(&mut self, _other: &dyn UndoCommand<T>) -> bool {
        false
    }

    /// Command description for UI display (e.g., "Change Slider" or "Select Tool")
    fn description(&self) -> &str;

    /// Helper for downcasting in try_merge
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Undo/Redo stack manager
///
/// Generic over the application context type `T`.
/// Use this to manage undo/redo functionality in your application.
///
/// # Example
/// ```ignore
/// let mut undo_stack = UndoStack::<MyApp>::new(100);
///
/// // Execute a command
/// let cmd = Box::new(MyCommand::new(/* ... */));
/// undo_stack.execute(cmd, &mut my_app);
///
/// // Undo
/// undo_stack.undo(&mut my_app);
///
/// // Redo
/// undo_stack.redo(&mut my_app);
/// ```
pub struct UndoStack<T> {
    commands: Vec<Box<dyn UndoCommand<T>>>,
    current_index: usize, // Points to the next command to redo
    max_size: usize,
    dirty: bool,
}

impl<T> UndoStack<T> {
    /// Create a new undo stack with a maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            commands: Vec::new(),
            current_index: 0,
            max_size,
            dirty: false,
        }
    }

    /// Add a new command and execute it
    pub fn execute(&mut self, mut cmd: Box<dyn UndoCommand<T>>, vm: &mut SceneVM, context: &mut T) {
        // Truncate any commands after current position (user did undo then new action)
        self.commands.truncate(self.current_index);

        // Try to merge with previous command (e.g., consecutive slider drags)
        if let Some(last) = self.commands.last_mut() {
            if last.try_merge(cmd.as_ref()) {
                self.dirty = true;
                return;
            }
        }

        // Execute the command (is_new = true, don't re-apply the UI action)
        cmd.execute(vm, context, true);

        // Add to stack
        self.commands.push(cmd);
        self.current_index += 1;
        self.dirty = true;

        // Enforce max size
        if self.commands.len() > self.max_size {
            self.commands.remove(0);
            self.current_index = self.current_index.saturating_sub(1);
        }
    }

    /// Undo the last command
    pub fn undo(&mut self, vm: &mut SceneVM, context: &mut T) -> bool {
        if self.current_index == 0 {
            return false;
        }

        self.current_index -= 1;
        self.commands[self.current_index].undo(vm, context);
        self.dirty = true;
        true
    }

    /// Redo the next command
    pub fn redo(&mut self, vm: &mut SceneVM, context: &mut T) -> bool {
        if self.current_index >= self.commands.len() {
            return false;
        }

        // is_new = false for redo (apply the UI action)
        self.commands[self.current_index].execute(vm, context, false);
        self.current_index += 1;
        self.dirty = true;
        true
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.current_index < self.commands.len()
    }

    /// Clear the entire undo stack
    pub fn clear(&mut self) {
        self.commands.clear();
        self.current_index = 0;
        self.dirty = false;
    }

    /// Get description of next undo action
    pub fn undo_description(&self) -> Option<&str> {
        if self.can_undo() {
            Some(self.commands[self.current_index - 1].description())
        } else {
            None
        }
    }

    /// Get description of next redo action
    pub fn redo_description(&self) -> Option<&str> {
        if self.can_redo() {
            Some(self.commands[self.current_index].description())
        } else {
            None
        }
    }

    /// Check if stack has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark stack as saved (clear dirty flag)
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Get number of commands in stack
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
