//! Versioned, controlled-language project syntax. IDs are separate from labels.
use crate::{Error, Result};
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub result: Option<String>,
    pub materials: Vec<Material>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub stack_slot: Option<usize>,
    pub locked: bool,
    pub tiling: crate::periodic::Tiling,
    pub animation: Animation,
    pub mixing: Option<MaterialMix>,
    pub defaults: Vec<(String, Vec<f32>)>,
    pub position: Option<[f32; 2]>,
    pub mode: String,
    pub id: String,
    pub label: String,
    pub rows: Vec<Row>,
    pub outputs: Vec<Output>,
}
/// Exports sample the normalized time interval [0,1); duration controls playback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Animation {
    pub frames: u32,
    pub duration: f32,
}
impl Default for Animation {
    fn default() -> Self {
        Self {
            frames: 1,
            duration: 2.,
        }
    }
}
impl Animation {
    pub fn validate(self) -> Result<Self> {
        if !(1..=32).contains(&self.frames)
            || !self.duration.is_finite()
            || !(0.25..=60.).contains(&self.duration)
        {
            return Err(Error(
                "Animation requires 1–32 frames and a duration of 0.25–60 seconds".into(),
            ));
        }
        Ok(self)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialMix {
    pub mask_material: Option<String>,
    pub kind: String,
    pub inputs: [Option<String>; 4],
    pub mask: Binding,
    pub blend: bool,
    pub direction: usize,
    pub levels: [f32; 3],
}
impl MaterialMix {
    pub fn new(kind: &str) -> Result<Self> {
        if !matches!(kind, "Mask" | "Bump Height" | "Overlay" | "UV Position") {
            return Err(Error(format!("Unknown material mixer {kind:?}")));
        }
        Ok(Self {
            mask_material: None,
            kind: kind.into(),
            inputs: Default::default(),
            mask: Binding::Constant(vec![0.5; 3]),
            blend: true,
            direction: 0,
            levels: [if kind == "Overlay" { 0.0 } else { 0.25 }, 0.5, 0.75],
        })
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub id: String,
    pub label: String,
    pub tiles: Vec<Tile>,
    pub coordinates: Option<Reference>,
    /// Row-owned connections survive removing or replacing their receiving tile.
    /// A connection is dormant until a coordinate, image or parameter binding uses it.
    pub incoming: Vec<RowConnection>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct RowConnection {
    /// Stable within the owning row; independent of a receiving tile.
    pub id: String,
    pub source: Reference,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    /// Opaque document-unique identity; independent of row membership and order.
    pub id: String,
    pub input_disconnected: bool,
    pub code: Option<String>,
    pub image: Option<crate::image_source::ImageSource>,
    pub kernel: String,
    pub parameters: Vec<(String, Binding)>,
    pub input: Option<Reference>,
    pub combine: Combine,
}
/// Generate an identity for a newly authored or duplicated tile.
pub fn new_tile_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "tile-{time:x}-{:x}-{:x}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Combine {
    #[default]
    Replace,
    Union,
    Subtract,
    Intersect,
    SmoothUnion,
}
impl Combine {
    pub fn name(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Union => "union",
            Self::Subtract => "subtract",
            Self::Intersect => "intersect",
            Self::SmoothUnion => "smooth union",
        }
    }
    fn parse(s: &str) -> Result<Self> {
        match s {
            "replace" => Ok(Self::Replace),
            "union" => Ok(Self::Union),
            "subtract" => Ok(Self::Subtract),
            "intersect" => Ok(Self::Intersect),
            "smooth union" => Ok(Self::SmoothUnion),
            _ => Err(Error(format!("Unknown shape combination {s:?}"))),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub row: String,
    pub components: String,
    pub mapping: Option<(f32, f32)>,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    Constant(Vec<f32>),
    From(Reference),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub channel: String,
    pub source: Reference,
}

pub(crate) fn tokens(line: &str) -> Result<Vec<String>> {
    let mut chars = line.chars().peekable();
    let mut result = vec![];
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if c == '#' {
            break;
        }
        let mut token = String::new();
        if c == '"' {
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '"' {
                    closed = true;
                    break;
                }
                if c == '\\' {
                    let escaped = chars
                        .next()
                        .ok_or_else(|| Error("Unfinished escape".into()))?;
                    token.push(match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        _ => {
                            return Err(Error(
                                "Only quote and backslash escapes are supported".into(),
                            ));
                        }
                    });
                } else {
                    token.push(c);
                }
            }
            if !closed {
                return Err(Error("Unterminated quoted string".into()));
            }
            if chars
                .peek()
                .is_some_and(|c| !c.is_whitespace() && *c != '#')
            {
                return Err(Error("Expected space after quoted string".into()));
            }
        } else {
            token.push(c);
            while chars
                .peek()
                .is_some_and(|c| !c.is_whitespace() && *c != '#')
            {
                token.push(chars.next().unwrap());
            }
        }
        result.push(token);
    }
    Ok(result)
}
fn number(s: &str) -> Result<f32> {
    s.parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| Error(format!("Expected finite number, got {s:?}")))
}
fn reference(t: &[String]) -> Result<Reference> {
    if t.len() < 3 || t[0] != "row" {
        return Err(Error(
            "Expected row \"id\" followed by x, xy, xyz or another component selection".into(),
        ));
    }
    let mapping = match &t[3..] {
        [] => None,
        [mapped, lo, to, hi] if mapped == "mapped" && to == "to" => {
            Some((number(lo)?, number(hi)?))
        }
        _ => return Err(Error("Expected mapped <minimum> to <maximum>".into())),
    };
    Ok(Reference {
        row: t[1].clone(),
        components: t[2].clone(),
        mapping,
    })
}

pub fn parse(source: &str) -> Result<Project> {
    let mut p = Project {
        materials: vec![],
        result: None,
    };
    let mut version = false;
    let mut previous_indent = 0;
    let mut row_open = false;
    let mut tile_open = false;
    let mut combine_seen = false;
    let mut mode_seen = false;
    let mut code_block: Option<String> = None;
    for (i, raw) in source.lines().enumerate() {
        if let Some(code) = &mut code_block {
            if raw == "      end source" {
                p.materials
                    .last_mut()
                    .unwrap()
                    .rows
                    .last_mut()
                    .unwrap()
                    .tiles
                    .last_mut()
                    .unwrap()
                    .code = code_block.take();
                previous_indent = 6;
                continue;
            }
            if !raw.is_empty() && !raw.starts_with("        ") {
                return Err(Error(format!(
                    "line {}: Source block needs eight-space indentation and end source",
                    i + 1
                )));
            }
            code.push_str(raw.strip_prefix("        ").unwrap_or(raw));
            code.push('\n');
            continue;
        }
        let line_number = i + 1;
        let parsed = (|| -> Result<()> {
            if raw.trim().is_empty() || raw.trim_start().starts_with('#') {
                return Ok(());
            }
            if raw.contains('\t') {
                return Err(Error("Use spaces, not tabs".into()));
            }
            let indent = raw.len() - raw.trim_start_matches(' ').len();
            let t = tokens(raw.trim())?;
            if t.is_empty() {
                return Ok(());
            }
            if !version {
                if indent != 0 || t != ["project", "1"] {
                    return Err(Error("First declaration must be project 1".into()));
                }
                version = true;
                return Ok(());
            }
            if indent > previous_indent + 2 {
                return Err(Error("Indentation jumps over a level".into()));
            }
            previous_indent = indent;
            match indent {
                0 => {
                    row_open = false;
                    tile_open = false;
                    mode_seen = false;
                    if t.len() == 4 && t[0] == "result" && t[1] == "from" && t[2] == "material" {
                        if p.result.is_some() {
                            return Err(Error("Only one material result connection".into()));
                        }
                        p.result = Some(t[3].clone());
                        return Ok(());
                    }
                    if t.len() != 4 || t[0] != "material" || t[2] != "called" {
                        return Err(Error(
                            "Expected material \"stable-id\" called \"Label\"".into(),
                        ));
                    }
                    p.materials.push(Material {
                        stack_slot: None,
                        locked: false,
                        tiling: Default::default(),
                        animation: Default::default(),
                        mixing: None,
                        id: t[1].clone(),
                        label: t[3].clone(),
                        mode: "bsdf".into(),
                        defaults: vec![],
                        position: None,
                        rows: vec![],
                        outputs: vec![],
                    });
                }
                2 => {
                    let m = p
                        .materials
                        .last_mut()
                        .ok_or_else(|| Error("Declaration outside material".into()))?;
                    if t[0] == "animation" && t.len() == 3 {
                        m.animation = Animation {
                            frames: t[1]
                                .parse()
                                .map_err(|_| Error("Animation frames must be an integer".into()))?,
                            duration: number(&t[2])?,
                        }
                        .validate()?;
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "tiling" && t.len() == 2 {
                        m.tiling.mode = crate::periodic::Mode::parse(&t[1])?;
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "period" && t.len() == 3 {
                        m.tiling.period = [number(&t[1])?, number(&t[2])?];
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "seam" && t.len() == 3 && t[1] == "blend" {
                        m.tiling.blend = number(&t[2])?;
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "mix" && t.len() == 2 {
                        if m.mixing.is_some() {
                            return Err(Error("Only one mixer per material".into()));
                        }
                        m.mixing = Some(MaterialMix::new(&t[1])?);
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "mixing" {
                        let mix = m
                            .mixing
                            .as_mut()
                            .ok_or_else(|| Error("Declare mix before its settings".into()))?;
                        match t.get(1).map(String::as_str) {
                            Some("mask-material") if t.len() == 3 => {
                                mix.mask_material = Some(t[2].clone())
                            }
                            Some("input")
                                if t.len() == 6 && t[3] == "from" && t[4] == "material" =>
                            {
                                let slot = t[2]
                                    .parse::<usize>()
                                    .ok()
                                    .filter(|i| (1..=4).contains(i))
                                    .ok_or_else(|| Error("Material input must be 1..4".into()))?;
                                if mix.inputs[slot - 1].is_some() {
                                    return Err(Error("Duplicate material input".into()));
                                }
                                mix.inputs[slot - 1] = Some(t[5].clone());
                            }
                            Some("mask") if t.get(2).is_some_and(|s| s == "from") => {
                                mix.mask = Binding::From(reference(&t[3..])?)
                            }
                            Some("mask") if t.len() == 6 && t[2] == "to" => {
                                mix.mask = Binding::Constant(
                                    t[3..]
                                        .iter()
                                        .map(|s| number(s))
                                        .collect::<Result<Vec<_>>>()?,
                                )
                            }
                            Some("blend")
                                if t.len() == 3 && matches!(t[2].as_str(), "true" | "false") =>
                            {
                                mix.blend = t[2] == "true"
                            }
                            Some("direction")
                                if t.len() == 3 && matches!(t[2].as_str(), "x" | "y") =>
                            {
                                mix.direction = usize::from(t[2] == "y")
                            }
                            Some("levels") if t.len() == 5 => {
                                mix.levels = [number(&t[2])?, number(&t[3])?, number(&t[4])?]
                            }
                            _ => return Err(Error("Invalid material mixing setting".into())),
                        }
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "mode" && t.len() == 2 {
                        if mode_seen || !matches!(t[1].as_str(), "art" | "pbr" | "bsdf") {
                            return Err(Error(
                                "Expected one mode art / pbr / bsdf declaration".into(),
                            ));
                        }
                        m.mode = t[1].clone();
                        mode_seen = true;
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "locked" && t.len() == 2 {
                        m.locked = match t[1].as_str() {
                            "true" => true,
                            "false" => false,
                            _ => return Err(Error("Expected locked true / false".into())),
                        };
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "stack" && t.len() == 3 && t[1] == "slot" {
                        let slot = t[2]
                            .parse::<usize>()
                            .map_err(|_| Error("Expected stack slot 1..4".into()))?;
                        if !(1..=4).contains(&slot) || m.stack_slot.is_some() {
                            return Err(Error("Expected one stack slot 1..4".into()));
                        }
                        m.stack_slot = Some(slot - 1);
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "position" {
                        if t.len() != 3 || m.position.is_some() {
                            return Err(Error("Expected one position x y declaration".into()));
                        }
                        let x = number(&t[1])?;
                        let y = number(&t[2])?;
                        if x < 0.0 || y < 0.0 {
                            return Err(Error("Material position must be nonnegative".into()));
                        }
                        m.position = Some([x, y]);
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "default" {
                        let to = t
                            .iter()
                            .position(|s| s == "to")
                            .ok_or_else(|| Error("Default needs channel to value".into()))?;
                        if to < 2 || to + 1 >= t.len() {
                            return Err(Error("Default needs channel to value".into()));
                        }
                        let name = t[1..to].join(" ");
                        if m.defaults.iter().any(|(id, _)| id == &name) {
                            return Err(Error(format!("Duplicate material default {name:?}")));
                        }
                        let values = t[to + 1..]
                            .iter()
                            .map(|v| number(v))
                            .collect::<Result<Vec<_>>>()?;
                        m.defaults.push((name, values));
                        row_open = false;
                        tile_open = false;
                    } else if t[0] == "row" && (t.len() == 2 || (t.len() == 4 && t[2] == "called"))
                    {
                        row_open = true;
                        tile_open = false;
                        m.rows.push(Row {
                            id: t[1].clone(),
                            label: t.get(3).cloned().unwrap_or_default(),
                            tiles: vec![],
                            coordinates: None,
                            incoming: vec![],
                        });
                    } else if t[0] == "output" {
                        row_open = false;
                        tile_open = false;
                        let from = t
                            .iter()
                            .position(|s| s == "from")
                            .ok_or_else(|| Error("Output needs from row".into()))?;
                        if from < 2 {
                            return Err(Error("Missing output channel".into()));
                        }
                        m.outputs.push(Output {
                            channel: t[1..from].join(" "),
                            source: reference(&t[from + 1..])?,
                        });
                    } else {
                        return Err(Error("Expected row or output declaration".into()));
                    }
                }
                4 => {
                    if !row_open {
                        return Err(Error("Declaration outside row".into()));
                    }
                    let r = p
                        .materials
                        .last_mut()
                        .and_then(|m| m.rows.last_mut())
                        .ok_or_else(|| Error("Declaration outside row".into()))?;
                    if t[0] == "use" && (t.len() == 2 || (t.len() == 4 && t[2] == "id")) {
                        tile_open = true;
                        combine_seen = false;
                        r.tiles.push(Tile {
                            id: t.get(3).cloned().unwrap_or_else(new_tile_id),
                            input_disconnected: false,
                            kernel: t[1].clone(),
                            code: None,
                            image: None,
                            parameters: vec![],
                            input: None,
                            combine: Combine::Replace,
                        });
                    } else if t.len() > 2 && t[..2] == ["coordinates", "from"] {
                        if r.coordinates.is_some() {
                            return Err(Error("Duplicate coordinates binding".into()));
                        }
                        r.coordinates = Some(reference(&t[2..])?);
                        tile_open = false;
                    } else if t.len() >= 7 && t[..2] == ["row", "input"] && t[3] == "from" {
                        r.incoming.push(RowConnection {
                            id: t[2].clone(),
                            source: reference(&t[4..])?,
                        });
                        tile_open = false;
                    } else {
                        return Err(Error(
                            "Expected use \"Source Name\", coordinates from row or row input \"id\" from row".into(),
                        ));
                    }
                }
                6 => {
                    if !tile_open {
                        return Err(Error("Declaration outside tile".into()));
                    }
                    let tile = p
                        .materials
                        .last_mut()
                        .and_then(|m| m.rows.last_mut())
                        .and_then(|r| r.tiles.last_mut())
                        .ok_or_else(|| Error("Declaration outside tile".into()))?;
                    if t == ["input", "disconnected"] {
                        tile.input_disconnected = true;
                        tile.input = None;
                    } else if t.first().is_some_and(|s| s == "image") {
                        use crate::image_source::{ImagePlane, ImageSource};
                        let image = tile.image.get_or_insert_with(ImageSource::default);
                        match t.get(1).map(String::as_str) {
                            Some("size") if t.len() == 4 && image.width == 0 => {
                                image.width = t[2]
                                    .parse()
                                    .map_err(|_| Error("Invalid image width".into()))?;
                                image.height = t[3]
                                    .parse()
                                    .map_err(|_| Error("Invalid image height".into()))?;
                            }
                            Some("reference") if t.len() == 3 && image.reference.is_none() => {
                                image.reference = Some(t[2].clone())
                            }
                            Some("frame") if t.len() == 5 => {
                                if let Some(p) = image.planes.iter_mut().find(|p| p.channel == t[2])
                                {
                                    if p.space != t[3] {
                                        return Err(Error("Image plane mixes color spaces".into()));
                                    }
                                    p.frames.push(t[4].clone());
                                } else {
                                    image.planes.push(ImagePlane {
                                        channel: t[2].clone(),
                                        space: t[3].clone(),
                                        frames: vec![t[4].clone()],
                                    })
                                }
                            }
                            _ => {
                                return Err(Error(
                                    "Expected image size, image reference or image frame".into(),
                                ));
                            }
                        }
                    } else if t == ["source"] {
                        if tile.code.is_some() {
                            return Err(Error("Duplicate tile source".into()));
                        }
                        code_block = Some(String::new());
                    } else if t.len() >= 2 && t[0] == "combine" {
                        if combine_seen {
                            return Err(Error("Duplicate combine declaration".into()));
                        }
                        combine_seen = true;
                        tile.combine = Combine::parse(&t[1..].join(" "))?;
                    } else if t.len() >= 4 && t[0] == "set" {
                        let value = match t[2].as_str() {
                            "to" => Binding::Constant(
                                t[3..]
                                    .iter()
                                    .map(|s| match s.as_str() {
                                        "true" => Ok(1.),
                                        "false" => Ok(0.),
                                        _ => number(s),
                                    })
                                    .collect::<Result<_>>()?,
                            ),
                            "from" => Binding::From(reference(&t[3..])?),
                            _ => {
                                return Err(Error(
                                    "Expected set \"parameter\" to value / from row".into(),
                                ));
                            }
                        };
                        if tile.parameters.iter().any(|(id, _)| id == &t[1]) {
                            return Err(Error(format!("Duplicate parameter {:?}", t[1])));
                        }
                        tile.parameters.push((t[1].clone(), value));
                    } else if t.len() > 2 && t[..2] == ["input", "from"] {
                        if tile.input.is_some() {
                            return Err(Error("Duplicate input binding".into()));
                        }
                        tile.input = Some(reference(&t[2..])?);
                    } else {
                        return Err(Error("Expected set or input from row".into()));
                    }
                }
                _ => return Err(Error("Expected indentation of 0, 2, 4 or 6 spaces".into())),
            }
            Ok(())
        })();
        parsed.map_err(|e| Error(format!("line {line_number}: {e}")))?;
    }
    if code_block.is_some() {
        return Err(Error("Unclosed tile source block".into()));
    }
    if !version {
        return Err(Error("Missing project 1 declaration".into()));
    }
    let mut slots = std::collections::HashSet::new();
    for material in &p.materials {
        if let Some(slot) = material.stack_slot
            && (material.mixing.is_none() || !slots.insert(slot))
        {
            return Err(Error("Stack slots must be unique material mixers".into()));
        }
    }
    validate_tile_ids(&p)?;
    validate_row_connections(&p)?;
    Ok(p)
}

/// Validate authoring connections without making dormant links computation edges.
/// Actual bindings still participate in normal dependency and distance validation.
pub fn validate_row_connections(project: &Project) -> Result<()> {
    for material in &project.materials {
        let rows = material
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| (row.id.clone(), crate::ir::NodeId(index)))
            .collect();
        for row in &material.rows {
            let mut ids = std::collections::HashSet::new();
            for connection in &row.incoming {
                if connection.id.is_empty() || !ids.insert(&connection.id) {
                    return Err(Error(format!(
                        "Empty or duplicate row input ID {:?} in material {:?}, row {:?}",
                        connection.id, material.id, row.id
                    )));
                }
                let selection = connection
                    .source
                    .components
                    .rsplit('.')
                    .next()
                    .unwrap_or_default();
                let width = if selection == "distance" {
                    1
                } else {
                    selection.len()
                };
                if !(1..=4).contains(&width) {
                    return Err(Error(
                        "Row inputs need one to four components or distance".into(),
                    ));
                }
                crate::ir::resolve(&connection.source, &rows, width).map_err(|error| {
                    Error(format!(
                        "Row input {:?} in row {:?}: {error}",
                        connection.id, row.id
                    ))
                })?;
            }
        }
    }
    Ok(())
}

pub fn validate_tile_ids(project: &Project) -> Result<()> {
    let mut ids = std::collections::HashSet::new();
    for m in &project.materials {
        for r in &m.rows {
            for t in &r.tiles {
                if t.id.is_empty() || !ids.insert(&t.id) {
                    return Err(Error(format!(
                        "Empty or duplicate tile ID {:?} in material {:?}, row {:?}",
                        t.id, m.id, r.id
                    )));
                }
            }
        }
    }
    Ok(())
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
fn write_reference(r: &Reference) -> String {
    let mut s = format!("row {} {}", quote(&r.row), r.components);
    if let Some((lo, hi)) = r.mapping {
        write!(s, " mapped {lo} to {hi}").unwrap();
    }
    s
}
/// Canonical serialization; comments and original whitespace are not retained yet.
pub fn write(project: &Project) -> String {
    let mut s = String::from("project 1\n");
    if let Some(result) = &project.result {
        writeln!(s, "result from material {}", quote(result)).unwrap();
    }
    for m in &project.materials {
        writeln!(s, "\nmaterial {} called {}", quote(&m.id), quote(&m.label)).unwrap();
        writeln!(s, "  mode {}", m.mode).unwrap();
        if m.animation != Animation::default() {
            writeln!(
                s,
                "  animation {} {}",
                m.animation.frames, m.animation.duration
            )
            .unwrap();
        }
        writeln!(
            s,
            "  tiling {}\n  period {} {}\n  seam blend {}",
            m.tiling.mode.name(),
            m.tiling.period[0],
            m.tiling.period[1],
            m.tiling.blend
        )
        .unwrap();
        if let Some(mix) = &m.mixing {
            writeln!(s, "  mix {}", quote(&mix.kind)).unwrap();
            if let Some(id) = &mix.mask_material {
                writeln!(s, "  mixing mask-material {}", quote(id)).unwrap();
            }
            for (i, input) in mix.inputs.iter().enumerate() {
                if let Some(input) = input {
                    writeln!(s, "  mixing input {} from material {}", i + 1, quote(input)).unwrap();
                }
            }
            match &mix.mask {
                Binding::Constant(v) => {
                    writeln!(
                        s,
                        "  mixing mask to {}",
                        v.iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                    .unwrap();
                }
                Binding::From(r) => {
                    writeln!(s, "  mixing mask from {}", write_reference(r)).unwrap();
                }
            }
            writeln!(
                s,
                "  mixing blend {}\n  mixing direction {}\n  mixing levels {} {} {}",
                mix.blend,
                if mix.direction == 0 { "x" } else { "y" },
                mix.levels[0],
                mix.levels[1],
                mix.levels[2]
            )
            .unwrap();
        }
        if let Some(slot) = m.stack_slot {
            writeln!(s, "  stack slot {}", slot + 1).unwrap();
        }
        if let Some([x, y]) = m.position {
            writeln!(s, "  position {x} {y}").unwrap();
        }
        if m.locked {
            writeln!(s, "  locked true").unwrap();
        }
        for (name, values) in &m.defaults {
            write!(s, "  default {name} to").unwrap();
            for value in values {
                write!(s, " {value}").unwrap()
            }
            s.push('\n');
        }
        for r in &m.rows {
            if r.label.is_empty() {
                writeln!(s, "  row {}", quote(&r.id)).unwrap();
            } else {
                writeln!(s, "  row {} called {}", quote(&r.id), quote(&r.label)).unwrap();
            }
            if let Some(c) = &r.coordinates {
                writeln!(s, "    coordinates from {}", write_reference(c)).unwrap();
            }
            for connection in &r.incoming {
                writeln!(
                    s,
                    "    row input {} from {}",
                    quote(&connection.id),
                    write_reference(&connection.source)
                )
                .unwrap();
            }
            for tile in &r.tiles {
                writeln!(s, "    use {} id {}", quote(&tile.kernel), quote(&tile.id)).unwrap();
                if tile.combine != Combine::Replace {
                    writeln!(s, "      combine {}", tile.combine.name()).unwrap();
                }
                if let Some(input) = &tile.input {
                    writeln!(s, "      input from {}", write_reference(input)).unwrap();
                } else if tile.input_disconnected {
                    s.push_str("      input disconnected\n");
                }
                if let Some(image) = &tile.image {
                    writeln!(s, "      image size {} {}", image.width, image.height).unwrap();
                    if let Some(reference) = &image.reference {
                        writeln!(s, "      image reference {}", quote(reference)).unwrap();
                    }
                    for plane in &image.planes {
                        for frame in &plane.frames {
                            writeln!(
                                s,
                                "      image frame {} {} {}",
                                plane.channel, plane.space, frame
                            )
                            .unwrap();
                        }
                    }
                }
                if let Some(code) = &tile.code {
                    s.push_str("      source\n");
                    for line in code.lines() {
                        writeln!(s, "        {line}").unwrap()
                    }
                    s.push_str("      end source\n");
                }
                for (id, binding) in &tile.parameters {
                    write!(s, "      set {} ", quote(id)).unwrap();
                    match binding {
                        Binding::Constant(v) => {
                            s.push_str("to");
                            for x in v {
                                write!(s, " {x}").unwrap();
                            }
                            s.push('\n');
                        }
                        Binding::From(r) => {
                            writeln!(s, "from {}", write_reference(r)).unwrap();
                        }
                    }
                }
            }
        }
        for o in &m.outputs {
            writeln!(
                s,
                "  output {} from {}",
                o.channel,
                write_reference(&o.source)
            )
            .unwrap();
        }
    }
    s
}
