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
        crate::thelogger::setup_logger();

        #[cfg(any(feature = "winit_app", feature = "winit_app_softbuffer"))]
        crate::thewinitapp::run_winit_app(self.args, app);

        #[cfg(not(any(feature = "winit_app", feature = "winit_app_softbuffer")))]
        let _ = app;
    }
}
