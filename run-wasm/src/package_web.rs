//! Reproducible bindings from the workspace-pinned wasm-bindgen family.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: eldiron-package-web <input.wasm> <output-directory>".into());
    }
    wasm_bindgen_cli_support::Bindgen::new()
        .web(true)?
        .omit_default_module_path(false)
        .input_path(&args[0])
        .generate(&args[1])?;
    Ok(())
}
