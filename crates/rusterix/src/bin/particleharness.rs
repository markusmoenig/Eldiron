use image::RgbaImage;
use rusterix::particleharness::render_reference_pair;
use std::env;
use std::path::PathBuf;

fn save_png(path: &PathBuf, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), String> {
    let image = RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| "failed to create image buffer".to_string())?;
    image
        .save(path)
        .map_err(|err| format!("failed to save {}: {err}", path.display()))
}

fn main() {
    let mut size = 256u32;
    let mut time = 0.75f32;
    let mut out = PathBuf::from("crates/rusterix/examples");

    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--size" => {
                index += 1;
                size = args
                    .get(index)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(256)
                    .max(64);
            }
            "--time" => {
                index += 1;
                time = args.get(index).and_then(|v| v.parse().ok()).unwrap_or(0.75);
            }
            "--out" => {
                index += 1;
                out = PathBuf::from(args.get(index).cloned().unwrap_or_else(|| ".".into()));
            }
            _ => {}
        }
        index += 1;
    }

    let pair = render_reference_pair(size, size, time);
    let preview_path = out.join("particle_harness_preview.png");
    let wall_path = out.join("particle_harness_bright_wall.png");
    let builder_path = out.join("particle_harness_builder_iso.png");
    if let Err(err) = save_png(
        &preview_path,
        pair.preview.width,
        pair.preview.height,
        pair.preview.pixels,
    ) {
        eprintln!("{err}");
        std::process::exit(1);
    }
    if let Err(err) = save_png(
        &wall_path,
        pair.bright_wall.width,
        pair.bright_wall.height,
        pair.bright_wall.pixels,
    ) {
        eprintln!("{err}");
        std::process::exit(1);
    }
    if let Err(err) = save_png(
        &builder_path,
        pair.builder_iso.width,
        pair.builder_iso.height,
        pair.builder_iso.pixels,
    ) {
        eprintln!("{err}");
        std::process::exit(1);
    }

    println!("preview: {}", preview_path.display());
    println!("bright_wall: {}", wall_path.display());
    println!("builder_iso: {}", builder_path.display());
}
