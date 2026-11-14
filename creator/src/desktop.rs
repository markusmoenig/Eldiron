//! Shared desktop entry point for the canonical Creator and its native alias.
use crate::native_application::NativeCreatorApplication;
use theframework::platform::{UiApplicationAdapter, winit};
pub fn run() -> Result<(), String> {
    let mut paths = vec!["eldiron-creator".to_owned()];
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
                        "Eldiron Creator\nUsage: eldiron-creator [--smoke] [--] [PROJECT.eldiron ...]"
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
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    #[cfg(feature = "log")]
    theframework::thelogger::setup_logger();
    let _ = crate::i18n::select_system_locales();
    winit::run(Box::new(UiApplicationAdapter::new(
        NativeCreatorApplication::new(paths, smoke),
    )))
    .map_err(|e| e.to_string())
}
