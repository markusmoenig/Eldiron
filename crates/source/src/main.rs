use clap::{Parser, Subcommand};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(
    name = "eldiron-source",
    version,
    about = "Source-first compiler and project tool for Eldiron games.",
    long_about = "Eldiron Source compiles eldiron.toml plus .els source files into regular .eldiron projects. It can scaffold source projects, build them, play them through the configured client, and watch source folders for live rebuilds.",
    after_help = "Examples:\n  eldiron-source new my-game\n  eldiron-source build my-game\n  eldiron-source play my-game\n  eldiron-source watch my-game\n\nRun `eldiron-source help <command>` for command-specific help."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Eldiron Source project folder.
    New {
        /// Folder to create for the source project.
        project_dir: PathBuf,

        /// Human-readable project name written to eldiron.toml.
        #[arg(long)]
        name: Option<String>,

        /// Allow scaffolding into an existing directory without overwriting files.
        #[arg(long)]
        force: bool,
    },

    /// Compile eldiron.toml and source files into a .eldiron file.
    Build {
        /// Project folder containing eldiron.toml.
        #[arg(default_value = ".")]
        project_dir: PathBuf,
    },

    /// Build, then play the generated .eldiron with the configured client.
    Play {
        /// Project folder containing eldiron.toml.
        #[arg(default_value = ".")]
        project_dir: PathBuf,
    },

    /// Watch source files and assets, rebuilding whenever they change.
    Watch {
        /// Project folder containing eldiron.toml.
        #[arg(default_value = ".")]
        project_dir: PathBuf,

        /// Minimum delay after a change before rebuilding.
        #[arg(long, default_value_t = 250)]
        debounce_ms: u64,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("eldiron-source: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Build {
        project_dir: PathBuf::from("."),
    }) {
        Commands::New {
            project_dir,
            name,
            force,
        } => scaffold_project(&project_dir, name, force),
        Commands::Build { project_dir } => build_once(&project_dir),
        Commands::Play { project_dir } => play_project(&project_dir),
        Commands::Watch {
            project_dir,
            debounce_ms,
        } => watch_project(&project_dir, Duration::from_millis(debounce_ms)),
    }
}

fn build_once(project_dir: &Path) -> Result<(), String> {
    let output = eldiron_source::build_project(project_dir)?;
    println!("Wrote {}", output.display());
    Ok(())
}

fn play_project(project_dir: &Path) -> Result<(), String> {
    let client_mode = source_client_mode(project_dir)?;
    let output = eldiron_source::build_project(project_dir)?;
    println!("Wrote {}", output.display());
    if client_mode_is_graphical(&client_mode) {
        run_graphical_client(&output)
    } else {
        run_terminal_client(&output)
    }
}

fn scaffold_project(project_dir: &Path, name: Option<String>, force: bool) -> Result<(), String> {
    if project_dir.exists() {
        if !project_dir.is_dir() {
            return Err(format!(
                "{} exists but is not a directory",
                project_dir.display()
            ));
        }
        if !force
            && project_dir
                .read_dir()
                .map_err(format_io(project_dir))?
                .next()
                .is_some()
        {
            return Err(format!(
                "{} already exists and is not empty. Use --force to scaffold anyway.",
                project_dir.display()
            ));
        }
    }

    fs::create_dir_all(project_dir).map_err(format_io(project_dir))?;
    for dir in [
        "assets",
        "characters",
        "items",
        "regions",
        "scripts",
        "tiles",
        "build",
    ] {
        let path = project_dir.join(dir);
        fs::create_dir_all(&path).map_err(format_io(&path))?;
        write_new_file(&path.join(".gitkeep"), "")?;
    }

    let project_name = name.unwrap_or_else(|| project_name_from_dir(project_dir));
    write_new_file(
        &project_dir.join("eldiron.toml"),
        &format_eldiron_toml(&project_name),
    )?;
    write_new_file(&project_dir.join("main.els"), STARTER_MAIN_ELS)?;
    write_new_file(
        &project_dir.join("characters/player.els"),
        STARTER_PLAYER_ELS,
    )?;

    println!("Created {}", project_dir.display());
    println!("Next: eldiron-source play {}", project_dir.display());
    Ok(())
}

fn watch_project(project_dir: &Path, debounce: Duration) -> Result<(), String> {
    let project_dir = project_dir
        .canonicalize()
        .map_err(|err| format!("failed to resolve {}: {err}", project_dir.display()))?;

    build_once(&project_dir)?;

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |result| {
            let _ = tx.send(result);
        },
        Config::default(),
    )
    .map_err(|err| format!("failed to create file watcher: {err}"))?;
    watcher
        .watch(&project_dir, RecursiveMode::Recursive)
        .map_err(|err| format!("failed to watch {}: {err}", project_dir.display()))?;

    println!("Watching {}. Press Ctrl-C to stop.", project_dir.display());

    loop {
        let event = rx
            .recv()
            .map_err(|err| format!("file watcher stopped: {err}"))?
            .map_err(|err| format!("file watcher error: {err}"))?;
        if !should_rebuild_for_event(&project_dir, &event) {
            continue;
        }

        let mut rebuild_at = Instant::now() + debounce;
        while let Ok(result) = rx.recv_timeout(rebuild_at.saturating_duration_since(Instant::now()))
        {
            let event = result.map_err(|err| format!("file watcher error: {err}"))?;
            if should_rebuild_for_event(&project_dir, &event) {
                rebuild_at = Instant::now() + debounce;
            }
            if Instant::now() >= rebuild_at {
                break;
            }
        }

        match eldiron_source::build_project(&project_dir) {
            Ok(output) => println!("Rebuilt {}", output.display()),
            Err(err) => eprintln!("Build failed: {err}"),
        }
    }
}

fn should_rebuild_for_event(project_dir: &Path, event: &Event) -> bool {
    if !matches!(
        event.kind,
        EventKind::Any
            | EventKind::Create(_)
            | EventKind::Modify(_)
            | EventKind::Remove(_)
            | EventKind::Other
    ) {
        return false;
    }
    event
        .paths
        .iter()
        .any(|path| should_rebuild_for_path(project_dir, path))
}

fn should_rebuild_for_path(project_dir: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(project_dir).unwrap_or(path);
    if relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .any(|part| matches!(part, "build" | "dist" | ".git"))
    {
        return false;
    }

    if relative == Path::new("eldiron.toml") {
        return true;
    }

    let top_level = relative
        .components()
        .next()
        .and_then(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        });
    if matches!(
        top_level,
        Some("assets" | "characters" | "items" | "regions" | "scripts" | "tiles" | "images")
    ) {
        return true;
    }

    matches!(
        path.extension().and_then(OsStr::to_str),
        Some(
            "els"
                | "eldrin"
                | "toml"
                | "png"
                | "jpg"
                | "jpeg"
                | "ttf"
                | "otf"
                | "wav"
                | "ogg"
                | "mp3"
                | "flac"
        )
    )
}

fn project_name_from_dir(project_dir: &Path) -> String {
    project_dir
        .file_name()
        .and_then(OsStr::to_str)
        .map(title_from_slug)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "Eldiron Source Game".to_string())
}

fn title_from_slug(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_new_file(path: &Path, contents: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    fs::write(path, contents).map_err(format_io(path))
}

fn format_io(path: &Path) -> impl FnOnce(std::io::Error) -> String + '_ {
    move |err| format!("{}: {err}", path.display())
}

fn format_eldiron_toml(project_name: &str) -> String {
    format!(
        r#"[project]
name = "{}"
version = "0.1.0"

[source]
main = "main.els"

[game]
start_region = "cellar"
start_screen = "play"
client_mode = "terminal"
terminal_mode = "roguelike"
simulation_mode = "hybrid"
game_tick_ms = 250
turn_timeout_ms = 600
movement_units_per_sec = 4
turn_speed_deg_per_sec = 120
collision_mode = "tile"
auto_create_player = true
player = "player"

[viewport]
width = 80
height = 24
grid_size = 40
unit = "cell"
resize = "fit"

[terminal]
text_updates = true

[build]
output = "build/game.eldiron"
"#,
        escape_toml_string(project_name)
    )
}

fn escape_toml_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn source_client_mode(project_dir: &Path) -> Result<String, String> {
    let config_path = project_dir.join("eldiron.toml");
    let config = std::fs::read_to_string(&config_path)
        .map_err(|err| format!("Failed to read {}: {}", config_path.display(), err))?;
    let value = toml::from_str::<toml::Value>(&config)
        .map_err(|err| format!("Failed to parse {}: {}", config_path.display(), err))?;
    Ok(value
        .get("game")
        .and_then(toml::Value::as_table)
        .and_then(|game| game.get("client_mode"))
        .and_then(toml::Value::as_str)
        .unwrap_or("terminal")
        .to_string())
}

fn client_mode_is_graphical(mode: &str) -> bool {
    matches!(
        mode.trim().to_ascii_lowercase().as_str(),
        "3d" | "2d"
            | "graphical"
            | "client"
            | "firstp"
            | "firstp_grid"
            | "dungeon3d"
            | "dungeon_3d"
    )
}

fn run_terminal_client(game_path: &PathBuf) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        return run_terminal_client_through_cargo(game_path);
    }

    #[cfg(not(debug_assertions))]
    {
        let mut candidates = Vec::new();
        if let Ok(current_exe) = std::env::current_exe()
            && let Some(dir) = current_exe.parent()
        {
            candidates.push(dir.join("eldiron-client-terminal"));
        }
        candidates.push(PathBuf::from("target/debug/eldiron-client-terminal"));
        candidates.push(PathBuf::from("target/release/eldiron-client-terminal"));

        for candidate in candidates {
            if candidate.exists() {
                let status = Command::new(&candidate)
                    .arg(game_path)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
                    .map_err(|err| format!("Failed to start {}: {}", candidate.display(), err))?;
                if status.success() {
                    return Ok(());
                }
                return Err(format!("{} exited with {}", candidate.display(), status));
            }
        }

        run_terminal_client_through_cargo(game_path)
    }
}

fn run_terminal_client_through_cargo(game_path: &PathBuf) -> Result<(), String> {
    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("eldiron-client-terminal")
        .arg("--")
        .arg(game_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| format!("Failed to start terminal client through cargo: {}", err))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("terminal client exited with {}", status))
    }
}

fn run_graphical_client(game_path: &PathBuf) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        return run_graphical_client_through_cargo(game_path);
    }

    #[cfg(not(debug_assertions))]
    {
        let mut candidates = Vec::new();
        if let Ok(current_exe) = std::env::current_exe()
            && let Some(dir) = current_exe.parent()
        {
            candidates.push(dir.join("eldiron-client"));
        }
        candidates.push(PathBuf::from("target/debug/eldiron-client"));
        candidates.push(PathBuf::from("target/release/eldiron-client"));

        for candidate in candidates {
            if candidate.exists() {
                let status = Command::new(&candidate)
                    .arg(game_path)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
                    .map_err(|err| format!("Failed to start {}: {}", candidate.display(), err))?;
                if status.success() {
                    return Ok(());
                }
                return Err(format!("{} exited with {}", candidate.display(), status));
            }
        }

        run_graphical_client_through_cargo(game_path)
    }
}

fn run_graphical_client_through_cargo(game_path: &PathBuf) -> Result<(), String> {
    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("eldiron-client")
        .arg("--")
        .arg(game_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| format!("Failed to start graphical client through cargo: {}", err))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("graphical client exited with {}", status))
    }
}

const STARTER_MAIN_ELS: &str = r##"Region "cellar" {
  name = "Source Cellar"
  default = wall.stone

  terrain """
  #########
  #.......#
  #...@...#
  #.......#
  #########
  """
}

Screen "play" {
  name = "Play"

  widget "Game" {
    role = "game"
    x = 0
    y = 0
    width = 80
    height = 24

    data {
      [ui]
      role = "game"
      grid_size = 40

      [camera]
      type = "2d"
    }
  }
}
"##;

const STARTER_PLAYER_ELS: &str = r##"Character "player" {
  name = "Player"
  glyph = "@"

  data {
    [attributes]
    race = "Human"
    class = "Ranger"
    LEVEL = 1
    size_2d = 1.25
    visible = true
    player = true
    inventory_slots = 6
    source_id = "player"

    [input]
    w = "action(forward)"
    a = "action(left)"
    s = "action(backward)"
    d = "action(right)"
    up = "action(forward)"
    left = "action(left)"
    down = "action(backward)"
    right = "action(right)"
    t = "intent(attack)"
    g = "intent(take)"
    l = "intent(look)"
  }

  script {
    fn event(event, value) {
        if event == "startup" {
            set_player_camera("2d_grid");
        }
        if event == "death" {
            set_attr("mode", "active");
            set_attr("visible", true);
            message(id(), "You died. Try again!", "severe");
        }
        if event == "intent" && value == "attack" {
            set_target(value.subject_id);
            attack();
        }
        if event == "intent" && value == "take" {
            take(value.subject_id);
        }
        if event == "kill" {
            let name = get_attr_of(value, "name");
            message(id(), "You kill the " + name, "success");
        }
    }
  }
}
"##;
