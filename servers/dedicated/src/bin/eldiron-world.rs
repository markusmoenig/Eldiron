use eldiron_runtime::persistence::{read_snapshot, write_snapshot_new};
use eldiron_server::recovery;
use std::path::PathBuf;
fn main() {
    if let Err(error) = run() {
        eprintln!("Eldiron recovery: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "--help".into());
    if matches!(command.as_str(), "--help" | "help") {
        println!(
            "eldiron-world inspect --source world.save [--project game.eldiron]\neldiron-world backup --source world.save --output checkpoint.snapshot\neldiron-world restore --source checkpoint.snapshot --project game.eldiron --output fresh.save\neldiron-world migrate-deployment --source world.save --from-project old.eldiron --to-project new.eldiron --output fresh.save\neldiron-world reset-password --source world.save --project game.eldiron --username account --password-stdin --output recovered.save\nOutputs must be fresh paths. This tool never overwrites an existing world."
        );
        return Ok(());
    }
    let (mut source, mut output, mut project, mut from, mut to) = (None, None, None, None, None);
    let mut username = None;
    let mut password_stdin = false;
    while let Some(key) = args.next() {
        if key == "--password-stdin" {
            password_stdin = true;
            continue;
        }
        if key == "--username" {
            username = Some(args.next().ok_or("Missing account name")?);
            continue;
        }
        let value = PathBuf::from(args.next().ok_or("Missing option value")?);
        match key.as_str() {
            "--source" => source = Some(value),
            "--output" => output = Some(value),
            "--project" => project = Some(value),
            "--from-project" => from = Some(value),
            "--to-project" => to = Some(value),
            _ => return Err(format!("Unknown option {key}")),
        }
    }
    if command != "reset-password" && (username.is_some() || password_stdin) {
        return Err("Account options are only supported by reset-password".into());
    }
    let saved = read_snapshot(source.ok_or("--source is required")?).map_err(|e| e.to_string())?;
    let prepared = match command.as_str() {
        "inspect" => {
            if output.is_some() || from.is_some() || to.is_some() {
                return Err("inspect accepts --source and optional --project".into());
            }
            if let Some(path) = project {
                recovery::validate_project(&saved, &recovery::load_project(path)?)
                    .map_err(|e| e.to_string())?;
            }
            saved
        }
        "backup" => {
            if project.is_some() || from.is_some() || to.is_some() {
                return Err("backup accepts --source and --output".into());
            }
            write_snapshot_new(output.ok_or("--output is required")?, &saved)
                .map_err(|e| e.to_string())?;
            saved
        }
        "restore" => {
            if from.is_some() || to.is_some() {
                return Err("restore accepts --source, --project and --output".into());
            }
            recovery::validate_project(
                &saved,
                &recovery::load_project(project.ok_or("--project is required")?)?,
            )
            .map_err(|e| e.to_string())?;
            write_snapshot_new(output.ok_or("--output is required")?, &saved)
                .map_err(|e| e.to_string())?;
            saved
        }
        "reset-password" => {
            use std::io::{IsTerminal, Read};
            if from.is_some() || to.is_some() {
                return Err("reset-password uses --source, --project, --username, --password-stdin and --output".into());
            }
            if !password_stdin || std::io::stdin().is_terminal() {
                return Err("Use --password-stdin with a pipe; passwords cannot be command-line arguments or echoed terminal input".into());
            }
            let username = username.ok_or("--username is required")?;
            let project = recovery::load_project(project.ok_or("--project is required")?)?;
            let output = output.ok_or("--output is required")?;
            if output.exists() {
                return Err("Output exists; choose a fresh save path".into());
            }
            let mut bytes = Vec::new();
            std::io::stdin()
                .take(259)
                .read_to_end(&mut bytes)
                .map_err(|_| "Cannot read password input")?;
            if bytes.ends_with(b"\n") {
                bytes.pop();
                if bytes.ends_with(b"\r") {
                    bytes.pop();
                }
            }
            if bytes.len() > 256 || bytes.contains(&b'\n') || bytes.contains(&b'\r') {
                return Err("Provide exactly one password line, 12–256 UTF-8 bytes".into());
            }
            let password = String::from_utf8(bytes).map_err(|_| "Password must be UTF-8")?;
            let next = recovery::reset_password(&saved, &project, &username, &password)
                .map_err(|e| e.to_string())?;
            write_snapshot_new(output, &next).map_err(|e| e.to_string())?;
            next
        }
        "migrate-deployment" => {
            if project.is_some() {
                return Err("Use --from-project and --to-project for migration".into());
            }
            let from = recovery::load_project(from.ok_or("--from-project is required")?)?;
            let to = recovery::load_project(to.ok_or("--to-project is required")?)?;
            let next =
                recovery::migrate_deployment(&saved, &from, &to).map_err(|e| e.to_string())?;
            write_snapshot_new(output.ok_or("--output is required")?, &next)
                .map_err(|e| e.to_string())?;
            next
        }
        _ => {
            return Err("Use inspect, backup, restore, migrate-deployment or reset-password".into());
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&recovery::describe(&prepared)).map_err(|e| e.to_string())?
    );
    Ok(())
}
