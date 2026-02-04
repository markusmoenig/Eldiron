use crate::SceneVM;

/// Trait for undoable/redoable commands
///
/// Implement this trait for your application-specific commands.
/// The generic parameter `T` is your main application context.
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
    fn execute(&mut self, vm: &mut SceneVM, context: &mut T, is_new: bool);
    fn undo(&mut self, vm: &mut SceneVM, context: &mut T);
    fn description(&self) -> &str;
}

/// Undo/Redo stack manager
pub struct UndoStack<T> {
    commands: Vec<Box<dyn UndoCommand<T>>>,
    current_index: usize, // Points to the next command to redo
    max_size: usize,
    saved_index: Option<usize>,
}

impl<T> UndoStack<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            commands: Vec::new(),
            current_index: 0,
            max_size,
            saved_index: Some(0),
        }
    }

    pub fn execute(&mut self, mut cmd: Box<dyn UndoCommand<T>>, vm: &mut SceneVM, context: &mut T) {
        self.commands.truncate(self.current_index);
        if let Some(saved) = self.saved_index {
            if saved > self.current_index {
                self.saved_index = None;
            }
        }

        cmd.execute(vm, context, true);
        self.commands.push(cmd);
        self.current_index += 1;
        self.validate_saved_index();

        if self.commands.len() > self.max_size {
            self.commands.remove(0);
            self.current_index = self.current_index.saturating_sub(1);
            if let Some(saved) = self.saved_index {
                if saved > 0 {
                    self.saved_index = Some(saved.saturating_sub(1));
                } else {
                    self.saved_index = None;
                }
            }
        }
    }

    pub fn undo(&mut self, vm: &mut SceneVM, context: &mut T) -> bool {
        if self.current_index == 0 {
            return false;
        }

        self.current_index -= 1;
        self.commands[self.current_index].undo(vm, context);
        self.validate_saved_index();
        true
    }

    pub fn redo(&mut self, vm: &mut SceneVM, context: &mut T) -> bool {
        if self.current_index >= self.commands.len() {
            return false;
        }

        self.commands[self.current_index].execute(vm, context, false);
        self.current_index += 1;
        self.validate_saved_index();
        true
    }

    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    pub fn can_redo(&self) -> bool {
        self.current_index < self.commands.len()
    }

    pub fn clear(&mut self) {
        self.commands.clear();
        self.current_index = 0;
        self.saved_index = Some(0);
    }

    pub fn undo_description(&self) -> Option<&str> {
        if self.can_undo() {
            Some(self.commands[self.current_index - 1].description())
        } else {
            None
        }
    }

    pub fn redo_description(&self) -> Option<&str> {
        if self.can_redo() {
            Some(self.commands[self.current_index].description())
        } else {
            None
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self.saved_index {
            Some(saved) => saved != self.current_index,
            None => true,
        }
    }

    pub fn mark_saved(&mut self) {
        self.saved_index = Some(self.current_index);
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    fn validate_saved_index(&mut self) {
        if let Some(saved) = self.saved_index {
            if saved > self.commands.len() {
                self.saved_index = None;
            }
        }
    }
}
