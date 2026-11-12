//! Migration entry point. Canonical Creator retirement follows full parity.
use eldiron_creator::native_application::NativeCreatorApplication;
use theframework::platform::{UiApplicationAdapter, winit};
fn run() -> Result<(), String> {
    let mut paths = vec!["eldiron-creator-native".to_owned()];
    let mut smoke = false;
    let mut literal = false;
    for argument in std::env::args_os().skip(1) {
        let argument = argument
            .into_string()
            .map_err(|_| "Native Creator requires UTF-8 project paths")?;
        if !literal {
            match argument.as_str() {
                "--help" | "-h" => {
                    println!(
                        "Eldiron Creator native GPU migration host\nUsage: eldiron-creator-native [--smoke] [--] [PROJECT.eldiron ...]\n\nUses the common GPU UI host and native geometry/appearance previews. Main world/tool/history,\nremaining previews and full platform parity are still being migrated."
                    );
                    return Ok(());
                }
                "--version" | "-V" => {
                    println!("Eldiron Creator {}", env!("CARGO_PKG_VERSION"));
                    return Ok(());
                }
                "--smoke" => {
                    smoke = true;
                    continue;
                }
                "--" => {
                    literal = true;
                    continue;
                }
                option if option.starts_with('-') => {
                    return Err(format!("Unknown argument {option}; see --help"));
                }
                _ => {}
            }
        }
        paths.push(argument);
    }
    let _ = eldiron_creator::i18n::select_system_locales();
    winit::run(Box::new(UiApplicationAdapter::new(
        NativeCreatorApplication::new(paths, smoke),
    )))
    .map_err(|e| e.to_string())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
