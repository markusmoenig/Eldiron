//! Export the actual palette-bound inventory pixels, without opening a GPU window.
//! cargo run -p eldiron-creator --example ruleset_icons -- <output-dir> [palette-size] [item-id]
use shared::{project::Project, ruleset_materials};
use theframework::prelude::TheRGBABuffer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or("/tmp/eldiron-ruleset-icons".into()));
    let mut project = Project::default();
    if let Some(size) = args.next() {
        project.palette.colors.truncate(size.parse::<usize>()?);
    }
    let filter = args.next();
    std::fs::create_dir_all(&out)?;
    let definitions = ruleset_materials::definitions(&project.config)
        .into_iter()
        .filter(|d| d.id.starts_with("icon/"))
        .filter(|d| {
            filter
                .as_ref()
                .is_none_or(|name| d.id.starts_with(&format!("icon/{name}/")))
        })
        .collect::<Vec<_>>();
    let columns = 7;
    let rows = definitions.len().div_ceil(columns);
    let mut sheet = vec![0; columns * 32 * rows * 32 * 4];
    let mut index = Vec::new();
    for (i, def) in definitions.iter().enumerate() {
        let texture = ruleset_materials::baked(&project, def.id)?;
        let name = def.id.strip_prefix("icon/").unwrap().replace('/', "-");
        std::fs::write(out.join(format!("{name}.png")), texture.to_rgba().to_png()?)?;
        std::fs::write(
            out.join(format!("{name}-large.png")),
            texture.resized(256, 256).to_rgba().to_png()?,
        )?;
        let frames = ruleset_materials::baked_frames(&project, def.id)?;
        if frames.len() > 1 {
            for (frame, texture) in frames.iter().enumerate() {
                std::fs::write(
                    out.join(format!("{name}-{frame}.png")),
                    texture.to_rgba().to_png()?,
                )?;
            }
        }
        for y in 0..32 {
            let to = (((i / columns) * 32 + y) * columns * 32 + (i % columns) * 32) * 4;
            sheet[to..to + 128].copy_from_slice(&texture.data[y * 128..y * 128 + 128]);
        }
        index.push(serde_json::json!({"id":def.id,"file":format!("{name}.png"),"column":i%columns,"row":i/columns,"frames":frames.len()}));
    }
    std::fs::write(
        out.join("icons.png"),
        TheRGBABuffer::from(sheet, (columns * 32) as u32, (rows * 32) as u32).to_png()?,
    )?;
    std::fs::write(
        out.join("index.json"),
        serde_json::to_string_pretty(&index)?,
    )?;
    eprintln!(
        "Exported {} procedural item states to {}",
        definitions.len(),
        out.display()
    );
    Ok(())
}
