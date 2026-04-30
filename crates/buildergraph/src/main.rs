use buildergraph::{
    BuilderCutMask, BuilderCutMode, BuilderCutShape, BuilderDocument, BuilderHost,
    BuilderSurfaceDetail,
};
use image::RgbaImage;
use std::env;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};
use vek::Vec2;

fn save_png(path: &Path, width: u32, height: u32, pixels: Vec<u8>) -> Result<(), String> {
    let image = RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| "failed to create image buffer".to_string())?;
    image
        .save(path)
        .map_err(|err| format!("failed to save {}: {err}", path.display()))
}

fn load_document(path: &str) -> Result<BuilderDocument, String> {
    let source = fs::read_to_string(path).map_err(|err| format!("failed to read {path}: {err}"))?;
    BuilderDocument::from_text(&source).map_err(|err| format!("failed to parse {path}: {err}"))
}

fn print_usage() {
    eprintln!("usage:");
    eprintln!("  buildergraph <file.buildergraph>");
    eprintln!("  buildergraph check <file.buildergraph>");
    eprintln!("  buildergraph inspect <file.buildergraph>");
    eprintln!(
        "  buildergraph eval <file.buildergraph> [--host wall|floor|linedef|vertex|object|terrain] [--host-json host.json] [--width N] [--height N] [--depth N] [--thickness N] [--seed N] [--out assembly.json]"
    );
    eprintln!(
        "  buildergraph surface <file.buildergraph> [--host floor|wall] [--width N] [--height N] [--depth N] [--thickness N] [--png [out.png]] [--png-size N] [--watch]"
    );
}

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn has_arg(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name)
}

fn arg_optional_value(args: &[String], name: &str) -> Option<Option<String>> {
    let index = args.iter().position(|arg| arg == name)?;
    let value = args.get(index + 1).and_then(|value| {
        if value.starts_with("--") {
            None
        } else {
            Some(value.clone())
        }
    });
    Some(value)
}

fn default_png_path(input_path: &str) -> String {
    let path = Path::new(input_path);
    path.with_extension("png").to_string_lossy().to_string()
}

fn arg_f32(args: &[String], name: &str, default: f32) -> f32 {
    arg_value(args, name)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(default)
}

fn arg_u64(args: &[String], name: &str, default: u64) -> u64 {
    arg_value(args, name)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn arg_u32(args: &[String], name: &str, default: u32) -> u32 {
    arg_value(args, name)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(default)
}

fn load_host(path: &str) -> Result<BuilderHost, String> {
    let source =
        fs::read_to_string(path).map_err(|err| format!("failed to read host {path}: {err}"))?;
    serde_json::from_str(&source)
        .or_else(|_| toml::from_str(&source))
        .map_err(|err| format!("failed to parse host {path}: {err}"))
}

fn host_from_args(args: &[String]) -> Result<BuilderHost, String> {
    if let Some(path) = arg_value(args, "--host-json") {
        return load_host(&path);
    }

    match arg_value(args, "--host")
        .unwrap_or_else(|| "wall".to_string())
        .as_str()
    {
        "object" => Ok(BuilderHost::preview_object(
            arg_f32(args, "--width", 1.0),
            arg_f32(args, "--depth", 1.0),
            arg_f32(args, "--height", 1.0),
        )),
        "floor" | "sector" => Ok(BuilderHost::preview_floor(
            arg_f32(args, "--width", 4.0),
            arg_f32(args, "--depth", 4.0),
        )),
        "linedef" | "line" => Ok(BuilderHost::preview_linedef(
            arg_f32(args, "--width", 4.0),
            arg_f32(args, "--height", 2.0),
            arg_f32(args, "--depth", 0.2),
        )),
        "vertex" | "point" => Ok(BuilderHost::preview_vertex(
            arg_f32(args, "--width", 1.0),
            arg_f32(args, "--depth", 1.0),
            arg_f32(args, "--height", 1.0),
        )),
        "terrain" => Ok(BuilderHost::preview_terrain(
            arg_f32(args, "--width", 16.0),
            arg_f32(args, "--depth", 16.0),
            arg_u64(args, "--seed", 0),
        )),
        "wall" | "surface" => Ok(BuilderHost::preview_wall(
            arg_f32(args, "--width", 4.0),
            arg_f32(args, "--height", 2.5),
            arg_f32(args, "--thickness", 0.2),
        )),
        other => Err(format!("unsupported host '{other}'")),
    }
}

fn print_inspection(document: &BuilderDocument, assembly: &buildergraph::BuilderAssembly) {
    let output_spec = document.output_spec();
    println!("graph: {}", document.name());
    println!(
        "target: {:?} (hosts: {})",
        output_spec.target, output_spec.host_refs
    );
    println!("primitives: {}", assembly.primitives.len());
    println!("anchors: {}", assembly.anchors.len());
    println!("cuts: {}", assembly.cuts.len());
    println!("surface details: {}", assembly.surface_details.len());
    println!(
        "static billboard batches: {}",
        assembly.static_billboards.len()
    );
    println!("warnings: {}", assembly.warnings.len());
    for warning in &assembly.warnings {
        println!("warning[{}]: {}", warning.code, warning.message);
    }
    for (index, primitive) in assembly.primitives.iter().enumerate() {
        println!("primitive[{index}]: {:?}", primitive);
    }
    for (index, anchor) in assembly.anchors.iter().enumerate() {
        println!("anchor[{index}]: {} {:?}", anchor.name, anchor.transform);
    }
    for (index, detail) in assembly.surface_details.iter().enumerate() {
        println!("surface_detail[{index}]: {:?}", detail);
    }
}

fn resolved_cut_loop(
    cut: &BuilderCutMask,
) -> Option<(BuilderCutMode, BuilderCutShape, f32, f32, Vec<Vec2<f32>>)> {
    match cut {
        BuilderCutMask::Rect {
            min,
            max,
            mode,
            offset,
            inset,
            shape,
        } => {
            if max.x <= min.x || max.y <= min.y {
                return None;
            }
            Some((
                *mode,
                *shape,
                *offset,
                *inset,
                vec![
                    Vec2::new(min.x, min.y),
                    Vec2::new(max.x, min.y),
                    Vec2::new(max.x, max.y),
                    Vec2::new(min.x, max.y),
                ],
            ))
        }
        BuilderCutMask::Loop {
            points,
            mode,
            offset,
            inset,
            shape,
        } => {
            if points.len() < 3 {
                return None;
            }
            Some((*mode, *shape, *offset, *inset, points.clone()))
        }
    }
}

fn resolved_detail_loop(
    detail: &BuilderSurfaceDetail,
) -> Option<(
    BuilderCutShape,
    f32,
    f32,
    Option<&str>,
    Option<&str>,
    Vec<Vec2<f32>>,
)> {
    match detail {
        BuilderSurfaceDetail::Rect {
            min,
            max,
            offset,
            inset,
            shape,
            material_slot,
            tile_alias,
        } => {
            if max.x <= min.x || max.y <= min.y {
                return None;
            }
            Some((
                *shape,
                *offset,
                *inset,
                material_slot.as_deref(),
                tile_alias.as_deref(),
                vec![
                    Vec2::new(min.x, min.y),
                    Vec2::new(max.x, min.y),
                    Vec2::new(max.x, max.y),
                    Vec2::new(min.x, max.y),
                ],
            ))
        }
        BuilderSurfaceDetail::Column {
            center,
            height,
            radius,
            offset,
            material_slot,
            tile_alias,
            ..
        } => {
            if *height <= 0.0 || *radius <= 0.0 {
                return None;
            }
            Some((
                BuilderCutShape::Fill,
                *offset,
                0.0,
                material_slot.as_deref(),
                tile_alias.as_deref(),
                vec![
                    Vec2::new(center.x - radius, center.y),
                    Vec2::new(center.x + radius, center.y),
                    Vec2::new(center.x + radius, center.y + height),
                    Vec2::new(center.x - radius, center.y + height),
                ],
            ))
        }
        BuilderSurfaceDetail::Masonry {
            min,
            max,
            offset,
            material_slot,
            tile_alias,
            ..
        } => {
            if max.x <= min.x || max.y <= min.y {
                return None;
            }
            Some((
                BuilderCutShape::Fill,
                *offset,
                0.0,
                material_slot.as_deref(),
                tile_alias.as_deref(),
                vec![
                    Vec2::new(min.x, min.y),
                    Vec2::new(max.x, min.y),
                    Vec2::new(max.x, max.y),
                    Vec2::new(min.x, max.y),
                ],
            ))
        }
    }
}

fn inset_loop(points: &[Vec2<f32>], inset: f32) -> Vec<Vec2<f32>> {
    if inset <= 0.0 || points.len() < 3 {
        return points.to_vec();
    }

    let mut center = Vec2::zero();
    for point in points {
        center += *point;
    }
    center /= points.len() as f32;

    points
        .iter()
        .map(|point| {
            let delta = *point - center;
            let len = delta.magnitude();
            if len <= inset || len <= 0.0001 {
                center
            } else {
                center + delta * ((len - inset) / len)
            }
        })
        .collect()
}

fn format_loop(points: &[Vec2<f32>]) -> String {
    points
        .iter()
        .map(|point| format!("({:.3}, {:.3})", point.x, point.y))
        .collect::<Vec<_>>()
        .join(", ")
}

fn host_surface_size(host: &BuilderHost) -> Vec2<f32> {
    match host {
        BuilderHost::Sector(host) => Vec2::new(host.width, host.depth),
        BuilderHost::Surface(host) => Vec2::new(host.width, host.height),
        BuilderHost::Terrain(host) => Vec2::new(host.width, host.depth),
        BuilderHost::Object(host) => Vec2::new(host.width, host.depth),
        BuilderHost::Linedef(host) => Vec2::new(host.length, host.height),
        BuilderHost::Vertex(host) => Vec2::new(host.width, host.depth),
    }
}

fn point_in_loop(point: Vec2<f32>, polygon: &[Vec2<f32>]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for current in polygon {
        let crosses_y = (current.y > point.y) != (previous.y > point.y);
        if crosses_y {
            let x_at_y = (previous.x - current.x) * (point.y - current.y)
                / (previous.y - current.y).max(f32::MIN_POSITIVE)
                + current.x;
            if point.x < x_at_y {
                inside = !inside;
            }
        }
        previous = *current;
    }
    inside
}

fn point_in_filled_region(point: Vec2<f32>, outer: &[Vec2<f32>], holes: &[Vec<Vec2<f32>>]) -> bool {
    point_in_loop(point, outer) && !holes.iter().any(|hole| point_in_loop(point, hole))
}

fn put_pixel(pixels: &mut [u8], width: u32, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 {
        return;
    }
    let index = (y as u32)
        .checked_mul(width)
        .and_then(|row| row.checked_add(x as u32))
        .and_then(|pixel| pixel.checked_mul(4))
        .map(|index| index as usize);
    let Some(index) = index else {
        return;
    };
    if index + 3 >= pixels.len() {
        return;
    }
    pixels[index..index + 4].copy_from_slice(&color);
}

fn draw_line(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    a: (i32, i32),
    b: (i32, i32),
    color: [u8; 4],
) {
    let (mut x0, mut y0) = a;
    let (x1, y1) = b;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if y0 >= 0 && y0 < height as i32 {
            put_pixel(pixels, width, x0, y0, color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn draw_loop_outline(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    points: &[Vec2<f32>],
    to_px: impl Fn(Vec2<f32>) -> (i32, i32),
    color: [u8; 4],
) {
    if points.len() < 2 {
        return;
    }
    for index in 0..points.len() {
        let a = to_px(points[index]);
        let b = to_px(points[(index + 1) % points.len()]);
        draw_line(pixels, width, height, a, b, color);
    }
}

fn render_surface_png(
    assembly: &buildergraph::BuilderAssembly,
    host: &BuilderHost,
    path: &Path,
    size: u32,
) -> Result<(), String> {
    let size = size.clamp(128, 2048);
    let host_size = host_surface_size(host);
    let margin = 24.0_f32;
    let drawable = (size as f32 - margin * 2.0).max(1.0);
    let scale = (drawable / host_size.x.max(0.01)).min(drawable / host_size.y.max(0.01));
    let surface_px = Vec2::new(host_size.x * scale, host_size.y * scale);
    let origin = Vec2::new(
        (size as f32 - surface_px.x) * 0.5,
        (size as f32 - surface_px.y) * 0.5,
    );
    let to_px = |point: Vec2<f32>| -> (i32, i32) {
        (
            (origin.x + point.x * scale).round() as i32,
            (origin.y + surface_px.y - point.y * scale).round() as i32,
        )
    };
    let from_px = |x: u32, y: u32| -> Vec2<f32> {
        Vec2::new(
            (x as f32 + 0.5 - origin.x) / scale,
            (surface_px.y - (y as f32 + 0.5 - origin.y)) / scale,
        )
    };

    let mut pixels = vec![245_u8; (size * size * 4) as usize];
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[28, 30, 34, 255]);
    }

    let host_outer = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(host_size.x, 0.0),
        Vec2::new(host_size.x, host_size.y),
        Vec2::new(0.0, host_size.y),
    ];

    for y in 0..size {
        for x in 0..size {
            let point = from_px(x, y);
            if point_in_loop(point, &host_outer) {
                let checker = (((point.x * 4.0).floor() + (point.y * 4.0).floor()) as i32) & 1;
                let color = if checker == 0 {
                    [174, 155, 118, 255]
                } else {
                    [161, 143, 108, 255]
                };
                put_pixel(&mut pixels, size, x as i32, y as i32, color);
            }
        }
    }

    for cut in &assembly.cuts {
        let Some((mode, shape, offset, inset, hole_loop)) = resolved_cut_loop(cut) else {
            continue;
        };
        if matches!(mode, BuilderCutMode::Cut | BuilderCutMode::Replace) {
            for y in 0..size {
                for x in 0..size {
                    let point = from_px(x, y);
                    if point_in_loop(point, &hole_loop) {
                        put_pixel(&mut pixels, size, x as i32, y as i32, [55, 49, 44, 255]);
                    }
                }
            }
        }

        if mode == BuilderCutMode::Replace {
            let inner_loop = inset_loop(&hole_loop, inset);
            let (cap_loop, cap_holes) = if shape == BuilderCutShape::Border {
                (
                    hole_loop.clone(),
                    if inset > 0.0 && inner_loop.len() >= 3 {
                        vec![inner_loop.clone()]
                    } else {
                        Vec::new()
                    },
                )
            } else {
                (inner_loop.clone(), Vec::new())
            };
            let cap_color = if offset < -0.001 {
                [80, 143, 190, 255]
            } else if offset > 0.001 {
                [89, 108, 130, 255]
            } else {
                [117, 138, 151, 255]
            };

            for y in 0..size {
                for x in 0..size {
                    let point = from_px(x, y);
                    if point_in_filled_region(point, &cap_loop, &cap_holes) {
                        put_pixel(&mut pixels, size, x as i32, y as i32, cap_color);
                    }
                }
            }

            let outline = if offset.abs() > 0.001 {
                [226, 202, 126, 255]
            } else {
                [218, 220, 224, 255]
            };
            draw_loop_outline(&mut pixels, size, size, &cap_loop, to_px, outline);
            for hole in cap_holes {
                draw_loop_outline(&mut pixels, size, size, &hole, to_px, [32, 35, 40, 255]);
            }
        }

        draw_loop_outline(
            &mut pixels,
            size,
            size,
            &hole_loop,
            to_px,
            [27, 29, 33, 255],
        );
    }

    draw_loop_outline(
        &mut pixels,
        size,
        size,
        &host_outer,
        to_px,
        [226, 216, 185, 255],
    );

    save_png(path, size, size, pixels)
}

fn run_surface_command(path: &str, args: &[String], force_png: bool) -> Result<(), String> {
    let document = load_document(path)?;
    let host = host_from_args(args)?;
    let assembly = document.evaluate_with_host(&host)?;
    print_surface_debug(&document, &assembly, &host);

    let png_out =
        arg_optional_value(args, "--png").or_else(|| if force_png { Some(None) } else { None });
    if let Some(out) = png_out {
        let out = out.unwrap_or_else(|| default_png_path(path));
        render_surface_png(
            &assembly,
            &host,
            Path::new(&out),
            arg_u32(args, "--png-size", 512),
        )?;
        println!("surface preview: {out}");
    } else if has_arg(args, "--png-size") {
        return Err("--png-size requires --png or --watch".to_string());
    }

    Ok(())
}

fn file_signature(path: &str) -> Result<(SystemTime, u64), String> {
    let metadata =
        fs::metadata(path).map_err(|err| format!("failed to read metadata for {path}: {err}"))?;
    let modified = metadata
        .modified()
        .map_err(|err| format!("failed to read modified time for {path}: {err}"))?;
    Ok((modified, metadata.len()))
}

fn watch_surface_command(path: &str, args: &[String]) -> Result<(), String> {
    println!("watching {path}");
    let mut last_signature: Option<(SystemTime, u64)> = None;

    loop {
        let signature = file_signature(path)?;
        if last_signature.as_ref() != Some(&signature) {
            println!("--- surface rebuild ---");
            if let Err(err) = run_surface_command(path, args, true) {
                eprintln!("{err}");
            }
            last_signature = Some(signature);
        }
        thread::sleep(Duration::from_millis(500));
    }
}

fn print_surface_debug(
    document: &BuilderDocument,
    assembly: &buildergraph::BuilderAssembly,
    host: &BuilderHost,
) {
    println!("graph: {}", document.name());
    println!("host: {}", host.kind_name());
    println!("cuts: {}", assembly.cuts.len());
    println!("surface details: {}", assembly.surface_details.len());
    if !assembly.warnings.is_empty() {
        println!("warnings: {}", assembly.warnings.len());
        for warning in &assembly.warnings {
            println!("warning[{}]: {}", warning.code, warning.message);
        }
    }

    for (index, cut) in assembly.cuts.iter().enumerate() {
        let Some((mode, shape, offset, inset, hole_loop)) = resolved_cut_loop(cut) else {
            println!("cut[{index}]: invalid");
            continue;
        };
        println!("cut[{index}]: mode={mode:?} shape={shape:?} offset={offset:.3} inset={inset:.3}");
        println!("  hole loop: {}", format_loop(&hole_loop));

        if mode == BuilderCutMode::Replace {
            let inner_loop = inset_loop(&hole_loop, inset);
            let cap_loop = if shape == BuilderCutShape::Border {
                hole_loop.clone()
            } else {
                inner_loop.clone()
            };
            let cap_holes = usize::from(shape == BuilderCutShape::Border && inset > 0.0);
            let side_walls = offset.abs() > 0.001;
            println!("  replacement cap: {}", format_loop(&cap_loop));
            if cap_holes > 0 {
                println!("  replacement cap holes: {}", format_loop(&inner_loop));
            }
            println!("  replacement side walls: {side_walls}");
            println!(
                "  expected meshes: base_with_hole=1 replacement_cap=1 replacement_sides={}",
                usize::from(side_walls)
            );
        }
    }

    for (index, detail) in assembly.surface_details.iter().enumerate() {
        if let BuilderSurfaceDetail::Column {
            center,
            height,
            radius,
            offset,
            base_height,
            cap_height,
            segments,
            placement,
            cut_footprint,
            material_slot,
            tile_alias,
        } = detail
        {
            println!(
                "detail[{index}]: column center=({:.3}, {:.3}) height={height:.3} radius={radius:.3} offset={offset:.3} base={base_height:.3} cap={cap_height:.3} segments={segments} placement={placement:?} cut_footprint={cut_footprint} material={} tile_alias={}",
                center.x,
                center.y,
                material_slot.as_deref().unwrap_or("-"),
                tile_alias.as_deref().unwrap_or("-")
            );
            continue;
        }
        if let BuilderSurfaceDetail::Masonry {
            min,
            max,
            block,
            mortar,
            offset,
            pattern,
            material_slot,
            tile_alias,
        } = detail
        {
            println!(
                "detail[{index}]: masonry min=({:.3}, {:.3}) max=({:.3}, {:.3}) block=({:.3}, {:.3}) mortar={mortar:.3} offset={offset:.3} pattern={pattern:?} material={} tile_alias={}",
                min.x,
                min.y,
                max.x,
                max.y,
                block.x,
                block.y,
                material_slot.as_deref().unwrap_or("-"),
                tile_alias.as_deref().unwrap_or("-")
            );
            continue;
        }
        let Some((shape, offset, inset, material, tile_alias, detail_loop)) =
            resolved_detail_loop(detail)
        else {
            println!("detail[{index}]: invalid");
            continue;
        };
        let inner_loop = inset_loop(&detail_loop, inset);
        let detail_holes = usize::from(shape == BuilderCutShape::Border && inset > 0.0);
        println!(
            "detail[{index}]: shape={shape:?} offset={offset:.3} inset={inset:.3} material={} tile_alias={}",
            material.unwrap_or("-"),
            tile_alias.unwrap_or("-")
        );
        println!("  detail loop: {}", format_loop(&detail_loop));
        if detail_holes > 0 {
            println!("  detail hole: {}", format_loop(&inner_loop));
        }
        println!("  detail side walls: {}", offset.abs() > 0.001);
    }
}

fn legacy_preview(path: &str) -> Result<(), String> {
    let document = load_document(path)?;
    let assembly = document.evaluate()?;
    print_inspection(&document, &assembly);
    let preview = document.render_preview(256);

    let input_path = Path::new(path);
    let output_file = if let Some(stem) = input_path.file_stem().and_then(|s| s.to_str()) {
        input_path.with_file_name(format!("{stem}.png"))
    } else {
        input_path.with_extension("png")
    };

    save_png(&output_file, preview.width, preview.height, preview.pixels)?;

    println!("preview: {}", output_file.display());
    Ok(())
}

fn run(args: Vec<String>) -> Result<(), String> {
    if args.len() < 2 {
        print_usage();
        return Err("missing command or input file".to_string());
    }

    match args[1].as_str() {
        "check" => {
            let Some(path) = args.get(2) else {
                print_usage();
                return Err("missing input file".to_string());
            };
            let document = load_document(path)?;
            let assembly = document.evaluate()?;
            println!("ok: {}", document.name());
            if !assembly.warnings.is_empty() {
                for warning in &assembly.warnings {
                    println!("warning[{}]: {}", warning.code, warning.message);
                }
            }
            Ok(())
        }
        "inspect" => {
            let Some(path) = args.get(2) else {
                print_usage();
                return Err("missing input file".to_string());
            };
            let document = load_document(path)?;
            let assembly = document.evaluate()?;
            print_inspection(&document, &assembly);
            Ok(())
        }
        "eval" => {
            let Some(path) = args.get(2) else {
                print_usage();
                return Err("missing input file".to_string());
            };
            let document = load_document(path)?;
            let host = host_from_args(&args[3..])?;
            let assembly = document.evaluate_with_host(&host)?;
            let json = serde_json::to_string_pretty(&assembly)
                .map_err(|err| format!("failed to encode assembly JSON: {err}"))?;
            if let Some(out) = arg_value(&args, "--out") {
                fs::write(&out, json).map_err(|err| format!("failed to write {out}: {err}"))?;
                println!("assembly: {out}");
            } else {
                println!("{json}");
            }
            Ok(())
        }
        "surface" => {
            let Some(path) = args.get(2) else {
                print_usage();
                return Err("missing input file".to_string());
            };
            if has_arg(&args, "--watch") {
                watch_surface_command(path, &args[3..])?;
            } else {
                run_surface_command(path, &args[3..], false)?;
            }
            Ok(())
        }
        "-h" | "--help" | "help" => {
            print_usage();
            Ok(())
        }
        path => legacy_preview(path),
    }
}

fn main() {
    if let Err(err) = run(env::args().collect()) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
