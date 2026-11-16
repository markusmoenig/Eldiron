//! Shared desktop bootstrap for both graphical executable names. Parsing and
//! project loading finish before the host constructs a surface or GPU context.
use super::*;
use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
    time::Instant,
};
#[derive(Debug, PartialEq, Eq)]
pub struct DesktopOptions {
    pub path: PathBuf,
    pub debug: bool,
    pub render_debug: bool,
    pub smoke: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub enum DesktopCommand {
    Help,
    Version,
    Run(DesktopOptions),
}
impl DesktopCommand {
    /// Arguments include the executable name, as with std::env::args_os.
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> std::result::Result<Self, String> {
        let mut path = None;
        let mut debug = false;
        let mut render_debug = false;
        let mut smoke = false;
        let mut literal = false;
        for argument in args.into_iter().skip(1) {
            if !literal {
                match argument.to_str() {
                    Some("--help" | "-h") => return Ok(Self::Help),
                    Some("--version" | "-V") => return Ok(Self::Version),
                    Some("--debug") => {
                        debug = true;
                        continue;
                    }
                    Some("--render-debug") => {
                        render_debug = true;
                        continue;
                    }
                    Some("--smoke") => {
                        smoke = true;
                        continue;
                    }
                    Some("--") => {
                        literal = true;
                        continue;
                    }
                    Some(value) if value.starts_with('-') => {
                        return Err(format!("Unknown option '{value}'; use --help"));
                    }
                    _ => {}
                }
            }
            if path.replace(PathBuf::from(argument)).is_some() {
                return Err("Supply one .eldiron project path".into());
            }
        }
        Ok(Self::Run(DesktopOptions {
            path: path.unwrap_or_else(|| PathBuf::from("game.eldiron")),
            debug,
            render_debug,
            smoke,
        }))
    }
}
struct DesktopApplication {
    player: PlayerApplication,
    presentations: u64,
    deadline: Option<Instant>,
    render_debug: bool,
}
impl GpuApplication for DesktopApplication {
    fn title(&self) -> String {
        self.player.title()
    }
    fn initial_size(&self) -> (u32, u32) {
        self.player.initial_size()
    }
    fn initialize(&mut self, h: &mut HostContext, i: &mut ImageRegistry) -> Result<()> {
        self.player.initialize(h, i)
    }
    fn gpu_recreated(&mut self, h: &mut HostContext, i: &mut ImageRegistry) -> Result<()> {
        self.player.gpu_recreated(h, i)
    }
    fn resize(&mut self, h: &mut HostContext) {
        self.player.resize(h)
    }
    fn native_text_snapshot(
        &mut self,
        h: &mut HostContext,
    ) -> Option<theframework::text_edit::NativeTextSnapshot> {
        self.player.native_text_snapshot(h)
    }
    fn native_text_query(
        &mut self,
        h: &mut HostContext,
        request: &theframework::text_edit::NativeTextRequest,
    ) -> Option<theframework::text_edit::NativeTextAnswer> {
        self.player.native_text_query(h, request)
    }
    fn input(&mut self, e: &InputEvent, h: &mut HostContext) -> InputResponse {
        self.player.input(e, h)
    }
    fn update(&mut self, h: &mut HostContext, elapsed: Duration) -> FrameRequest {
        let request = self.player.update(h, elapsed);
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            h.requests.push(HostRequest::Fail(format!(
                "Player presentation validation timed out: {} successful surface presentations",
                self.presentations
            )));
        }
        request
    }
    fn frame(&mut self, h: &mut HostContext, i: &mut ImageRegistry) -> Result<PaintList> {
        let before = i.context().transfers().readback_bytes;
        let list = self.player.frame(h, i)?;
        if self.deadline.is_some() && i.context().transfers().readback_bytes != before {
            return Err(RenderError(
                "Native player frame performed an implicit readback".into(),
            ));
        }
        if self.render_debug {
            eprintln!(
                "[NativePlayer] prepared frame {}, {} paint operations, {} scene outputs, transfers {:?}",
                self.player.frame_count(),
                list.ops.len(),
                self.player.screen.view_images().len(),
                i.context().transfers()
            );
        }
        Ok(list)
    }
    fn presented(&mut self, h: &mut HostContext) {
        self.presentations += 1;
        self.player.presented(h);
        if self.render_debug || self.deadline.is_some() {
            eprintln!(
                "[NativePlayer] successful surface presentation {}",
                self.presentations
            );
        }
        if self.deadline.is_some() && self.presentations >= 3 {
            h.requests.push(HostRequest::Exit);
        }
    }
    fn suspended(&mut self, h: &mut HostContext) {
        self.player.suspended(h)
    }
    fn resumed(&mut self, h: &mut HostContext) {
        self.player.resumed(h)
    }
}
pub fn run(
    args: impl IntoIterator<Item = OsString>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let options = match DesktopCommand::parse(args)? {
        DesktopCommand::Help => {
            println!(
                "Eldiron Player\nUsage: eldiron-client [OPTIONS] [PROJECT.eldiron]\n\nDefaults to ./game.eldiron.\n  --debug         Run authored debug behavior sources\n  --render-debug  Report preparation/transfers and surface presentations\n  --smoke         Present three frames and exit, or fail after 10 seconds\n  --help          Show this help\n  --version       Show the version\n  --              Treat the following argument as a literal project path"
            );
            return Ok(());
        }
        DesktopCommand::Version => {
            println!("Eldiron Player {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        DesktopCommand::Run(options) => options,
    };
    let render_debug = options.render_debug
        || std::env::var_os("ELDIRON_RENDER_DEBUG").is_some_and(|v| v != OsStr::new("0"));
    if options.render_debug {
        // CLI bootstrap runs before constructing the session, worker pools or
        // GPU host; preserve the engine's existing diagnostics switch.
        unsafe {
            std::env::set_var("ELDIRON_RENDER_DEBUG", "1");
        }
    }
    let bytes = std::fs::read(&options.path)
        .map_err(|e| format!("Cannot read {}: {e}", options.path.display()))?;
    let mut project = shared::project_io::decode_project(&bytes)
        .map_err(|e| format!("Cannot load {}: {e}", options.path.display()))?;
    project.migrate_default_ruleset();
    let _ = project.sync_ruleset_items();
    let player = PlayerApplication::new(
        project,
        options.debug,
        Box::new(AudioServices::new(StandardServices)),
    )?;
    theframework::platform::winit::run(Box::new(DesktopApplication {
        player,
        presentations: 0,
        deadline: options
            .smoke
            .then(|| Instant::now() + Duration::from_secs(10)),
        render_debug,
    }))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> std::result::Result<DesktopCommand, String> {
        DesktopCommand::parse(args.iter().map(OsString::from))
    }
    #[test]
    fn desktop_arguments_preserve_paths_flags_and_default_without_loading_a_session() {
        assert_eq!(
            parse(&["player"]).unwrap(),
            DesktopCommand::Run(DesktopOptions {
                path: "game.eldiron".into(),
                debug: false,
                render_debug: false,
                smoke: false
            })
        );
        assert_eq!(
            parse(&[
                "player",
                "Some Game 世界.eldiron",
                "--debug",
                "--smoke",
                "--render-debug"
            ])
            .unwrap(),
            DesktopCommand::Run(DesktopOptions {
                path: "Some Game 世界.eldiron".into(),
                debug: true,
                render_debug: true,
                smoke: true
            })
        );
        assert_eq!(parse(&["player", "--help"]).unwrap(), DesktopCommand::Help);
        assert_eq!(parse(&["player", "-V"]).unwrap(), DesktopCommand::Version);
        assert!(parse(&["player", "one", "two"]).is_err());
        assert!(parse(&["player", "--unknown"]).is_err());
        assert_eq!(
            parse(&["player", "--", "--debug"]).unwrap(),
            DesktopCommand::Run(DesktopOptions {
                path: "--debug".into(),
                debug: false,
                render_debug: false,
                smoke: false
            })
        );
    }
    #[cfg(unix)]
    #[test]
    fn desktop_project_paths_keep_non_utf8_bytes() {
        use std::os::unix::ffi::OsStringExt;
        let path = OsString::from_vec(vec![b'p', 0xff, b'.', b'e']);
        let DesktopCommand::Run(options) =
            DesktopCommand::parse([OsString::from("player"), path.clone()]).unwrap()
        else {
            panic!("run")
        };
        assert_eq!(options.path, PathBuf::from(path));
    }
}
