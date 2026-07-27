use crate::ast::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl ParseError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

impl Error for ParseError {}

#[derive(Clone, Debug)]
struct SourceLine {
    number: usize,
    indent: usize,
    text: String,
}

#[derive(Clone, Debug)]
struct Node {
    name: String,
    line: usize,
    fields: BTreeMap<String, (String, usize)>,
    children: Vec<Node>,
}

pub fn parse_recipe(source: &str) -> Result<Recipe, ParseError> {
    match parse_document(source)? {
        RecipeDocument::Tile(recipe) => Ok(recipe),
        RecipeDocument::Materials(_) => Err(ParseError::new(
            0,
            "expected a Tile recipe, found Material declarations",
        )),
    }
}

pub fn parse_material_document(source: &str) -> Result<MaterialDocument, ParseError> {
    match parse_document(source)? {
        RecipeDocument::Materials(materials) => Ok(materials),
        RecipeDocument::Tile(_) => Err(ParseError::new(
            0,
            "expected Material declarations, found a Tile recipe",
        )),
    }
}

pub fn parse_document(source: &str) -> Result<RecipeDocument, ParseError> {
    let lines = tokenize(source)?;
    if lines.is_empty() {
        return Err(ParseError::new(0, "recipe is empty"));
    }
    let mut cursor = 0;
    let roots = parse_nodes(&lines, &mut cursor, 0)?;
    if roots.len() == 1 && roots[0].name.eq_ignore_ascii_case("Tile") {
        return recipe_from_node(&roots[0]).map(RecipeDocument::Tile);
    }

    let mut ids = BTreeSet::new();
    let mut materials = Vec::with_capacity(roots.len());
    for root in &roots {
        let (kind, declared_name) = declaration(&root.name);
        if kind != "material" {
            return Err(ParseError::new(
                root.line,
                "a .recipe must contain either one top-level Tile block or only Material <id> blocks",
            ));
        }
        let id = required_declaration_name(root, declared_name)?;
        let normalized = id.to_ascii_lowercase();
        if !ids.insert(normalized) {
            return Err(ParseError::new(
                root.line,
                format!("duplicate material id '{id}'"),
            ));
        }
        materials.push(material_from_node(root, id)?);
    }
    Ok(RecipeDocument::Materials(MaterialDocument { materials }))
}

fn tokenize(source: &str) -> Result<Vec<SourceLine>, ParseError> {
    let mut result = Vec::new();
    for (index, raw) in source.lines().enumerate() {
        let line_number = index + 1;
        if raw.contains('\t') {
            return Err(ParseError::new(
                line_number,
                "tabs are not allowed; indent with spaces",
            ));
        }
        let without_comment = raw.split_once("//").map_or(raw, |(text, _)| text);
        let text = without_comment.trim_end();
        if text.trim().is_empty() {
            continue;
        }
        result.push(SourceLine {
            number: line_number,
            indent: text.len() - text.trim_start().len(),
            text: text.trim_start().to_string(),
        });
    }
    Ok(result)
}

fn parse_nodes(
    lines: &[SourceLine],
    cursor: &mut usize,
    indent: usize,
) -> Result<Vec<Node>, ParseError> {
    let mut nodes = Vec::new();
    while *cursor < lines.len() {
        let line = &lines[*cursor];
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(ParseError::new(
                line.number,
                "unexpected indentation; blocks must be introduced by a name",
            ));
        }
        if line.text.contains('=') {
            return Err(ParseError::new(
                line.number,
                "a field must be nested inside a block",
            ));
        }

        let mut node = Node {
            name: line.text.trim().to_string(),
            line: line.number,
            fields: BTreeMap::new(),
            children: Vec::new(),
        };
        *cursor += 1;
        let child_indent = lines
            .get(*cursor)
            .and_then(|next| (next.indent > indent).then_some(next.indent));
        let Some(child_indent) = child_indent else {
            nodes.push(node);
            continue;
        };

        while *cursor < lines.len() && lines[*cursor].indent == child_indent {
            let child_line = &lines[*cursor];
            if let Some((key, value)) = child_line.text.split_once('=') {
                let key = key.trim().to_ascii_lowercase();
                if key.is_empty() || value.trim().is_empty() {
                    return Err(ParseError::new(
                        child_line.number,
                        "field assignments require a name and value",
                    ));
                }
                if node
                    .fields
                    .insert(key.clone(), (value.trim().to_string(), child_line.number))
                    .is_some()
                {
                    return Err(ParseError::new(
                        child_line.number,
                        format!("duplicate field '{key}'"),
                    ));
                }
                *cursor += 1;
            } else {
                node.children
                    .extend(parse_nodes(lines, cursor, child_indent)?);
            }
        }
        nodes.push(node);
    }
    Ok(nodes)
}

fn recipe_from_node(node: &Node) -> Result<Recipe, ParseError> {
    reject_unknown_fields(
        node,
        &[
            "name", "material", "size", "coverage", "wrap", "seed", "pixelate",
        ],
    )?;
    let mut recipe = Recipe {
        name: field(node, "name")
            .map(|(value, line)| parse_string(value, line))
            .transpose()?
            .unwrap_or_else(|| "Untitled Tile".to_string()),
        material: field(node, "material")
            .map(|(value, line)| parse_alias(value, line))
            .transpose()?,
        size: field(node, "size")
            .map(|(value, line)| parse_i2(value, line))
            .transpose()?
            .unwrap_or([64, 64]),
        coverage: field(node, "coverage")
            .map(|(value, line)| parse_i2(value, line))
            .transpose()?
            .unwrap_or([1, 1]),
        wrap: parse_wrap(node)?,
        seed: optional_u64(node, "seed", 1)?,
        pixelate: optional_u32(node, "pixelate", 1)?.max(1),
        ..Recipe::default()
    };

    let mut colorize = None;
    let mut output = None;
    let mut animation = None;
    for child in &node.children {
        let (kind, declared_name) = declaration(&child.name);
        match kind.as_str() {
            "animation" => {
                ensure_unnamed(child, declared_name)?;
                if animation.replace(parse_animation(child)?).is_some() {
                    return Err(ParseError::new(child.line, "duplicate Animation block"));
                }
            }
            "noise" => recipe
                .fields
                .push(FieldDefinition::Noise(parse_noise(child, declared_name)?)),
            "height" => recipe
                .fields
                .push(FieldDefinition::Height(parse_height(child, declared_name)?)),
            "pattern" => recipe.patterns.push(parse_pattern(child, declared_name)?),
            "colorize" => {
                ensure_unnamed(child, declared_name)?;
                if colorize.replace(parse_colorize(child)?).is_some() {
                    return Err(ParseError::new(child.line, "duplicate Colorize block"));
                }
            }
            "output" => {
                ensure_unnamed(child, declared_name)?;
                if output.replace(parse_output(child)?).is_some() {
                    return Err(ParseError::new(child.line, "duplicate Output block"));
                }
            }
            _ => {
                return Err(ParseError::new(
                    child.line,
                    format!(
                        "unknown top-level block '{}'; expected Noise <name>, Pattern <name>, Height <name>, Colorize, or Output",
                        child.name
                    ),
                ));
            }
        }
    }
    recipe.animation = animation.unwrap_or_default();
    recipe.output = output.ok_or_else(|| ParseError::new(node.line, "Tile requires Output"))?;
    recipe.colorize = match colorize {
        Some(colorize) => colorize,
        None if recipe.material.is_some() => Colorize {
            source: recipe.output.height.clone(),
            ..Colorize::default()
        },
        None => return Err(ParseError::new(node.line, "Tile requires Colorize")),
    };
    validate_names_and_references(&recipe, node.line)?;
    Ok(recipe)
}

fn material_from_node(node: &Node, id: &str) -> Result<MaterialRecipe, ParseError> {
    reject_unknown_fields(node, &["name", "wrap", "seed"])?;
    let mut material = MaterialRecipe {
        id: id.to_string(),
        name: field(node, "name")
            .map(|(value, line)| parse_string(value, line))
            .transpose()?
            .unwrap_or_else(|| id.replace('_', " ")),
        wrap: parse_wrap(node)?,
        seed: optional_u64(node, "seed", 1)?,
        fields: Vec::new(),
        patterns: Vec::new(),
        colorize: Colorize::default(),
        data: MaterialData::default(),
        normal: MaterialNormal::default(),
    };

    let mut colorize = None;
    let mut data = None;
    let mut normal = None;
    for child in &node.children {
        let (kind, declared_name) = declaration(&child.name);
        match kind.as_str() {
            "noise" => material
                .fields
                .push(FieldDefinition::Noise(parse_noise(child, declared_name)?)),
            "height" => material
                .fields
                .push(FieldDefinition::Height(parse_height(child, declared_name)?)),
            "pattern" => material.patterns.push(parse_pattern(child, declared_name)?),
            "colorize" => {
                ensure_unnamed(child, declared_name)?;
                if colorize.replace(parse_colorize(child)?).is_some() {
                    return Err(ParseError::new(child.line, "duplicate Colorize block"));
                }
            }
            "materialdata" | "data" => {
                ensure_unnamed(child, declared_name)?;
                if data.replace(parse_material_data(child)?).is_some() {
                    return Err(ParseError::new(child.line, "duplicate MaterialData block"));
                }
            }
            "normal" => {
                ensure_unnamed(child, declared_name)?;
                if normal.replace(parse_material_normal(child)?).is_some() {
                    return Err(ParseError::new(child.line, "duplicate Normal block"));
                }
            }
            _ => {
                return Err(ParseError::new(
                    child.line,
                    format!(
                        "unknown material block '{}'; expected Noise <name>, Pattern <name>, Height <name>, Colorize, MaterialData, or Normal",
                        child.name
                    ),
                ));
            }
        }
    }
    material.colorize =
        colorize.ok_or_else(|| ParseError::new(node.line, "Material requires Colorize"))?;
    material.data = data.unwrap_or_default();
    material.normal = normal.unwrap_or_default();
    validate_material_names_and_references(&material, node.line)?;
    Ok(material)
}

fn declaration(value: &str) -> (String, Option<&str>) {
    let mut words = value.split_whitespace();
    let kind = words.next().unwrap_or_default().to_ascii_lowercase();
    let name = words.next();
    if words.next().is_some() {
        return (value.to_ascii_lowercase(), None);
    }
    (kind, name)
}

fn ensure_unnamed(node: &Node, name: Option<&str>) -> Result<(), ParseError> {
    if name.is_some() {
        Err(ParseError::new(
            node.line,
            format!("{} does not take a declaration name", node.name),
        ))
    } else {
        Ok(())
    }
}

fn required_declaration_name<'a>(
    node: &Node,
    name: Option<&'a str>,
) -> Result<&'a str, ParseError> {
    let name = name.ok_or_else(|| {
        ParseError::new(
            node.line,
            format!(
                "{} requires a name, for example '{} Surface'",
                node.name, node.name
            ),
        )
    })?;
    validate_identifier(name, node.line)?;
    Ok(name)
}

fn parse_animation(node: &Node) -> Result<Animation, ParseError> {
    reject_unknown_fields(node, &["frames", "fps", "looping"])?;
    reject_children(node)?;
    Ok(Animation {
        frames: optional_u32(node, "frames", 1)?.clamp(1, 1024),
        fps: optional_f32(node, "fps", 12.0)?.max(0.001),
        looping: optional_bool(node, "looping", true)?,
    })
}

fn parse_noise(node: &Node, name: Option<&str>) -> Result<NoiseField, ParseError> {
    reject_unknown_fields(
        node,
        &[
            "space",
            "key",
            "type",
            "kind",
            "fractal",
            "scale",
            "octaves",
            "persistence",
            "seed",
        ],
    )?;
    reject_children(node)?;
    if field(node, "type").is_some() && field(node, "kind").is_some() {
        return Err(ParseError::new(
            field(node, "kind").unwrap().1,
            "Noise.type and Noise.kind are aliases; specify only one",
        ));
    }
    Ok(NoiseField {
        name: required_declaration_name(node, name)?.to_string(),
        domain: field(node, "space")
            .map(|(value, line)| parse_domain(value, line))
            .transpose()?
            .unwrap_or(Domain::Global),
        key: field(node, "key")
            .map(|(value, line)| parse_id_source(value, line))
            .transpose()?,
        kind: field(node, "type")
            .or_else(|| field(node, "kind"))
            .map(|(value, line)| parse_noise_kind(value, line))
            .transpose()?
            .unwrap_or_default(),
        fractal: field(node, "fractal")
            .map(|(value, line)| parse_fractal_kind(value, line))
            .transpose()?
            .unwrap_or_default(),
        scale: optional_positive_f2(node, "scale", [4.0, 4.0])?,
        octaves: optional_u32(node, "octaves", 1)?.clamp(1, 8),
        persistence: optional_f32(node, "persistence", 0.5)?.clamp(0.0, 1.0),
        seed: optional_u64(node, "seed", 0)?,
    })
}

fn parse_height(node: &Node, name: Option<&str>) -> Result<HeightField, ParseError> {
    reject_unknown_fields(node, &["source"])?;
    let source = required_scalar(node, "source")?;
    let operations = node
        .children
        .iter()
        .map(parse_height_operation)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(HeightField {
        name: required_declaration_name(node, name)?.to_string(),
        source,
        operations,
    })
}

fn parse_height_operation(node: &Node) -> Result<HeightOperation, ParseError> {
    let kind = node.name.to_ascii_lowercase();
    match kind.as_str() {
        "shape" => {
            reject_unknown_fields(node, &["contrast", "bias", "plateau", "rim"])?;
            reject_children(node)?;
            Ok(HeightOperation::Shape {
                contrast: optional_scalar(node, "contrast", 1.0)?,
                bias: optional_scalar(node, "bias", 0.0)?,
                plateau: optional_scalar(node, "plateau", 0.0)?,
                rim: optional_scalar(node, "rim", 0.0)?,
            })
        }
        "add" | "subtract" | "multiply" | "min" | "max" => {
            reject_unknown_fields(node, &["source", "amount"])?;
            reject_children(node)?;
            let mode = match kind.as_str() {
                "add" => CombineMode::Add,
                "subtract" => CombineMode::Subtract,
                "multiply" => CombineMode::Multiply,
                "min" => CombineMode::Min,
                _ => CombineMode::Max,
            };
            Ok(HeightOperation::Combine {
                mode,
                source: required_scalar(node, "source")?,
                amount: optional_scalar(node, "amount", 1.0)?,
            })
        }
        "clamp" => {
            reject_unknown_fields(node, &["min", "max"])?;
            reject_children(node)?;
            Ok(HeightOperation::Clamp {
                min: optional_scalar(node, "min", 0.0)?,
                max: optional_scalar(node, "max", 1.0)?,
            })
        }
        "remap" => {
            reject_unknown_fields(node, &["from", "to"])?;
            reject_children(node)?;
            Ok(HeightOperation::Remap {
                from: optional_f2(node, "from", [0.0, 1.0])?,
                to: optional_f2(node, "to", [0.0, 1.0])?,
            })
        }
        "terrace" => {
            reject_unknown_fields(node, &["steps", "smoothness"])?;
            reject_children(node)?;
            Ok(HeightOperation::Terrace {
                steps: optional_u32(node, "steps", 4)?.clamp(2, 64),
                smoothness: optional_f32(node, "smoothness", 0.0)?.clamp(0.0, 1.0),
            })
        }
        "invert" => {
            reject_unknown_fields(node, &[])?;
            reject_children(node)?;
            Ok(HeightOperation::Invert)
        }
        _ => Err(ParseError::new(
            node.line,
            format!("unknown height operation '{}'", node.name),
        )),
    }
}

fn parse_pattern(node: &Node, name: Option<&str>) -> Result<PatternDefinition, ParseError> {
    reject_unknown_fields(node, &[])?;
    if node.children.len() != 1 {
        return Err(ParseError::new(
            node.line,
            "Pattern requires exactly one Bricks, Voronoi, or Discs block",
        ));
    }
    let generator = &node.children[0];
    let (kind, allowed): (PatternKind, Vec<&str>) =
        match generator.name.to_ascii_lowercase().as_str() {
            "bricks" => (
                PatternKind::Bricks {
                    columns: optional_u32(generator, "columns", 8)?.max(1),
                    rows: optional_u32(generator, "rows", 8)?.max(1),
                    stagger: optional_f32(generator, "stagger", 0.5)?,
                    gap: optional_scalar(generator, "gap", 0.08)?,
                    rounding: optional_scalar(generator, "rounding", 0.04)?,
                    rotation: optional_scalar(generator, "rotation", 0.0)?,
                    size_variation: optional_f2(generator, "size_variation", [0.0, 0.0])?
                        .map(|value| value.abs().clamp(0.0, 0.45)),
                    perturb: field(generator, "perturb")
                        .map(|(value, line)| parse_scalar_source(value, line))
                        .transpose()?,
                    perturb_amount: optional_scalar(generator, "perturb_amount", 0.0)?,
                    falloff: optional_scalar(generator, "falloff", 1.0)?,
                    seed: optional_u64(generator, "seed", 0)?,
                },
                vec![
                    "columns",
                    "rows",
                    "stagger",
                    "gap",
                    "rounding",
                    "rotation",
                    "size_variation",
                    "perturb",
                    "perturb_amount",
                    "space",
                    "key",
                    "seed",
                    "falloff",
                ],
            ),
            "voronoi" => (
                PatternKind::Voronoi {
                    cells: optional_i2(generator, "cells", [6, 6])?,
                    jitter: optional_f32(generator, "jitter", 0.8)?.clamp(0.0, 1.0),
                    falloff: optional_f32(generator, "falloff", 1.0)?.clamp(0.1, 8.0),
                    seed: optional_u64(generator, "seed", 0)?,
                },
                vec![
                    "cells",
                    "jitter",
                    "space",
                    "key",
                    "warp",
                    "warp_amount",
                    "seed",
                    "falloff",
                ],
            ),
            "discs" => (
                PatternKind::Discs {
                    cells: optional_i2(generator, "cells", [8, 8])?,
                    jitter: optional_scalar(generator, "jitter", 1.0)?,
                    radius: optional_scalar(generator, "radius", 0.5)?,
                    falloff: optional_scalar(generator, "falloff", 1.0)?,
                    seed: optional_u64(generator, "seed", 0)?,
                },
                vec![
                    "cells",
                    "jitter",
                    "radius",
                    "space",
                    "key",
                    "warp",
                    "warp_amount",
                    "seed",
                    "falloff",
                ],
            ),
            _ => {
                return Err(ParseError::new(
                    generator.line,
                    "Pattern generator must be Bricks, Voronoi, or Discs",
                ));
            }
        };
    reject_unknown_fields(generator, &allowed)?;
    reject_children(generator)?;
    let domain = field(generator, "space")
        .map(|(value, line)| parse_domain(value, line))
        .transpose()?
        .unwrap_or(Domain::Global);
    let key = field(generator, "key")
        .map(|(value, line)| parse_id_source(value, line))
        .transpose()?;
    let warp = field(generator, "warp")
        .map(|(value, line)| {
            Ok::<_, ParseError>(Warp {
                source: parse_scalar_source(value, line)?,
                amount: optional_f32(generator, "warp_amount", 0.05)?
                    .abs()
                    .clamp(0.0, 1.0),
            })
        })
        .transpose()?;
    if warp.is_none() && field(generator, "warp_amount").is_some() {
        return Err(ParseError::new(
            field(generator, "warp_amount").unwrap().1,
            "warp_amount requires warp",
        ));
    }
    if let PatternKind::Bricks { perturb, .. } = &kind
        && perturb.is_none()
        && field(generator, "perturb_amount").is_some()
    {
        return Err(ParseError::new(
            field(generator, "perturb_amount").unwrap().1,
            "perturb_amount requires perturb",
        ));
    }
    Ok(PatternDefinition {
        name: required_declaration_name(node, name)?.to_string(),
        domain,
        key,
        warp,
        kind,
    })
}

fn parse_colorize(node: &Node) -> Result<Colorize, ParseError> {
    reject_unknown_fields(
        node,
        &[
            "source",
            "palette",
            "base",
            "brightness",
            "saturation",
            "hue",
            "ramp",
            "ramp_range",
            "saturation_range",
            "steps",
            "range",
            "dither",
        ],
    )?;
    reject_children(node)?;
    let base = field(node, "base")
        .map(|(value, line)| parse_base_color(value, line))
        .transpose()?;
    let palette = field(node, "palette")
        .map(|(value, line)| parse_palette_mode(value, line))
        .transpose()?
        .unwrap_or_default();
    if palette == PaletteMode::BaseOnly && base.is_none() {
        return Err(ParseError::new(
            field(node, "palette").unwrap().1,
            "Colorize.palette = BaseOnly requires Colorize.base",
        ));
    }
    if base.is_some() {
        for legacy in ["ramp", "ramp_range", "saturation_range"] {
            if let Some((_, line)) = field(node, legacy) {
                return Err(ParseError::new(
                    line,
                    format!("Colorize.{legacy} cannot be combined with Colorize.base"),
                ));
            }
        }
    } else {
        for anchored in ["brightness", "saturation", "hue"] {
            if let Some((_, line)) = field(node, anchored) {
                return Err(ParseError::new(
                    line,
                    format!("Colorize.{anchored} requires Colorize.base"),
                ));
            }
        }
    }
    Ok(Colorize {
        source: required_scalar(node, "source")?,
        palette,
        base,
        brightness: optional_f2(node, "brightness", [-0.22, 0.22])?
            .map(|value| value.clamp(-1.0, 1.0)),
        saturation: optional_f2(node, "saturation", [-0.08, 0.08])?
            .map(|value| value.clamp(-1.0, 1.0)),
        hue: optional_f2(node, "hue", [0.0, 0.0])?.map(|value| value.clamp(-1.0, 1.0)),
        family: field(node, "ramp")
            .map(|(value, line)| parse_family(value, line))
            .transpose()?
            .unwrap_or(ColorFamily::Any),
        ramp_range: optional_f2(node, "ramp_range", [0.0, 1.0])?.map(|value| value.clamp(0.0, 1.0)),
        saturation_range: optional_f2(node, "saturation_range", [0.0, 1.0])?
            .map(|value| value.clamp(0.0, 1.0)),
        steps: optional_u32(node, "steps", 4)?.clamp(2, 256),
        range: field(node, "range")
            .map(|(value, line)| parse_color_range(value, line))
            .transpose()?
            .unwrap_or(ColorRange::Auto),
        dither: optional_bool(node, "dither", false)?,
    })
}

fn parse_palette_mode(value: &str, line: usize) -> Result<PaletteMode, ParseError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "strict" => Ok(PaletteMode::Strict),
        "baseonly" | "base_only" | "base-only" => Ok(PaletteMode::BaseOnly),
        _ => Err(ParseError::new(
            line,
            "Colorize.palette must be Strict or BaseOnly",
        )),
    }
}

fn parse_output(node: &Node) -> Result<Output, ParseError> {
    reject_unknown_fields(node, &["height"])?;
    reject_children(node)?;
    Ok(Output {
        height: required_scalar(node, "height")?,
    })
}

fn parse_material_data(node: &Node) -> Result<MaterialData, ParseError> {
    reject_unknown_fields(
        node,
        &[
            "roughness",
            "metallic",
            "metal",
            "opacity",
            "emissive",
            "emission",
        ],
    )?;
    reject_children(node)?;
    reject_alias_pair(node, "metallic", "metal")?;
    reject_alias_pair(node, "emissive", "emission")?;
    Ok(MaterialData {
        roughness: optional_scalar(node, "roughness", 0.5)?,
        metallic: optional_scalar_alias(node, "metallic", "metal", 0.0)?,
        opacity: optional_scalar(node, "opacity", 1.0)?,
        emissive: optional_scalar_alias(node, "emissive", "emission", 0.0)?,
    })
}

fn parse_material_normal(node: &Node) -> Result<MaterialNormal, ParseError> {
    reject_unknown_fields(node, &["source", "strength"])?;
    reject_children(node)?;
    Ok(MaterialNormal {
        source: field(node, "source")
            .map(|(value, line)| parse_scalar_source(value, line))
            .transpose()?
            .unwrap_or(ScalarSource::InputHeight),
        strength: optional_f32(node, "strength", 0.35)?.clamp(0.0, 8.0),
    })
}

fn validate_names_and_references(recipe: &Recipe, line: usize) -> Result<(), ParseError> {
    let mut fields = BTreeSet::new();
    let mut patterns = BTreeSet::new();
    for field in &recipe.fields {
        let name = field.name().to_ascii_lowercase();
        if !fields.insert(name.clone()) {
            return Err(ParseError::new(
                line,
                format!("duplicate field name '{}'", field.name()),
            ));
        }
    }
    for pattern in &recipe.patterns {
        let name = pattern.name.to_ascii_lowercase();
        if fields.contains(&name) || !patterns.insert(name) {
            return Err(ParseError::new(
                line,
                format!("duplicate definition name '{}'", pattern.name),
            ));
        }
    }
    let validate_scalar =
        |source: &ScalarSource| validate_scalar_source(source, &fields, &patterns);
    for field in &recipe.fields {
        match field {
            FieldDefinition::Noise(noise) => {
                if let Domain::PatternLocal(name) = &noise.domain {
                    require_pattern(name, &patterns)?;
                }
                if let Some(IdSource::Pattern(name)) = &noise.key {
                    require_pattern(name, &patterns)?;
                }
            }
            FieldDefinition::Height(height) => {
                validate_scalar(&height.source)?;
                for operation in &height.operations {
                    match operation {
                        HeightOperation::Shape {
                            contrast,
                            bias,
                            plateau,
                            rim,
                        } => {
                            for value in [contrast, bias, plateau, rim] {
                                validate_scalar(value)?;
                            }
                        }
                        HeightOperation::Combine { source, amount, .. } => {
                            validate_scalar(source)?;
                            validate_scalar(amount)?;
                        }
                        HeightOperation::Clamp { min, max } => {
                            validate_scalar(min)?;
                            validate_scalar(max)?;
                        }
                        HeightOperation::Remap { .. }
                        | HeightOperation::Terrace { .. }
                        | HeightOperation::Invert => {}
                    }
                }
            }
        }
    }
    for pattern in &recipe.patterns {
        if let Domain::PatternLocal(name) = &pattern.domain {
            require_pattern(name, &patterns)?;
        }
        if let Some(IdSource::Pattern(name)) = &pattern.key {
            require_pattern(name, &patterns)?;
        }
        if let Some(warp) = &pattern.warp {
            validate_scalar(&warp.source)?;
        }
        match &pattern.kind {
            PatternKind::Bricks {
                gap,
                rounding,
                rotation,
                perturb,
                perturb_amount,
                falloff,
                ..
            } => {
                validate_scalar(gap)?;
                validate_scalar(rounding)?;
                validate_scalar(rotation)?;
                if let Some(perturb) = perturb {
                    validate_scalar(perturb)?;
                }
                validate_scalar(perturb_amount)?;
                validate_scalar(falloff)?;
            }
            PatternKind::Discs {
                jitter,
                radius,
                falloff,
                ..
            } => {
                validate_scalar(jitter)?;
                validate_scalar(radius)?;
                validate_scalar(falloff)?;
            }
            PatternKind::Voronoi { .. } => {}
        }
    }
    validate_scalar(&recipe.colorize.source)?;
    validate_scalar(&recipe.output.height)?;
    if recipe.colorize.source != recipe.output.height {
        return Err(ParseError::new(
            line,
            "Colorize.source and Output.height must reference the same field; height is the single source of truth",
        ));
    }
    Ok(())
}

fn validate_material_names_and_references(
    material: &MaterialRecipe,
    line: usize,
) -> Result<(), ParseError> {
    // Reuse the tile definition-graph validation with the material's color source as
    // the synthetic output. Materials deliberately allow every other channel to use
    // a different scalar graph.
    let recipe = Recipe {
        name: material.name.clone(),
        wrap: material.wrap,
        seed: material.seed,
        fields: material.fields.clone(),
        patterns: material.patterns.clone(),
        colorize: material.colorize.clone(),
        output: Output {
            height: material.colorize.source.clone(),
        },
        ..Recipe::default()
    };
    validate_names_and_references(&recipe, line)?;

    let fields = material
        .fields
        .iter()
        .map(|field| field.name().to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let patterns = material
        .patterns
        .iter()
        .map(|pattern| pattern.name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    for source in [
        &material.data.roughness,
        &material.data.metallic,
        &material.data.opacity,
        &material.data.emissive,
        &material.normal.source,
    ] {
        validate_scalar_source(source, &fields, &patterns)?;
    }
    Ok(())
}

fn validate_scalar_source(
    source: &ScalarSource,
    fields: &BTreeSet<String>,
    patterns: &BTreeSet<String>,
) -> Result<(), ParseError> {
    match source {
        ScalarSource::Field(name) => {
            if !fields.contains(&name.to_ascii_lowercase()) {
                return Err(ParseError::new(0, format!("unknown field '{name}'")));
            }
        }
        ScalarSource::Pattern { name, .. } => require_pattern(name, patterns)?,
        ScalarSource::RandomId { id, .. } => {
            if let IdSource::Pattern(name) = id {
                require_pattern(name, patterns)?;
            }
        }
        ScalarSource::Unary { source, .. } => {
            validate_scalar_source(source, fields, patterns)?;
        }
        ScalarSource::Binary { left, right, .. } => {
            validate_scalar_source(left, fields, patterns)?;
            validate_scalar_source(right, fields, patterns)?;
        }
        ScalarSource::Clamp { source, min, max } => {
            validate_scalar_source(source, fields, patterns)?;
            validate_scalar_source(min, fields, patterns)?;
            validate_scalar_source(max, fields, patterns)?;
        }
        ScalarSource::Mix { a, b, factor } => {
            validate_scalar_source(a, fields, patterns)?;
            validate_scalar_source(b, fields, patterns)?;
            validate_scalar_source(factor, fields, patterns)?;
        }
        ScalarSource::Smoothstep { min, max, source } => {
            validate_scalar_source(min, fields, patterns)?;
            validate_scalar_source(max, fields, patterns)?;
            validate_scalar_source(source, fields, patterns)?;
        }
        ScalarSource::Constant(_)
        | ScalarSource::Coordinate(_)
        | ScalarSource::InputHeight
        | ScalarSource::Wave { .. } => {}
    }
    Ok(())
}

fn require_pattern(name: &str, patterns: &BTreeSet<String>) -> Result<(), ParseError> {
    if patterns.contains(&name.to_ascii_lowercase()) {
        Ok(())
    } else {
        Err(ParseError::new(0, format!("unknown pattern '{name}'")))
    }
}

fn parse_scalar_source(value: &str, line: usize) -> Result<ScalarSource, ParseError> {
    let tokens = tokenize_scalar_expression(value, line)?;
    ScalarExpressionParser::new(&tokens, line).parse()
}

#[derive(Clone, Debug, PartialEq)]
enum ScalarToken {
    Number(f32),
    Identifier(String),
    Plus,
    Minus,
    Star,
    Slash,
    LeftParen,
    RightParen,
    Comma,
}

fn tokenize_scalar_expression(value: &str, line: usize) -> Result<Vec<ScalarToken>, ParseError> {
    let bytes = value.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let character = bytes[cursor] as char;
        if character.is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        let token = match character {
            '+' => {
                cursor += 1;
                ScalarToken::Plus
            }
            '-' => {
                cursor += 1;
                ScalarToken::Minus
            }
            '*' => {
                cursor += 1;
                ScalarToken::Star
            }
            '/' => {
                cursor += 1;
                ScalarToken::Slash
            }
            '(' => {
                cursor += 1;
                ScalarToken::LeftParen
            }
            ')' => {
                cursor += 1;
                ScalarToken::RightParen
            }
            ',' => {
                cursor += 1;
                ScalarToken::Comma
            }
            _ if character.is_ascii_digit()
                || (character == '.'
                    && bytes
                        .get(cursor + 1)
                        .is_some_and(|next| (*next as char).is_ascii_digit())) =>
            {
                let start = cursor;
                while bytes
                    .get(cursor)
                    .is_some_and(|next| (*next as char).is_ascii_digit() || *next == b'.')
                {
                    cursor += 1;
                }
                if bytes
                    .get(cursor)
                    .is_some_and(|next| *next == b'e' || *next == b'E')
                {
                    cursor += 1;
                    if bytes
                        .get(cursor)
                        .is_some_and(|next| *next == b'+' || *next == b'-')
                    {
                        cursor += 1;
                    }
                    while bytes
                        .get(cursor)
                        .is_some_and(|next| (*next as char).is_ascii_digit())
                    {
                        cursor += 1;
                    }
                }
                let number = value[start..cursor].parse::<f32>().map_err(|_| {
                    ParseError::new(line, format!("invalid number '{}'", &value[start..cursor]))
                })?;
                ScalarToken::Number(number)
            }
            _ if character.is_ascii_alphabetic() || character == '_' => {
                let start = cursor;
                cursor += 1;
                while bytes.get(cursor).is_some_and(|next| {
                    let next = *next as char;
                    next.is_ascii_alphanumeric() || next == '_' || next == '.'
                }) {
                    cursor += 1;
                }
                ScalarToken::Identifier(value[start..cursor].to_string())
            }
            _ => {
                return Err(ParseError::new(
                    line,
                    format!("unexpected character '{character}' in scalar expression"),
                ));
            }
        };
        tokens.push(token);
    }
    if tokens.is_empty() {
        return Err(ParseError::new(line, "scalar expression is empty"));
    }
    Ok(tokens)
}

struct ScalarExpressionParser<'a> {
    tokens: &'a [ScalarToken],
    cursor: usize,
    line: usize,
}

impl<'a> ScalarExpressionParser<'a> {
    fn new(tokens: &'a [ScalarToken], line: usize) -> Self {
        Self {
            tokens,
            cursor: 0,
            line,
        }
    }

    fn parse(mut self) -> Result<ScalarSource, ParseError> {
        let expression = self.parse_additive()?;
        if let Some(token) = self.peek() {
            return Err(ParseError::new(
                self.line,
                format!("unexpected token {token:?} after scalar expression"),
            ));
        }
        Ok(expression)
    }

    fn parse_additive(&mut self) -> Result<ScalarSource, ParseError> {
        let mut expression = self.parse_multiplicative()?;
        loop {
            let operator = match self.peek() {
                Some(ScalarToken::Plus) => BinaryOperator::Add,
                Some(ScalarToken::Minus) => BinaryOperator::Subtract,
                _ => break,
            };
            self.cursor += 1;
            expression = binary(operator, expression, self.parse_multiplicative()?);
        }
        Ok(expression)
    }

    fn parse_multiplicative(&mut self) -> Result<ScalarSource, ParseError> {
        let mut expression = self.parse_unary()?;
        loop {
            let operator = match self.peek() {
                Some(ScalarToken::Star) => BinaryOperator::Multiply,
                Some(ScalarToken::Slash) => BinaryOperator::Divide,
                _ => break,
            };
            self.cursor += 1;
            expression = binary(operator, expression, self.parse_unary()?);
        }
        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<ScalarSource, ParseError> {
        match self.peek() {
            Some(ScalarToken::Minus) => {
                self.cursor += 1;
                Ok(ScalarSource::Unary {
                    op: UnaryOperator::Negate,
                    source: Box::new(self.parse_unary()?),
                })
            }
            Some(ScalarToken::Plus) => {
                self.cursor += 1;
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<ScalarSource, ParseError> {
        match self.next().cloned() {
            Some(ScalarToken::Number(value)) => Ok(ScalarSource::Constant(value)),
            Some(ScalarToken::Identifier(name)) => {
                if self.consume(&ScalarToken::LeftParen) {
                    self.parse_function(&name)
                } else {
                    scalar_identifier(&name, self.line)
                }
            }
            Some(ScalarToken::LeftParen) => {
                let expression = self.parse_additive()?;
                self.expect(ScalarToken::RightParen, "expected ')'")?;
                Ok(expression)
            }
            token => Err(ParseError::new(
                self.line,
                format!("expected a scalar value, got {token:?}"),
            )),
        }
    }

    fn parse_function(&mut self, name: &str) -> Result<ScalarSource, ParseError> {
        match name.to_ascii_lowercase().as_str() {
            "random" => {
                let id = match self.next().cloned() {
                    Some(ScalarToken::Identifier(value)) => parse_id_source(&value, self.line)?,
                    _ => {
                        return Err(ParseError::new(
                            self.line,
                            "Random first argument must be a stable pattern ID",
                        ));
                    }
                };
                self.expect(ScalarToken::Comma, "Random requires min and max")?;
                let min = literal_expression(self.parse_additive()?, self.line, "Random min")?;
                self.expect(ScalarToken::Comma, "Random requires max")?;
                let max = literal_expression(self.parse_additive()?, self.line, "Random max")?;
                let seed = if self.consume(&ScalarToken::Comma) {
                    let value =
                        literal_expression(self.parse_additive()?, self.line, "Random seed")?;
                    if !value.is_finite() || value < 0.0 || value.fract().abs() > f32::EPSILON {
                        return Err(ParseError::new(
                            self.line,
                            "Random seed must be a non-negative integer",
                        ));
                    }
                    value as u64
                } else {
                    0
                };
                self.expect(ScalarToken::RightParen, "expected ')' after Random")?;
                Ok(ScalarSource::RandomId { id, min, max, seed })
            }
            "wave" => {
                let min = literal_expression(self.parse_additive()?, self.line, "Wave min")?;
                self.expect(ScalarToken::Comma, "Wave requires max and cycles")?;
                let max = literal_expression(self.parse_additive()?, self.line, "Wave max")?;
                self.expect(ScalarToken::Comma, "Wave requires cycles")?;
                let cycles = literal_expression(self.parse_additive()?, self.line, "Wave cycles")?;
                let phase = if self.consume(&ScalarToken::Comma) {
                    literal_expression(self.parse_additive()?, self.line, "Wave phase")?
                } else {
                    0.0
                };
                self.expect(ScalarToken::RightParen, "expected ')' after Wave")?;
                Ok(ScalarSource::Wave {
                    min,
                    max,
                    cycles,
                    phase,
                })
            }
            "abs" | "invert" | "sin" | "cos" | "fract" | "sqrt" => {
                let source = self.parse_additive()?;
                self.expect(ScalarToken::RightParen, "expected ')' after unary function")?;
                let op = match name.to_ascii_lowercase().as_str() {
                    "abs" => UnaryOperator::Abs,
                    "invert" => UnaryOperator::Invert,
                    "sin" => UnaryOperator::Sin,
                    "cos" => UnaryOperator::Cos,
                    "fract" => UnaryOperator::Fract,
                    _ => UnaryOperator::Sqrt,
                };
                Ok(ScalarSource::Unary {
                    op,
                    source: Box::new(source),
                })
            }
            "min" | "max" | "pow" => {
                let left = self.parse_additive()?;
                self.expect(ScalarToken::Comma, "function requires two arguments")?;
                let right = self.parse_additive()?;
                self.expect(ScalarToken::RightParen, "expected ')' after function")?;
                let operator = match name.to_ascii_lowercase().as_str() {
                    "min" => BinaryOperator::Min,
                    "max" => BinaryOperator::Max,
                    _ => BinaryOperator::Pow,
                };
                Ok(binary(operator, left, right))
            }
            "clamp" => {
                let source = self.parse_additive()?;
                self.expect(ScalarToken::Comma, "Clamp requires min and max")?;
                let min = self.parse_additive()?;
                self.expect(ScalarToken::Comma, "Clamp requires max")?;
                let max = self.parse_additive()?;
                self.expect(ScalarToken::RightParen, "expected ')' after Clamp")?;
                Ok(ScalarSource::Clamp {
                    source: Box::new(source),
                    min: Box::new(min),
                    max: Box::new(max),
                })
            }
            "mix" => {
                let a = self.parse_additive()?;
                self.expect(ScalarToken::Comma, "Mix requires three arguments")?;
                let b = self.parse_additive()?;
                self.expect(ScalarToken::Comma, "Mix requires a factor")?;
                let factor = self.parse_additive()?;
                self.expect(ScalarToken::RightParen, "expected ')' after Mix")?;
                Ok(ScalarSource::Mix {
                    a: Box::new(a),
                    b: Box::new(b),
                    factor: Box::new(factor),
                })
            }
            "smoothstep" => {
                let min = self.parse_additive()?;
                self.expect(ScalarToken::Comma, "Smoothstep requires max and source")?;
                let max = self.parse_additive()?;
                self.expect(ScalarToken::Comma, "Smoothstep requires a source")?;
                let source = self.parse_additive()?;
                self.expect(ScalarToken::RightParen, "expected ')' after Smoothstep")?;
                Ok(ScalarSource::Smoothstep {
                    min: Box::new(min),
                    max: Box::new(max),
                    source: Box::new(source),
                })
            }
            _ => Err(ParseError::new(
                self.line,
                format!("unknown scalar function '{name}'"),
            )),
        }
    }

    fn peek(&self) -> Option<&ScalarToken> {
        self.tokens.get(self.cursor)
    }

    fn next(&mut self) -> Option<&ScalarToken> {
        let token = self.tokens.get(self.cursor);
        self.cursor += usize::from(token.is_some());
        token
    }

    fn consume(&mut self, expected: &ScalarToken) -> bool {
        if self.peek() == Some(expected) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: ScalarToken, message: &str) -> Result<(), ParseError> {
        if self.consume(&expected) {
            Ok(())
        } else {
            Err(ParseError::new(self.line, message))
        }
    }
}

fn binary(operator: BinaryOperator, left: ScalarSource, right: ScalarSource) -> ScalarSource {
    ScalarSource::Binary {
        op: operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn scalar_identifier(value: &str, line: usize) -> Result<ScalarSource, ParseError> {
    let coordinate = match value.to_ascii_lowercase().as_str() {
        "u" => Some(CoordinateChannel::U),
        "v" => Some(CoordinateChannel::V),
        "radius" => Some(CoordinateChannel::Radius),
        "angle" => Some(CoordinateChannel::Angle),
        _ => None,
    };
    if let Some(coordinate) = coordinate {
        return Ok(ScalarSource::Coordinate(coordinate));
    }
    if value.eq_ignore_ascii_case("Input.height") {
        return Ok(ScalarSource::InputHeight);
    }
    if let Some((name, channel)) = value.rsplit_once('.') {
        validate_identifier(name, line)?;
        let channel = match channel.to_ascii_lowercase().as_str() {
            "height" => PatternChannel::Height,
            "edge" => PatternChannel::Edge,
            "center" => PatternChannel::Center,
            "id" => {
                return Err(ParseError::new(
                    line,
                    "pattern IDs are not scalar fields; use Random(Pattern.id, min, max)",
                ));
            }
            _ => {
                return Err(ParseError::new(
                    line,
                    format!("unknown pattern channel '{channel}'"),
                ));
            }
        };
        Ok(ScalarSource::Pattern {
            name: name.to_string(),
            channel,
        })
    } else {
        validate_identifier(value, line)?;
        Ok(ScalarSource::Field(value.to_string()))
    }
}

fn literal_expression(
    expression: ScalarSource,
    line: usize,
    label: &str,
) -> Result<f32, ParseError> {
    match expression {
        ScalarSource::Constant(value) => Ok(value),
        ScalarSource::Unary {
            op: UnaryOperator::Negate,
            source,
        } => literal_expression(*source, line, label).map(|value| -value),
        _ => Err(ParseError::new(
            line,
            format!("{label} must be a literal number"),
        )),
    }
}

fn parse_id_source(value: &str, line: usize) -> Result<IdSource, ParseError> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("Id") || value.eq_ignore_ascii_case("Current.id") {
        return Ok(IdSource::Current);
    }
    let (name, channel) = value.rsplit_once('.').ok_or_else(|| {
        ParseError::new(line, "expected Id or a stable pattern ID such as Stones.id")
    })?;
    validate_identifier(name, line)?;
    if !channel.eq_ignore_ascii_case("id") {
        return Err(ParseError::new(
            line,
            "stable ID references must end in .id",
        ));
    }
    Ok(IdSource::Pattern(name.to_string()))
}

fn parse_domain(value: &str, line: usize) -> Result<Domain, ParseError> {
    if value.trim().eq_ignore_ascii_case("Global") {
        return Ok(Domain::Global);
    }
    let (name, channel) = value.trim().rsplit_once('.').ok_or_else(|| {
        ParseError::new(
            line,
            "space must be Global or a pattern local space such as Stones.local",
        )
    })?;
    validate_identifier(name, line)?;
    if !channel.eq_ignore_ascii_case("local") {
        return Err(ParseError::new(
            line,
            "pattern-local space references must end in .local",
        ));
    }
    Ok(Domain::PatternLocal(name.to_string()))
}

fn parse_color_range(value: &str, line: usize) -> Result<ColorRange, ParseError> {
    if value.trim().eq_ignore_ascii_case("Auto") {
        Ok(ColorRange::Auto)
    } else {
        parse_f2(value, line).map(ColorRange::Fixed)
    }
}

fn parse_family(value: &str, line: usize) -> Result<ColorFamily, ParseError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "any" => Ok(ColorFamily::Any),
        "neutral" => Ok(ColorFamily::Neutral),
        "warm" => Ok(ColorFamily::Warm),
        "cool" => Ok(ColorFamily::Cool),
        "earth" => Ok(ColorFamily::Earth),
        "red" => Ok(ColorFamily::Red),
        "orange" => Ok(ColorFamily::Orange),
        "yellow" => Ok(ColorFamily::Yellow),
        "green" => Ok(ColorFamily::Green),
        "cyan" => Ok(ColorFamily::Cyan),
        "blue" => Ok(ColorFamily::Blue),
        "purple" => Ok(ColorFamily::Purple),
        "magenta" => Ok(ColorFamily::Magenta),
        _ => Err(ParseError::new(
            line,
            format!("unknown palette family '{value}'"),
        )),
    }
}

fn parse_noise_kind(value: &str, line: usize) -> Result<NoiseKind, ParseError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "value" => Ok(NoiseKind::Value),
        "gradient" | "perlin" => Ok(NoiseKind::Gradient),
        _ => Err(ParseError::new(
            line,
            "noise type must be Value or Gradient",
        )),
    }
}

fn parse_fractal_kind(value: &str, line: usize) -> Result<FractalKind, ParseError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "fbm" => Ok(FractalKind::Fbm),
        "ridged" => Ok(FractalKind::Ridged),
        "billow" => Ok(FractalKind::Billow),
        "turbulence" => Ok(FractalKind::Turbulence),
        _ => Err(ParseError::new(
            line,
            "noise fractal must be FBm, Ridged, Billow, or Turbulence",
        )),
    }
}

fn parse_base_color(value: &str, line: usize) -> Result<[u8; 4], ParseError> {
    let value = value.trim().trim_matches('"');
    if let Some(hex) = value.strip_prefix('#') {
        if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ParseError::new(
                line,
                "base color must be #RRGGBB, #RRGGBBAA, or a named color",
            ));
        }
        let component = |start| u8::from_str_radix(&hex[start..start + 2], 16).unwrap();
        return Ok([
            component(0),
            component(2),
            component(4),
            if hex.len() == 8 { component(6) } else { 255 },
        ]);
    }

    let normalized = value
        .chars()
        .filter(|character| !matches!(character, ' ' | '_' | '-'))
        .flat_map(char::to_lowercase)
        .collect::<String>();
    let rgb = match normalized.as_str() {
        "black" => [12, 12, 14],
        "charcoal" => [38, 40, 43],
        "darkgray" | "darkgrey" => [55, 57, 60],
        "gray" | "grey" => [118, 120, 122],
        "lightgray" | "lightgrey" => [190, 192, 194],
        "white" => [238, 238, 234],
        "darkbrown" => [58, 36, 24],
        "brown" => [105, 67, 40],
        "lightbrown" => [166, 124, 82],
        "darkred" => [92, 30, 30],
        "red" => [180, 45, 42],
        "orange" => [205, 105, 35],
        "gold" => [190, 145, 38],
        "yellow" => [220, 202, 70],
        "olive" => [105, 105, 42],
        "darkgreen" => [30, 72, 42],
        "green" => [55, 135, 65],
        "teal" => [35, 112, 105],
        "cyan" => [52, 165, 175],
        "darkblue" => [30, 45, 90],
        "blue" => [50, 88, 175],
        "purple" => [105, 62, 145],
        "magenta" => [165, 55, 135],
        _ => {
            return Err(ParseError::new(
                line,
                format!(
                    "unknown base color '{value}'; use #RRGGBB or a standard name such as DarkBrown"
                ),
            ));
        }
    };
    Ok([rgb[0], rgb[1], rgb[2], 255])
}

fn parse_wrap(node: &Node) -> Result<WrapMode, ParseError> {
    let Some((value, line)) = field(node, "wrap") else {
        return Ok(WrapMode::Repeat);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "clamp" => Ok(WrapMode::Clamp),
        "repeat" => Ok(WrapMode::Repeat),
        "mirror" => Ok(WrapMode::Mirror),
        _ => Err(ParseError::new(
            line,
            "wrap must be Clamp, Repeat, or Mirror",
        )),
    }
}

fn required_scalar(node: &Node, name: &str) -> Result<ScalarSource, ParseError> {
    let (value, line) = field(node, name)
        .ok_or_else(|| ParseError::new(node.line, format!("{} requires {name}", node.name)))?;
    parse_scalar_source(value, line)
}

fn optional_scalar(node: &Node, name: &str, default: f32) -> Result<ScalarSource, ParseError> {
    field(node, name)
        .map(|(value, line)| parse_scalar_source(value, line))
        .unwrap_or(Ok(ScalarSource::Constant(default)))
}

fn optional_scalar_alias(
    node: &Node,
    canonical: &str,
    alias: &str,
    default: f32,
) -> Result<ScalarSource, ParseError> {
    field(node, canonical)
        .or_else(|| field(node, alias))
        .map(|(value, line)| parse_scalar_source(value, line))
        .unwrap_or(Ok(ScalarSource::Constant(default)))
}

fn reject_alias_pair(node: &Node, canonical: &str, alias: &str) -> Result<(), ParseError> {
    if field(node, canonical).is_some() && field(node, alias).is_some() {
        Err(ParseError::new(
            field(node, alias).unwrap().1,
            format!("{canonical} and {alias} are aliases; specify only one"),
        ))
    } else {
        Ok(())
    }
}

fn optional_i2(node: &Node, name: &str, default: [u32; 2]) -> Result<[u32; 2], ParseError> {
    field(node, name)
        .map(|(value, line)| parse_i2(value, line))
        .unwrap_or(Ok(default))
}

fn optional_f2(node: &Node, name: &str, default: [f32; 2]) -> Result<[f32; 2], ParseError> {
    field(node, name)
        .map(|(value, line)| parse_f2(value, line))
        .unwrap_or(Ok(default))
}

fn optional_positive_f2(
    node: &Node,
    name: &str,
    default: [f32; 2],
) -> Result<[f32; 2], ParseError> {
    let result = match field(node, name) {
        Some((value, line)) => {
            let (call, _) = parse_call(value, line)?;
            if call.eq_ignore_ascii_case("I2") {
                parse_i2(value, line)?.map(|component| component as f32)
            } else {
                parse_f2(value, line)?
            }
        }
        None => default,
    };
    if result
        .iter()
        .any(|component| !component.is_finite() || *component <= 0.0)
    {
        return Err(ParseError::new(
            field(node, name).map(|(_, line)| line).unwrap_or(node.line),
            format!("{name} values must be finite and greater than zero"),
        ));
    }
    Ok(result)
}

fn parse_i2(value: &str, line: usize) -> Result<[u32; 2], ParseError> {
    let (call, arguments) = parse_call(value, line)?;
    if !call.eq_ignore_ascii_case("I2") {
        return Err(ParseError::new(line, "expected I2(x, y)"));
    }
    let args = split_arguments(arguments);
    if args.len() != 2 {
        return Err(ParseError::new(line, "I2 requires two positive integers"));
    }
    let result = [parse_u32(args[0], line)?, parse_u32(args[1], line)?];
    if result.contains(&0) {
        return Err(ParseError::new(line, "I2 values must be greater than zero"));
    }
    Ok(result)
}

fn parse_f2(value: &str, line: usize) -> Result<[f32; 2], ParseError> {
    let (call, arguments) = parse_call(value, line)?;
    if !call.eq_ignore_ascii_case("F2") {
        return Err(ParseError::new(line, "expected F2(x, y)"));
    }
    let args = split_arguments(arguments);
    if args.len() != 2 {
        return Err(ParseError::new(line, "F2 requires two numbers"));
    }
    Ok([parse_number(args[0], line)?, parse_number(args[1], line)?])
}

fn parse_call(value: &str, line: usize) -> Result<(&str, &str), ParseError> {
    let open = value.find('(').ok_or_else(|| {
        ParseError::new(line, format!("expected function expression, got '{value}'"))
    })?;
    if !value.ends_with(')') {
        return Err(ParseError::new(line, "function expression is missing ')'"));
    }
    Ok((value[..open].trim(), &value[open + 1..value.len() - 1]))
}

fn split_arguments(value: &str) -> Vec<&str> {
    let mut arguments = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                arguments.push(value[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if !value[start..].trim().is_empty() {
        arguments.push(value[start..].trim());
    }
    arguments
}

fn validate_identifier(value: &str, line: usize) -> Result<(), ParseError> {
    let mut characters = value.chars();
    let valid_first = characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_');
    if !valid_first
        || !characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(ParseError::new(
            line,
            format!("'{value}' is not a valid identifier"),
        ));
    }
    Ok(())
}

fn parse_string(value: &str, line: usize) -> Result<String, ParseError> {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        Ok(value[1..value.len() - 1].to_string())
    } else {
        Err(ParseError::new(line, "expected a quoted string"))
    }
}

fn parse_alias(value: &str, line: usize) -> Result<String, ParseError> {
    let value = value.trim();
    let value = if value.starts_with('"') || value.ends_with('"') {
        parse_string(value, line)?
    } else {
        value.to_string()
    };
    let normalized = value.replace('\\', "/").trim_matches('/').to_string();
    if normalized.is_empty()
        || normalized.split('/').any(|part| {
            part.is_empty()
                || !part.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
                })
        })
    {
        return Err(ParseError::new(
            line,
            "material must be an alias such as dungeon/wall_stone",
        ));
    }
    Ok(normalized)
}

fn parse_number(value: &str, line: usize) -> Result<f32, ParseError> {
    value
        .trim()
        .parse::<f32>()
        .map_err(|_| ParseError::new(line, format!("'{value}' is not a number")))
}

fn parse_u32(value: &str, line: usize) -> Result<u32, ParseError> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| ParseError::new(line, format!("'{value}' is not a positive integer")))
}

fn parse_u64(value: &str, line: usize) -> Result<u64, ParseError> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| ParseError::new(line, format!("'{value}' is not a positive integer")))
}

fn optional_u32(node: &Node, name: &str, default: u32) -> Result<u32, ParseError> {
    field(node, name)
        .map(|(value, line)| parse_u32(value, line))
        .unwrap_or(Ok(default))
}

fn optional_u64(node: &Node, name: &str, default: u64) -> Result<u64, ParseError> {
    field(node, name)
        .map(|(value, line)| parse_u64(value, line))
        .unwrap_or(Ok(default))
}

fn optional_f32(node: &Node, name: &str, default: f32) -> Result<f32, ParseError> {
    field(node, name)
        .map(|(value, line)| parse_number(value, line))
        .unwrap_or(Ok(default))
}

fn optional_bool(node: &Node, name: &str, default: bool) -> Result<bool, ParseError> {
    field(node, name)
        .map(
            |(value, line)| match value.trim().to_ascii_lowercase().as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(ParseError::new(
                    line,
                    format!("{name} must be true or false"),
                )),
            },
        )
        .unwrap_or(Ok(default))
}

fn field<'a>(node: &'a Node, name: &str) -> Option<(&'a str, usize)> {
    node.fields
        .get(name)
        .map(|(value, line)| (value.as_str(), *line))
}

fn reject_unknown_fields(node: &Node, allowed: &[&str]) -> Result<(), ParseError> {
    for (key, (_, line)) in &node.fields {
        if !allowed.contains(&key.as_str()) {
            return Err(ParseError::new(
                *line,
                format!("unknown field '{key}' in {} block", node.name),
            ));
        }
    }
    Ok(())
}

fn reject_children(node: &Node) -> Result<(), ParseError> {
    if let Some(child) = node.children.first() {
        Err(ParseError::new(
            child.line,
            format!("{} does not accept nested blocks", node.name),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_height_first_recipe_and_typed_references() {
        let recipe = parse_recipe(include_str!("../examples/bricks.recipe")).unwrap();
        assert_eq!(recipe.patterns.len(), 2);
        assert_eq!(recipe.fields.len(), 3);
        assert!(matches!(
            &recipe.patterns[0].kind,
            PatternKind::Discs { .. }
        ));
        assert!(matches!(
            &recipe.patterns[1].kind,
            PatternKind::Bricks {
                rounding: ScalarSource::Binary { .. },
                perturb: Some(ScalarSource::Binary { .. }),
                ..
            }
        ));
        assert!(matches!(recipe.colorize.range, ColorRange::Auto));
    }

    #[test]
    fn rejects_pattern_id_as_an_untyped_scalar() {
        let error = parse_recipe(
            r#"
Tile
    Pattern Wall
        Bricks

    Height Surface
        source = Wall.id

    Colorize
        source = Surface

    Output
        height = Surface
"#,
        )
        .unwrap_err();
        assert!(error.message.contains("not scalar"));
    }

    #[test]
    fn rejects_unknown_references() {
        let error = parse_recipe(
            r#"
Tile
    Height Surface
        source = Missing.height

    Colorize
        source = Surface

    Output
        height = Surface
"#,
        )
        .unwrap_err();
        assert!(error.message.contains("unknown pattern"));
    }

    #[test]
    fn rejects_a_color_source_different_from_output_height() {
        let error = parse_recipe(
            r#"
Tile
    Height Surface
        source = 0.5

    Height Other
        source = 0.2

    Colorize
        source = Other

    Output
        height = Surface
"#,
        )
        .unwrap_err();
        assert!(error.message.contains("single source of truth"));
    }

    #[test]
    fn parses_named_and_hex_anchored_color_ramps() {
        let named = parse_recipe(
            r#"
Tile
    Height Surface
        source = 0.5

    Colorize
        source = Surface
        base = DarkBrown
        brightness = F2(-0.18, 0.22)
        saturation = F2(-0.08, 0.06)
        hue = F2(-0.01, 0.01)

    Output
        height = Surface
"#,
        )
        .unwrap();
        assert_eq!(named.colorize.base, Some([58, 36, 24, 255]));
        assert_eq!(named.colorize.brightness, [-0.18, 0.22]);

        let hex = parse_base_color("\"#3A2418CC\"", 1).unwrap();
        assert_eq!(hex, [58, 36, 24, 204]);
    }

    #[test]
    fn rejects_mixed_colorize_modes() {
        let error = parse_recipe(
            r#"
Tile
    Height Surface
        source = 0.5

    Colorize
        source = Surface
        base = DarkBrown
        ramp = Earth

    Output
        height = Surface
"#,
        )
        .unwrap_err();
        assert!(error.message.contains("cannot be combined"));
    }

    #[test]
    fn scalar_expressions_preserve_operator_precedence() {
        let expression = parse_scalar_source("1 + 2 * 3", 1).unwrap();
        assert!(matches!(
            expression,
            ScalarSource::Binary {
                op: BinaryOperator::Add,
                right,
                ..
            } if matches!(
                *right,
                ScalarSource::Binary {
                    op: BinaryOperator::Multiply,
                    ..
                }
            )
        ));
    }

    #[test]
    fn parses_multiple_materials_and_generic_surface_primitives() {
        let document = parse_material_document(
            r#"
Material old_oak
    name = "Old Oak"

    Noise Grain
        type = Gradient
        fractal = Ridged
        scale = F2(12.0, 3.0)

    Height Tone
        source = Abs(Sin(V * 6.0 + U + Grain))

    Colorize
        source = Tone
        base = DarkBrown

    MaterialData
        roughness = Clamp(0.4 + Grain * 0.4, 0.0, 1.0)
        metal = 0.0
        opacity = 1.0
        emission = 0.0

    Normal
        source = Input.height + Tone * 0.1
        strength = 0.25

Material marble
    Colorize
        source = Input.height
        base = LightGray
"#,
        )
        .unwrap();

        assert_eq!(document.materials.len(), 2);
        assert_eq!(document.materials[0].id, "old_oak");
        assert!(matches!(
            document.materials[0].fields[0],
            FieldDefinition::Noise(NoiseField {
                kind: NoiseKind::Gradient,
                fractal: FractalKind::Ridged,
                ..
            })
        ));
        assert_eq!(document.materials[1].normal, MaterialNormal::default());
    }

    #[test]
    fn tile_material_reference_makes_inline_colorize_optional() {
        let recipe = parse_recipe(
            r#"
Tile
    material = dungeon/wall_stone

    Height Surface
        source = 0.5

    Output
        height = Surface
"#,
        )
        .unwrap();
        assert_eq!(recipe.material.as_deref(), Some("dungeon/wall_stone"));
        assert_eq!(recipe.colorize.source, recipe.output.height);
    }

    #[test]
    fn rejects_duplicate_material_ids() {
        let error = parse_material_document(
            r#"
Material stone
    Colorize
        source = Input.height
        base = Gray

Material stone
    Colorize
        source = Input.height
        base = Gray
"#,
        )
        .unwrap_err();
        assert!(error.message.contains("duplicate material id"));
    }

    #[test]
    fn unified_recipe_documents_classify_tiles_and_materials() {
        assert!(matches!(
            parse_document(
                "Tile\n    Colorize\n        source = 0.5\n    Output\n        height = 0.5\n"
            )
            .unwrap(),
            RecipeDocument::Tile(_)
        ));
        assert!(matches!(
            parse_document(
                "Material stone\n    Colorize\n        source = Input.height\n        base = Gray\n"
            )
            .unwrap(),
            RecipeDocument::Materials(_)
        ));
    }

    #[test]
    fn parses_base_only_palette_mode_and_requires_a_base() {
        let recipe = parse_recipe(
            r#"
Tile
    Height Surface
        source = U

    Colorize
        source = Surface
        palette = BaseOnly
        base = DarkBrown
        steps = 128

    Output
        height = Surface
"#,
        )
        .unwrap();
        assert_eq!(recipe.colorize.palette, PaletteMode::BaseOnly);
        assert_eq!(recipe.colorize.steps, 128);

        let error = parse_recipe(
            r#"
Tile
    Height Surface
        source = U

    Colorize
        source = Surface
        palette = BaseOnly

    Output
        height = Surface
"#,
        )
        .unwrap_err();
        assert!(error.message.contains("requires Colorize.base"));
    }
}
