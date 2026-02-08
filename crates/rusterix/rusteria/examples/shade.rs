use rusteria::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: shade <script.rusteria> [output.png] [width] [height]");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  cargo run --example shade -- examples/marble.rusteria");
        eprintln!("  cargo run --example shade -- examples/wood.rusteria output.png 512 512");
        std::process::exit(1);
    }

    let script_path = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(if args.len() > 2 {
        args[2].clone()
    } else {
        script_path
            .with_extension("png")
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    });
    let width: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(512);
    let height: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(512);

    // Parse
    let mut rusteria = Rusteria::new();
    let module = match rusteria.parse(script_path.clone()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    };

    // Compile
    if let Err(e) = rusteria.compile(&module) {
        eprintln!("Compile error: {}", e);
        std::process::exit(1);
    }

    // Find the shade function
    let shade_index = match rusteria
        .context
        .program
        .user_functions_name_map
        .get("shade")
        .copied()
    {
        Some(idx) => idx,
        None => {
            eprintln!("Error: no 'shade' function found in script");
            std::process::exit(1);
        }
    };

    // Shade
    let palette = rusteria.create_default_palette();
    let mut buffer = Arc::new(Mutex::new(RenderBuffer::new(width, height)));

    let start = std::time::Instant::now();
    rusteria.shade(&mut buffer, shade_index, &palette);
    let elapsed = start.elapsed();

    // Write PNG
    buffer.lock().unwrap().save(output_path.clone());

    println!(
        "Shaded {} -> {} ({}x{}) in {:.2?}",
        script_path.display(),
        output_path.display(),
        width,
        height,
        elapsed
    );
}
