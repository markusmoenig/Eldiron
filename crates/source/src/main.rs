use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    if let Err(err) = run() {
        eprintln!("eldiron-source: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "build".to_string());

    match command.as_str() {
        "build" => {
            let project_dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let output = eldiron_source::build_project(&project_dir)?;
            println!("Wrote {}", output.display());
            Ok(())
        }
        "play" => {
            let project_dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let output = eldiron_source::build_project(&project_dir)?;
            println!("Wrote {}", output.display());
            run_terminal_client(&output)
        }
        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }
        other => Err(format!(
            "unknown command '{other}'. Try 'eldiron-source help'."
        )),
    }
}

fn print_help() {
    println!(
        "eldiron-source\n\nUsage:\n  eldiron-source build [project-dir]\n  eldiron-source play [project-dir]\n\nCommands:\n  build    Compile eldiron.toml + .els source into a .eldiron file\n  play     Build, then play the generated .eldiron in the terminal"
    );
}

fn run_terminal_client(game_path: &PathBuf) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        return run_terminal_client_through_cargo(game_path);
    }

    #[cfg(not(debug_assertions))]
    {
        let mut candidates = Vec::new();
        if let Ok(current_exe) = env::current_exe()
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
