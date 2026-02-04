use crate::prelude::*;

/// TheApp class handles running an application on the current backend.
pub struct TheApp {
    pub args: Option<Vec<String>>,
}

impl Default for TheApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TheApp {
    pub fn new() -> Self {
        Self { args: None }
    }

    /// Optionally set the command line arguments of the app.
    pub fn set_cmd_line_args(&mut self, args: Vec<String>) {
        self.args = Some(args);
    }

    /// Runs the app
    pub fn run(self, app: Box<dyn crate::TheTrait>) {
        #[cfg(feature = "log")]
        setup_logger();

        #[cfg(feature = "winit_app")]
        run_winit_app(self.args, app);
    }
}
