use denrim_materials::{
    Error, Result, compile,
    cpu::{Evaluator, PreviewMode},
    format,
    ir::Channel,
    metadata::Kernel,
    metal,
};
use std::{env, fs, io::Write};

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "sources") {
        for k in Kernel::ALL {
            let m = k.metadata();
            println!("{} [{}]", m.name, m.category);
            for p in m.parameters {
                println!("  {}: {:?} = {:?} ({})", p.id, p.kind, p.default, p.label);
            }
        }
        return Ok(());
    }
    if args.as_slice() == ["catalog"] {
        println!("{}", denrim_materials::metadata::catalog_json());
        return Ok(());
    }
    let usage = "Usage:\n  materials sources\n  materials catalog\n  materials check PROJECT\n  materials format PROJECT OUTPUT\n  materials metal PROJECT MATERIAL CHANNEL OUTPUT.metal\n  materials live-metal PROJECT MATERIAL CHANNEL OUTPUT_PREFIX\n  materials thumbnail PROJECT MATERIAL ROW TILE MODE WIDTH HEIGHT OUTPUT.ppm\n  materials render PROJECT MATERIAL CHANNEL WIDTH HEIGHT OUTPUT.ppm\nQuote multiword channel names, e.g. \"base color\".";
    let get = |i: usize| args.get(i).ok_or_else(|| Error(usage.into()));
    let command = get(0)?;
    let expected = match command.as_str() {
        "check" => 2,
        "format" => 3,
        "metal" | "live-metal" => 5,
        "render" => 7,
        "thumbnail" => 9,
        _ => return Err(Error(usage.into())),
    };
    if args.len() != expected {
        return Err(Error(usage.into()));
    }
    let source = fs::read_to_string(get(1)?).map_err(|e| Error(e.to_string()))?;
    let ast = format::parse(&source)?;
    let program = compile(&source)?;
    let io = |e: std::io::Error| Error(e.to_string());
    match command.as_str() {
        "check" => {
            let cost = program
                .costs
                .iter()
                .map(|c| c.expanded_calls)
                .max()
                .unwrap_or(0);
            println!(
                "Valid: {} materials, {} tiles; maximum expanded calls {cost}",
                program.materials.len(),
                program.nodes.len()
            );
        }
        "format" => fs::write(get(2)?, format::write(&ast)).map_err(io)?,
        "metal" => fs::write(
            get(4)?,
            metal::generate(&program, get(2)?, Channel::parse(get(3)?)?)?,
        )
        .map_err(io)?,
        "live-metal" => {
            let shader = metal::compile(&program, get(2)?, Channel::parse(get(3)?)?)?;
            let prefix = get(4)?;
            fs::write(format!("{prefix}.metal"), &shader.source).map_err(io)?;
            fs::write(format!("{prefix}.params"), shader.parameter_bytes()).map_err(io)?;
            fs::write(format!("{prefix}.layout.json"), shader.layout_json()).map_err(io)?;
        }
        "thumbnail" => {
            let tile = get(4)?
                .parse::<usize>()
                .map_err(|_| Error("Invalid tile index".into()))?;
            let mode = PreviewMode::parse(get(5)?)?;
            let w = get(6)?
                .parse::<u32>()
                .map_err(|_| Error("Invalid width".into()))?;
            let h = get(7)?
                .parse::<u32>()
                .map_err(|_| Error("Invalid height".into()))?;
            let pixels =
                Evaluator::new(&program).render_tile(get(2)?, get(3)?, tile, mode, w, h)?;
            let mut out = fs::File::create(get(8)?).map_err(io)?;
            write!(out, "P6\n{w} {h}\n255\n").map_err(io)?;
            let mut bytes = Vec::with_capacity(pixels.len() * 3);
            for (i, p) in pixels.iter().enumerate() {
                let checker = if ((i % w as usize) / 8 + (i / w as usize) / 8).is_multiple_of(2) {
                    0.16
                } else {
                    0.24
                };
                for v in &p[..3] {
                    bytes.push(
                        ((v * p[3] + checker * (1. - p[3])).clamp(0., 1.) * 255.).round() as u8,
                    );
                }
            }
            out.write_all(&bytes).map_err(io)?;
        }
        "render" => {
            let dimension = |i| {
                get(i)?
                    .parse::<u32>()
                    .map_err(|_| Error("Invalid image dimension".into()))
            };
            let (w, h) = (dimension(4)?, dimension(5)?);
            let pixels =
                Evaluator::new(&program).render(get(2)?, Channel::parse(get(3)?)?, w, h)?;
            let mut out = fs::File::create(get(6)?).map_err(io)?;
            write!(out, "P6\n{w} {h}\n255\n").map_err(io)?;
            let mut bytes = Vec::with_capacity(pixels.len() * 3);
            for p in pixels {
                for c in 0..3 {
                    bytes.push(
                        (p[if p.len() == 1 { 0 } else { c }].clamp(0., 1.) * 255.).round() as u8,
                    );
                }
            }
            out.write_all(&bytes).map_err(io)?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
