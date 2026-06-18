fn main() {
    if let Err(err) = eldiron_ruleset::cli::run_from_env() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
