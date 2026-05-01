use buildergraph::{
    BuilderCutMask, BuilderCutMode, BuilderCutShape, BuilderDocument, BuilderHost,
    BuilderPreviewHost, BuilderSurfaceDetail,
};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime};
use vek::Vec2;

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

fn run_shared_builderpreview(input_path: &str, out_path: &str, size: u32) -> Result<(), String> {
    let size_arg = size.clamp(128, 2048).to_string();
    let mut command = Command::new("cargo");
    command.args([
        "run",
        "-p",
        "rusterix",
        "--bin",
        "builderpreview",
        "--",
        input_path,
        "--size",
        &size_arg,
    ]);

    let output = command
        .output()
        .map_err(|err| format!("failed to run shared builder preview: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "shared builder preview failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
        ));
    }

    let default_out = Path::new(input_path).with_extension("png");
    let requested_out = Path::new(out_path);
    if default_out != requested_out {
        fs::copy(&default_out, requested_out).map_err(|err| {
            format!(
                "failed to copy preview {} to {}: {err}",
                default_out.display(),
                requested_out.display()
            )
        })?;
    }
    Ok(())
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

fn host_from_args(
    args: &[String],
    preview_host: Option<&BuilderPreviewHost>,
) -> Result<BuilderHost, String> {
    if let Some(path) = arg_value(args, "--host-json") {
        return load_host(&path);
    }

    let preview_width = preview_host.map(|host| host.width).unwrap_or(4.0);
    let preview_height = preview_host.map(|host| host.height).unwrap_or(2.5);
    let preview_depth = preview_host.map(|host| host.depth).unwrap_or(0.2);

    match arg_value(args, "--host")
        .unwrap_or_else(|| "wall".to_string())
        .as_str()
    {
        "object" => Ok(BuilderHost::preview_object(
            arg_f32(args, "--width", preview_width),
            arg_f32(args, "--depth", preview_depth),
            arg_f32(args, "--height", preview_height),
        )),
        "floor" | "sector" => Ok(BuilderHost::preview_floor(
            arg_f32(args, "--width", preview_width),
            arg_f32(args, "--depth", preview_depth),
        )),
        "linedef" | "line" => Ok(BuilderHost::preview_linedef(
            arg_f32(args, "--width", preview_width),
            arg_f32(args, "--height", preview_height),
            arg_f32(args, "--depth", preview_depth),
        )),
        "vertex" | "point" => Ok(BuilderHost::preview_vertex(
            arg_f32(args, "--width", preview_width),
            arg_f32(args, "--depth", preview_depth),
            arg_f32(args, "--height", preview_height),
        )),
        "terrain" => Ok(BuilderHost::preview_terrain(
            arg_f32(args, "--width", preview_width),
            arg_f32(args, "--depth", preview_depth),
            arg_u64(args, "--seed", 0),
        )),
        "wall" | "surface" => Ok(BuilderHost::preview_wall(
            arg_f32(args, "--width", preview_width),
            arg_f32(args, "--height", preview_height),
            arg_f32(args, "--thickness", preview_depth),
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

fn run_surface_command(path: &str, args: &[String], force_png: bool) -> Result<(), String> {
    let document = load_document(path)?;
    let preview_host = document.preview_host();
    let host = host_from_args(args, Some(&preview_host))?;
    let assembly = document.evaluate_with_host(&host)?;
    print_surface_debug(&document, &assembly, &host);

    let png_out =
        arg_optional_value(args, "--png").or_else(|| if force_png { Some(None) } else { None });
    if let Some(out) = png_out {
        let out = out.unwrap_or_else(|| default_png_path(path));
        run_shared_builderpreview(path, &out, arg_u32(args, "--png-size", 512))?;
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
            transition_height,
            segments,
            placement,
            cut_footprint,
            material_slot,
            rect_material_slot,
            cyl_material_slot,
            tile_alias,
        } = detail
        {
            println!(
                "detail[{index}]: column center=({:.3}, {:.3}) height={height:.3} radius={radius:.3} offset={offset:.3} base={base_height:.3} cap={cap_height:.3} transition={transition_height:.3} segments={segments} placement={placement:?} cut_footprint={cut_footprint} material={} rect_material={} cyl_material={} tile_alias={}",
                center.x,
                center.y,
                material_slot.as_deref().unwrap_or("-"),
                rect_material_slot.as_deref().unwrap_or("-"),
                cyl_material_slot.as_deref().unwrap_or("-"),
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

    let output_file = Path::new(path).with_extension("png");
    run_shared_builderpreview(path, &output_file.to_string_lossy(), 512)?;
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
            let preview_host = document.preview_host();
            let host = host_from_args(&args[3..], Some(&preview_host))?;
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
