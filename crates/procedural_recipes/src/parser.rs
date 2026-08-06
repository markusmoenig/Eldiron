use crate::ast::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

/// Stable categories for machine-readable recipe diagnostics.
///
/// Callers should use this code instead of matching the human-readable
/// message, which may become more descriptive over time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorCode {
    Syntax,
    Document,
    MissingRequired,
    DuplicateDefinition,
    UnknownConstruct,
    InvalidValue,
    ConflictingFields,
    UnknownReference,
}

impl ParseErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Syntax => "PR0001",
            Self::Document => "PR0002",
            Self::MissingRequired => "PR0003",
            Self::DuplicateDefinition => "PR0004",
            Self::UnknownConstruct => "PR0005",
            Self::InvalidValue => "PR0006",
            Self::ConflictingFields => "PR0007",
            Self::UnknownReference => "PR0008",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub code: ParseErrorCode,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub source_line: Option<String>,
    pub source_name: Option<String>,
}

impl ParseError {
    fn new(line: usize, message: impl Into<String>) -> Self {
        Self::with_code(ParseErrorCode::Syntax, line, message)
    }

    fn with_code(code: ParseErrorCode, line: usize, message: impl Into<String>) -> Self {
        Self {
            code,
            line: line.max(1),
            column: 1,
            message: message.into(),
            source_line: None,
            source_name: None,
        }
    }

    fn document(line: usize, message: impl Into<String>) -> Self {
        Self::with_code(ParseErrorCode::Document, line, message)
    }

    fn missing(line: usize, message: impl Into<String>) -> Self {
        Self::with_code(ParseErrorCode::MissingRequired, line, message)
    }

    fn duplicate(line: usize, message: impl Into<String>) -> Self {
        Self::with_code(ParseErrorCode::DuplicateDefinition, line, message)
    }

    fn unknown(line: usize, message: impl Into<String>) -> Self {
        Self::with_code(ParseErrorCode::UnknownConstruct, line, message)
    }

    fn invalid(line: usize, message: impl Into<String>) -> Self {
        Self::with_code(ParseErrorCode::InvalidValue, line, message)
    }

    fn conflict(line: usize, message: impl Into<String>) -> Self {
        Self::with_code(ParseErrorCode::ConflictingFields, line, message)
    }

    fn reference(line: usize, message: impl Into<String>) -> Self {
        Self::with_code(ParseErrorCode::UnknownReference, line, message)
    }

    fn attach_source(mut self, source: &str) -> Self {
        if let Some(source_line) = source.lines().nth(self.line.saturating_sub(1)) {
            self.column = source_line
                .chars()
                .take_while(|character| character.is_whitespace())
                .count()
                + 1;
            self.source_line = Some(source_line.to_string());
        }
        self
    }

    /// Adds a filename or other source label to the rendered diagnostic.
    pub fn with_source_name(mut self, source_name: impl Into<String>) -> Self {
        self.source_name = Some(source_name.into());
        self
    }

    pub const fn stable_code(&self) -> &'static str {
        self.code.as_str()
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "error[{}]: {}", self.stable_code(), self.message)?;
        if let Some(source_name) = &self.source_name {
            write!(f, " --> {source_name}:{}:{}", self.line, self.column)?;
        } else {
            write!(f, " --> line {}:{}", self.line, self.column)?;
        }
        if let Some(source_line) = &self.source_line {
            let gutter_width = self.line.to_string().len();
            write!(
                f,
                "\n {:gutter_width$} |\n {} | {}\n {:gutter_width$} | {}^",
                "",
                self.line,
                source_line,
                "",
                " ".repeat(self.column.saturating_sub(1)),
            )?;
        }
        Ok(())
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
        RecipeDocument::Materials(_) => Err(ParseError::document(
            1,
            "expected a Tile recipe, found Material declarations",
        )
        .attach_source(source)),
        RecipeDocument::Sdfs(_) => Err(ParseError::document(
            1,
            "expected a Tile recipe, found Sdf declarations",
        )
        .attach_source(source)),
    }
}

pub fn parse_material_document(source: &str) -> Result<MaterialDocument, ParseError> {
    match parse_document(source)? {
        RecipeDocument::Materials(materials) => Ok(materials),
        RecipeDocument::Tile(_) => Err(ParseError::document(
            1,
            "expected Material declarations, found a Tile recipe",
        )
        .attach_source(source)),
        RecipeDocument::Sdfs(_) => Err(ParseError::document(
            1,
            "expected Material declarations, found Sdf declarations",
        )
        .attach_source(source)),
    }
}

pub fn parse_sdf_document(source: &str) -> Result<SdfDocument, ParseError> {
    match parse_document(source)? {
        RecipeDocument::Sdfs(document) => Ok(document),
        RecipeDocument::Tile(_) => Err(ParseError::document(
            1,
            "expected Sdf declarations, found a Tile recipe",
        )
        .attach_source(source)),
        RecipeDocument::Materials(_) => Err(ParseError::document(
            1,
            "expected Sdf declarations, found Material declarations",
        )
        .attach_source(source)),
    }
}

pub fn parse_document(source: &str) -> Result<RecipeDocument, ParseError> {
    let mut document = parse_document_inner(source).map_err(|error| error.attach_source(source))?;
    attach_warning_sources(&mut document, source);
    Ok(document)
}

fn attach_warning_sources(document: &mut RecipeDocument, source: &str) {
    let attach = |patterns: &mut [PatternDefinition]| {
        for pattern in patterns {
            for warning in &mut pattern.warnings {
                if let Some(source_line) = source.lines().nth(warning.line.saturating_sub(1)) {
                    warning.column = source_line
                        .chars()
                        .take_while(|character| character.is_whitespace())
                        .count()
                        + 1;
                    warning.source_line = Some(source_line.to_string());
                }
            }
        }
    };
    match document {
        RecipeDocument::Tile(recipe) => attach(&mut recipe.patterns),
        RecipeDocument::Materials(document) => {
            for material in &mut document.materials {
                attach(&mut material.patterns);
            }
        }
        RecipeDocument::Sdfs(_) => {}
    }
}

fn parse_document_inner(source: &str) -> Result<RecipeDocument, ParseError> {
    let lines = tokenize(source)?;
    if lines.is_empty() {
        return Err(ParseError::document(1, "recipe is empty"));
    }
    let mut cursor = 0;
    let roots = parse_nodes(&lines, &mut cursor, 0)?;
    if roots.len() == 1 && roots[0].name.eq_ignore_ascii_case("Tile") {
        return recipe_from_node(&roots[0]).map(RecipeDocument::Tile);
    }

    let root_kind = declaration(&roots[0].name).0;
    if root_kind == "sdf" {
        let mut ids = BTreeSet::new();
        let mut recipes = Vec::with_capacity(roots.len());
        for root in &roots {
            let (kind, declared_name) = declaration(&root.name);
            if kind != "sdf" {
                return Err(ParseError::document(
                    root.line,
                    "Sdf and Material declarations cannot be mixed in one .recipe file",
                ));
            }
            let id = required_declaration_name(root, declared_name)?;
            if !ids.insert(id.to_ascii_lowercase()) {
                return Err(ParseError::duplicate(
                    root.line,
                    format!("duplicate SDF id '{id}'"),
                ));
            }
            recipes.push(sdf_from_node(root, id)?);
        }
        return Ok(RecipeDocument::Sdfs(SdfDocument { recipes }));
    }

    let mut ids = BTreeSet::new();
    let mut materials = Vec::with_capacity(roots.len());
    for root in &roots {
        let (kind, declared_name) = declaration(&root.name);
        if kind != "material" {
            return Err(ParseError::document(
                root.line,
                "a .recipe must contain one Tile block, only Material <id> blocks, or only Sdf <id> blocks",
            ));
        }
        let id = required_declaration_name(root, declared_name)?;
        let normalized = id.to_ascii_lowercase();
        if !ids.insert(normalized) {
            return Err(ParseError::duplicate(
                root.line,
                format!("duplicate material id '{id}'"),
            ));
        }
        materials.push(material_from_node(root, id)?);
    }
    Ok(RecipeDocument::Materials(MaterialDocument { materials }))
}

fn sdf_from_node(node: &Node, id: &str) -> Result<SdfRecipe, ParseError> {
    reject_unknown_fields(node, &["name"])?;
    let mut shapes = Vec::new();
    let mut names = BTreeSet::new();
    let mut output = None;
    for child in &node.children {
        let (kind, declared_name) = declaration(&child.name);
        match kind.as_str() {
            "shape" => {
                let name = required_declaration_name(child, declared_name)?.to_string();
                if !names.insert(name.to_ascii_lowercase()) {
                    return Err(ParseError::duplicate(
                        child.line,
                        format!("duplicate SDF shape '{name}'"),
                    ));
                }
                shapes.push(SdfShape {
                    name,
                    kind: parse_sdf_shape(child)?,
                });
            }
            "output" => {
                ensure_unnamed(child, declared_name)?;
                reject_unknown_fields(child, &["coverage"])?;
                reject_children(child)?;
                let (value, line) = field(child, "coverage").ok_or_else(|| {
                    ParseError::missing(child.line, "Sdf Output requires coverage")
                })?;
                let value = parse_sdf_reference(value, line)?;
                if output.replace(value).is_some() {
                    return Err(ParseError::duplicate(child.line, "duplicate Output block"));
                }
            }
            _ => {
                return Err(ParseError::unknown(
                    child.line,
                    format!(
                        "unknown SDF block '{}'; expected Shape <name> or Output",
                        child.name
                    ),
                ));
            }
        }
    }
    let output = output.ok_or_else(|| ParseError::missing(node.line, "Sdf requires Output"))?;
    if !names.contains(&output.to_ascii_lowercase()) {
        return Err(ParseError::reference(
            node.line,
            format!("unknown SDF output shape '{output}'"),
        ));
    }
    for shape in &shapes {
        let references: &[&str] = match &shape.kind {
            SdfShapeKind::Union { a, b }
            | SdfShapeKind::Subtract { a, b }
            | SdfShapeKind::Intersect { a, b } => &[a, b],
            SdfShapeKind::Expand { source, .. } | SdfShapeKind::Contract { source, .. } => {
                &[source]
            }
            _ => &[],
        };
        for reference in references {
            if !names.contains(&reference.to_ascii_lowercase()) {
                return Err(ParseError::reference(
                    node.line,
                    format!(
                        "SDF shape '{}' references unknown shape '{reference}'",
                        shape.name
                    ),
                ));
            }
        }
    }
    Ok(SdfRecipe {
        id: id.to_string(),
        name: field(node, "name")
            .map(|(value, line)| parse_string(value, line))
            .transpose()?
            .unwrap_or_else(|| id.replace('_', " ")),
        shapes,
        output,
    })
}

fn parse_sdf_shape(node: &Node) -> Result<SdfShapeKind, ParseError> {
    reject_unknown_fields(node, &[])?;
    if node.children.len() != 1 {
        return Err(ParseError::invalid(
            node.line,
            "Shape requires exactly one primitive or operation block",
        ));
    }
    let operation = &node.children[0];
    let (kind, declared_name) = declaration(&operation.name);
    ensure_unnamed(operation, declared_name)?;
    reject_children(operation)?;
    let finite_positive_size =
        |size: [f32; 2]| size.iter().all(|value| value.is_finite() && *value > 0.0);
    match kind.as_str() {
        "ellipse" => {
            reject_unknown_fields(operation, &["position", "size", "rotation"])?;
            let size = optional_f2(operation, "size", [0.5, 0.5])?;
            if !finite_positive_size(size) {
                return Err(ParseError::invalid(
                    operation.line,
                    "Ellipse.size must be finite and greater than zero",
                ));
            }
            Ok(SdfShapeKind::Ellipse {
                position: optional_f2(operation, "position", [0.5, 0.5])?,
                size,
                rotation: optional_f32(operation, "rotation", 0.0)?,
            })
        }
        "roundedrectangle" | "rounded_rectangle" => {
            reject_unknown_fields(operation, &["position", "size", "radius", "rotation"])?;
            let size = optional_f2(operation, "size", [0.5, 0.5])?;
            let radius = optional_f32(operation, "radius", 0.05)?;
            if !finite_positive_size(size) || !radius.is_finite() || radius < 0.0 {
                return Err(ParseError::invalid(
                    operation.line,
                    "RoundedRectangle requires a positive finite size and non-negative radius",
                ));
            }
            Ok(SdfShapeKind::RoundedRectangle {
                position: optional_f2(operation, "position", [0.5, 0.5])?,
                size,
                radius,
                rotation: optional_f32(operation, "rotation", 0.0)?,
            })
        }
        "capsule" => {
            reject_unknown_fields(operation, &["from", "to", "radius"])?;
            let radius = optional_f32(operation, "radius", 0.05)?;
            if !radius.is_finite() || radius <= 0.0 {
                return Err(ParseError::invalid(
                    operation.line,
                    "Capsule.radius must be finite and greater than zero",
                ));
            }
            Ok(SdfShapeKind::Capsule {
                from: optional_f2(operation, "from", [0.25, 0.5])?,
                to: optional_f2(operation, "to", [0.75, 0.5])?,
                radius,
            })
        }
        "union" | "subtract" | "intersect" => {
            reject_unknown_fields(operation, &["a", "b"])?;
            let reference = |field_name| {
                let (value, line) = field(operation, field_name).ok_or_else(|| {
                    ParseError::missing(
                        operation.line,
                        format!("{} requires {field_name}", operation.name),
                    )
                })?;
                parse_sdf_reference(value, line)
            };
            let a = reference("a")?;
            let b = reference("b")?;
            Ok(match kind.as_str() {
                "union" => SdfShapeKind::Union { a, b },
                "subtract" => SdfShapeKind::Subtract { a, b },
                _ => SdfShapeKind::Intersect { a, b },
            })
        }
        "expand" | "contract" => {
            reject_unknown_fields(operation, &["source", "amount"])?;
            let (source, line) = field(operation, "source").ok_or_else(|| {
                ParseError::missing(
                    operation.line,
                    format!("{} requires source", operation.name),
                )
            })?;
            let amount = optional_f32(operation, "amount", 0.0)?;
            if !amount.is_finite() || amount < 0.0 {
                return Err(ParseError::invalid(
                    operation.line,
                    "SDF expand/contract amount must be finite and non-negative",
                ));
            }
            let source = parse_sdf_reference(source, line)?;
            Ok(if kind == "expand" {
                SdfShapeKind::Expand { source, amount }
            } else {
                SdfShapeKind::Contract { source, amount }
            })
        }
        _ => Err(ParseError::unknown(
            operation.line,
            format!("unknown SDF primitive or operation '{}'", operation.name),
        )),
    }
}

fn parse_sdf_reference(value: &str, line: usize) -> Result<String, ParseError> {
    let value = value.trim();
    validate_identifier(value, line)?;
    Ok(value.to_string())
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
                    return Err(ParseError::duplicate(
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
            "name",
            "placement",
            "blocking",
            "material",
            "size",
            "coverage",
            "wrap",
            "seed",
            "pixelate",
        ],
    )?;
    let mut recipe = Recipe {
        name: field(node, "name")
            .map(|(value, line)| parse_string(value, line))
            .transpose()?
            .unwrap_or_else(|| "Untitled Tile".to_string()),
        placement: parse_recipe_placement(node)?,
        blocking: optional_bool(node, "blocking", false)?,
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
    let mut material_map = None;
    let mut implicit_output = None;
    let mut geometry_seen = false;
    let mut effect_names = BTreeSet::new();
    for child in &node.children {
        let (kind, declared_name) = declaration(&child.name);
        match kind.as_str() {
            "animation" => {
                ensure_unnamed(child, declared_name)?;
                if animation.replace(parse_animation(child)?).is_some() {
                    return Err(ParseError::duplicate(
                        child.line,
                        "duplicate Animation block",
                    ));
                }
            }
            "noise" => {
                let noise = parse_noise(child, declared_name)?;
                implicit_output = Some(ScalarSource::Field(noise.name.clone()));
                recipe.fields.push(FieldDefinition::Noise(noise));
            }
            "height" => {
                let height = parse_height(child, declared_name)?;
                implicit_output = Some(ScalarSource::Field(height.name.clone()));
                recipe.fields.push(FieldDefinition::Height(height));
            }
            "pattern" => {
                let pattern = parse_pattern(child, declared_name)?;
                implicit_output = Some(ScalarSource::Pattern {
                    name: pattern.name.clone(),
                    channel: PatternChannel::Height,
                });
                recipe.patterns.push(pattern);
            }
            "colorize" => {
                ensure_unnamed(child, declared_name)?;
                let parsed = parse_colorize(child)?;
                implicit_output = Some(parsed.source.clone());
                if colorize.replace(parsed).is_some() {
                    return Err(ParseError::duplicate(
                        child.line,
                        "duplicate Colorize block",
                    ));
                }
            }
            "output" => {
                ensure_unnamed(child, declared_name)?;
                if output.replace(parse_output(child)?).is_some() {
                    return Err(ParseError::duplicate(child.line, "duplicate Output block"));
                }
            }
            "materialmap" => {
                ensure_unnamed(child, declared_name)?;
                if material_map.replace(parse_material_map(child)?).is_some() {
                    return Err(ParseError::duplicate(
                        child.line,
                        "duplicate MaterialMap block",
                    ));
                }
            }
            "geometry" => {
                ensure_unnamed(child, declared_name)?;
                if geometry_seen {
                    return Err(ParseError::duplicate(
                        child.line,
                        "duplicate Geometry block",
                    ));
                }
                recipe.geometry = parse_geometry(child)?;
                geometry_seen = true;
            }
            "attachment" => {
                let attachment = parse_attachment(child, declared_name)?;
                if !effect_names.insert(format!(
                    "attachment:{}",
                    attachment.name.to_ascii_lowercase()
                )) {
                    return Err(ParseError::duplicate(
                        child.line,
                        format!("duplicate Attachment '{}'", attachment.name),
                    ));
                }
                recipe.attachments.push(attachment);
            }
            "light" => {
                let light = parse_light_effect(child, declared_name)?;
                if !effect_names.insert(format!("light:{}", light.name.to_ascii_lowercase())) {
                    return Err(ParseError::duplicate(
                        child.line,
                        format!("duplicate Light '{}'", light.name),
                    ));
                }
                recipe.lights.push(light);
            }
            "particles" => {
                let particles = parse_particle_effect(child, declared_name)?;
                if !effect_names
                    .insert(format!("particles:{}", particles.name.to_ascii_lowercase()))
                {
                    return Err(ParseError::duplicate(
                        child.line,
                        format!("duplicate Particles '{}'", particles.name),
                    ));
                }
                recipe.particles.push(particles);
            }
            _ => {
                return Err(ParseError::unknown(
                    child.line,
                    format!(
                        "unknown top-level block '{}'; expected Noise <name>, Pattern <name>, Height <name>, Geometry, Attachment <name>, Light <name>, Particles <name>, Colorize, MaterialMap, or Output",
                        child.name
                    ),
                ));
            }
        }
    }
    recipe.animation = animation.unwrap_or_default();
    if recipe.material.is_some() && material_map.is_some() {
        return Err(ParseError::conflict(
            node.line,
            "Tile.material and MaterialMap are alternatives; use only one",
        ));
    }
    recipe.material_map = material_map;
    recipe.output = output
        .or_else(|| {
            implicit_output.map(|height| Output {
                height,
                ..Output::default()
            })
        })
        .ok_or_else(|| {
            ParseError::missing(
                node.line,
                "Tile needs Output or at least one scalar-producing Noise, Pattern, Height, or Colorize block",
            )
        })?;
    recipe.colorize = colorize;
    validate_names_and_references(&recipe, node, true)?;
    Ok(recipe)
}

fn parse_geometry(node: &Node) -> Result<Vec<GeometryFeature>, ParseError> {
    reject_unknown_fields(node, &[])?;
    let mut features = Vec::new();
    let mut names = BTreeSet::new();
    for child in &node.children {
        let (kind, declared_name) = declaration(&child.name);
        match kind.as_str() {
            "box" => {
                let geometry_box = parse_box_geometry(child, declared_name)?;
                if !names.insert(geometry_box.name.to_ascii_lowercase()) {
                    return Err(ParseError::duplicate(
                        child.line,
                        format!("duplicate Geometry feature '{}'", geometry_box.name),
                    ));
                }
                features.push(GeometryFeature::Box(geometry_box));
            }
            _ => {
                return Err(ParseError::unknown(
                    child.line,
                    format!(
                        "unknown Geometry primitive '{}'; expected Box <name>",
                        child.name
                    ),
                ));
            }
        }
    }
    if features.is_empty() {
        return Err(ParseError::missing(
            node.line,
            "Geometry requires at least one feature",
        ));
    }
    Ok(features)
}

fn parse_attachment(node: &Node, name: Option<&str>) -> Result<Attachment, ParseError> {
    reject_unknown_fields(node, &["position", "direction"])?;
    reject_children(node)?;
    let name = required_declaration_name(node, name)?.to_string();
    let position = optional_f3(node, "position", [0.0, 0.0, 0.0])?;
    let direction = optional_f3(node, "direction", [0.0, 1.0, 0.0])?;
    validate_finite_f3(node, "position", position)?;
    validate_direction(node, direction)?;
    Ok(Attachment {
        name,
        position,
        direction,
    })
}

fn parse_light_effect(node: &Node, name: Option<&str>) -> Result<LightEffect, ParseError> {
    reject_unknown_fields(
        node,
        &["attach", "color", "intensity", "range", "flicker", "lift"],
    )?;
    reject_children(node)?;
    let name = required_declaration_name(node, name)?.to_string();
    let attachment = required_identifier_field(node, "attach")?;
    let color = field(node, "color")
        .map(|(value, line)| parse_authored_color(value, line))
        .transpose()?
        .unwrap_or([255, 196, 96, 255]);
    let intensity = optional_f32(node, "intensity", 1.0)?;
    let range = optional_f32(node, "range", 4.0)?;
    let flicker = optional_f32(node, "flicker", 0.0)?;
    let lift = optional_f32(node, "lift", 0.0)?;
    for (field_name, value) in [
        ("intensity", intensity),
        ("range", range),
        ("flicker", flicker),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(ParseError::invalid(
                field(node, field_name).map_or(node.line, |(_, line)| line),
                format!("Light.{field_name} must be a finite non-negative number"),
            ));
        }
    }
    if !lift.is_finite() {
        return Err(ParseError::invalid(
            field(node, "lift").map_or(node.line, |(_, line)| line),
            "Light.lift must be finite",
        ));
    }
    Ok(LightEffect {
        name,
        attachment,
        color,
        intensity,
        range,
        flicker,
        lift,
    })
}

fn parse_particle_effect(node: &Node, name: Option<&str>) -> Result<ParticleEffect, ParseError> {
    reject_unknown_fields(
        node,
        &[
            "attach",
            "direction",
            "spread",
            "rate",
            "color",
            "color_ramp",
            "color_variation",
            "lifetime",
            "radius",
            "speed",
            "spawn_area",
            "flame_base",
        ],
    )?;
    reject_children(node)?;
    let name = required_declaration_name(node, name)?.to_string();
    let attachment = required_identifier_field(node, "attach")?;
    let direction = optional_f3(node, "direction", [0.0, 1.0, 0.0])?;
    validate_direction(node, direction)?;
    let spread = optional_f32(node, "spread", std::f32::consts::FRAC_PI_4)?;
    let rate = optional_f32(node, "rate", 30.0)?;
    let color = field(node, "color")
        .map(|(value, line)| parse_authored_color(value, line))
        .transpose()?
        .unwrap_or([255, 160, 0, 255]);
    let color_ramp = field(node, "color_ramp")
        .map(|(value, line)| parse_color_ramp4(value, line))
        .transpose()?;
    let variation = optional_u32(node, "color_variation", 30)?;
    if variation > u8::MAX as u32 {
        return Err(ParseError::invalid(
            field(node, "color_variation").unwrap().1,
            "Particles.color_variation must be within 0..255",
        ));
    }
    let lifetime = optional_f2(node, "lifetime", [0.5, 1.5])?;
    let radius = optional_f2(node, "radius", [0.05, 0.15])?;
    let speed = optional_f2(node, "speed", [0.5, 1.5])?;
    let spawn_area = optional_f3(node, "spawn_area", [0.0, 0.0, 0.0])?;
    for (field_name, range) in [("lifetime", lifetime), ("radius", radius), ("speed", speed)] {
        validate_non_negative_range(node, field_name, range)?;
    }
    validate_finite_f3(node, "spawn_area", spawn_area)?;
    if spawn_area.iter().any(|value| *value < 0.0) {
        return Err(ParseError::invalid(
            field(node, "spawn_area").map_or(node.line, |(_, line)| line),
            "Particles.spawn_area components must be non-negative",
        ));
    }
    for (field_name, value) in [("spread", spread), ("rate", rate)] {
        if !value.is_finite() || value < 0.0 {
            return Err(ParseError::invalid(
                field(node, field_name).map_or(node.line, |(_, line)| line),
                format!("Particles.{field_name} must be a finite non-negative number"),
            ));
        }
    }
    Ok(ParticleEffect {
        name,
        attachment,
        direction,
        spread,
        rate,
        color,
        color_ramp,
        color_variation: variation as u8,
        lifetime,
        radius,
        speed,
        spawn_area,
        flame_base: optional_bool(node, "flame_base", false)?,
    })
}

fn required_identifier_field(node: &Node, name: &str) -> Result<String, ParseError> {
    let (value, line) = field(node, name)
        .ok_or_else(|| ParseError::missing(node.line, format!("{} requires {name}", node.name)))?;
    let value = value.trim().trim_matches('"');
    validate_identifier(value, line)?;
    Ok(value.to_string())
}

fn parse_color_ramp4(value: &str, line: usize) -> Result<[[u8; 4]; 4], ParseError> {
    let values = split_arguments(value);
    if values.len() != 4 {
        return Err(ParseError::invalid(
            line,
            "Particles.color_ramp requires four comma-separated colors",
        ));
    }
    Ok([
        parse_authored_color(values[0], line)?,
        parse_authored_color(values[1], line)?,
        parse_authored_color(values[2], line)?,
        parse_authored_color(values[3], line)?,
    ])
}

fn validate_direction(node: &Node, direction: [f32; 3]) -> Result<(), ParseError> {
    validate_finite_f3(node, "direction", direction)?;
    if direction.iter().map(|value| value * value).sum::<f32>() <= f32::EPSILON {
        return Err(ParseError::invalid(
            field(node, "direction").map_or(node.line, |(_, line)| line),
            "direction must not be zero",
        ));
    }
    Ok(())
}

fn validate_finite_f3(node: &Node, name: &str, value: [f32; 3]) -> Result<(), ParseError> {
    if value.iter().any(|value| !value.is_finite()) {
        Err(ParseError::invalid(
            field(node, name).map_or(node.line, |(_, line)| line),
            format!("{name} components must be finite"),
        ))
    } else {
        Ok(())
    }
}

fn validate_non_negative_range(node: &Node, name: &str, range: [f32; 2]) -> Result<(), ParseError> {
    if range.iter().any(|value| !value.is_finite() || *value < 0.0) || range[0] > range[1] {
        Err(ParseError::invalid(
            field(node, name).map_or(node.line, |(_, line)| line),
            format!("Particles.{name} must be an ordered non-negative F2 range"),
        ))
    } else {
        Ok(())
    }
}

fn parse_box_geometry(node: &Node, name: Option<&str>) -> Result<BoxGeometry, ParseError> {
    reject_unknown_fields(
        node,
        &[
            "operation",
            "surface",
            "position",
            "size",
            "repeat",
            "spacing",
        ],
    )?;
    reject_children(node)?;

    let name = required_declaration_name(node, name)?.to_string();
    let (surface, surface_line) = field(node, "surface")
        .ok_or_else(|| ParseError::missing(node.line, "Box requires surface"))?;
    let operation = match field(node, "operation") {
        Some((value, _)) if value.eq_ignore_ascii_case("add") => GeometryOperation::Add,
        Some((value, _)) if value.eq_ignore_ascii_case("subtract") => GeometryOperation::Subtract,
        Some((_value, line)) => {
            return Err(ParseError::invalid(
                line,
                "Box.operation must be Add or Subtract",
            ));
        }
        None => GeometryOperation::Add,
    };
    let position = optional_f3(node, "position", [0.0, 0.0, 0.0])?;
    let size = optional_f3(node, "size", [1.0, 1.0, 1.0])?;
    let repeat = optional_i3(node, "repeat", [1, 1, 1])?;
    let spacing = optional_f3(node, "spacing", [0.0, 0.0, 0.0])?;

    if position.iter().any(|value| !value.is_finite()) {
        return Err(ParseError::invalid(
            node.line,
            "Box.position must use finite placement-local coordinates",
        ));
    }
    if size.iter().any(|value| !value.is_finite() || *value <= 0.0) {
        return Err(ParseError::invalid(
            node.line,
            "Box.size components must be finite and greater than zero",
        ));
    }
    if repeat.iter().any(|value| *value == 0 || *value > 64) {
        return Err(ParseError::invalid(
            node.line,
            "Box.repeat components must be between 1 and 64",
        ));
    }
    if spacing.iter().any(|value| !value.is_finite()) {
        return Err(ParseError::invalid(
            node.line,
            "Box.spacing must use finite placement-local distances",
        ));
    }

    Ok(BoxGeometry {
        name,
        operation,
        surface: parse_alias(surface, surface_line)?,
        position,
        size,
        repeat,
        spacing,
    })
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
        colors: Vec::new(),
        surface: MaterialSurface::default(),
        output: None,
    };

    let mut surface = None;
    let mut output = None;
    for child in &node.children {
        let (kind, declared_name) = declaration(&child.name);
        match kind.as_str() {
            "noise" => material
                .fields
                .push(FieldDefinition::Noise(parse_noise(child, declared_name)?)),
            "value" => material
                .fields
                .push(FieldDefinition::Value(parse_value(child, declared_name)?)),
            "height" => {
                return Err(ParseError::invalid(
                    child.line,
                    "Height is tile geometry and is not available in materials; use Value for an intermediate scalar",
                ));
            }
            "pattern" => material.patterns.push(parse_pattern(child, declared_name)?),
            "color" => material
                .colors
                .push(parse_color_definition(child, declared_name)?),
            "surface" => {
                ensure_unnamed(child, declared_name)?;
                if surface.replace(parse_material_surface(child)?).is_some() {
                    return Err(ParseError::duplicate(child.line, "duplicate Surface block"));
                }
            }
            "output" => {
                ensure_unnamed(child, declared_name)?;
                if output.replace(parse_material_output(child)?).is_some() {
                    return Err(ParseError::duplicate(child.line, "duplicate Output block"));
                }
            }
            _ => {
                return Err(ParseError::unknown(
                    child.line,
                    format!(
                        "unknown material block '{}'; expected Noise <name>, Pattern <name>, Value <name>, Color <name>, Surface, or Output",
                        child.name
                    ),
                ));
            }
        }
    }
    material.surface =
        surface.ok_or_else(|| ParseError::missing(node.line, "Material requires Surface"))?;
    material.output = output;
    validate_material_names_and_references(&material, node)?;
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
        Err(ParseError::conflict(
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
        ParseError::missing(
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
        return Err(ParseError::conflict(
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

fn parse_value(node: &Node, name: Option<&str>) -> Result<ValueField, ParseError> {
    reject_unknown_fields(node, &["source"])?;
    reject_children(node)?;
    Ok(ValueField {
        name: required_declaration_name(node, name)?.to_string(),
        source: required_scalar(node, "source")?,
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
        _ => Err(ParseError::unknown(
            node.line,
            format!("unknown height operation '{}'", node.name),
        )),
    }
}

fn parse_pattern(node: &Node, name: Option<&str>) -> Result<PatternDefinition, ParseError> {
    reject_unknown_fields(node, &[])?;
    if node.children.len() != 1 {
        return Err(ParseError::missing(
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
                    falloff: optional_scalar(generator, "falloff", 1.0)?,
                    seed: optional_u64(generator, "seed", 0)?,
                },
                vec![
                    "columns",
                    "rows",
                    "stagger",
                    "gap",
                    "bevel",
                    "rounding",
                    "rotation",
                    "size_variation",
                    "warp",
                    "warp_amount",
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
                    "bevel",
                    "warp",
                    "warp_amount",
                    "perturb",
                    "perturb_amount",
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
                    "bevel",
                    "warp",
                    "warp_amount",
                    "perturb",
                    "perturb_amount",
                    "seed",
                    "falloff",
                ],
            ),
            _ => {
                return Err(ParseError::unknown(
                    generator.line,
                    "Pattern generator must be Bricks, Voronoi, or Discs",
                ));
            }
        };
    let warnings = unsupported_field_warnings(generator, &allowed);
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
        return Err(ParseError::missing(
            field(generator, "warp_amount").unwrap().1,
            "warp_amount requires warp",
        ));
    }
    let perturb = field(generator, "perturb")
        .map(|(value, line)| {
            Ok::<_, ParseError>(Perturb {
                source: parse_scalar_source(value, line)?,
                amount: optional_scalar(generator, "perturb_amount", 0.05)?,
            })
        })
        .transpose()?;
    if perturb.is_none() && field(generator, "perturb_amount").is_some() {
        return Err(ParseError::missing(
            field(generator, "perturb_amount").unwrap().1,
            "perturb_amount requires perturb",
        ));
    }
    Ok(PatternDefinition {
        name: required_declaration_name(node, name)?.to_string(),
        domain,
        key,
        warp,
        bevel: optional_scalar(generator, "bevel", 0.08)?,
        perturb,
        kind,
        warnings,
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
        .map(|(value, line)| parse_authored_color(value, line))
        .transpose()?;
    let palette = field(node, "palette")
        .map(|(value, line)| parse_palette_mode(value, line))
        .transpose()?
        .unwrap_or_default();
    if palette == PaletteMode::BaseOnly && base.is_none() {
        return Err(ParseError::missing(
            field(node, "palette").unwrap().1,
            "Colorize.palette = BaseOnly requires Colorize.base",
        ));
    }
    if base.is_some() {
        for legacy in ["ramp", "ramp_range", "saturation_range"] {
            if let Some((_, line)) = field(node, legacy) {
                return Err(ParseError::conflict(
                    line,
                    format!("Colorize.{legacy} cannot be combined with Colorize.base"),
                ));
            }
        }
    } else {
        for anchored in ["brightness", "saturation", "hue"] {
            if let Some((_, line)) = field(node, anchored) {
                return Err(ParseError::missing(
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
        _ => Err(ParseError::invalid(
            line,
            "palette must be Strict or BaseOnly",
        )),
    }
}

fn parse_output(node: &Node) -> Result<Output, ParseError> {
    reject_unknown_fields(node, &["height", "space"])?;
    reject_children(node)?;
    Ok(Output {
        height: required_scalar(node, "height")?,
        space: field(node, "space")
            .map(|(value, line)| parse_domain(value, line))
            .transpose()?
            .unwrap_or(Domain::Global),
    })
}

fn parse_material_map(node: &Node) -> Result<MaterialMap, ParseError> {
    reject_unknown_fields(node, &["base", "space", "tiling"])?;
    let (base, line) = field(node, "base")
        .ok_or_else(|| ParseError::missing(node.line, "MaterialMap requires base"))?;
    let mut layers = Vec::new();
    for child in &node.children {
        let (kind, declared_name) = declaration(&child.name);
        if kind != "layer" {
            return Err(ParseError::unknown(
                child.line,
                format!("unknown MaterialMap block '{}'; expected Layer", child.name),
            ));
        }
        ensure_unnamed(child, declared_name)?;
        reject_unknown_fields(child, &["material", "mask", "space", "tiling"])?;
        reject_children(child)?;
        let (material, material_line) = field(child, "material")
            .ok_or_else(|| ParseError::missing(child.line, "Layer requires material"))?;
        layers.push(MaterialLayer {
            material: parse_alias(material, material_line)?,
            mask: required_scalar(child, "mask")?,
            space: field(child, "space")
                .map(|(value, line)| parse_domain(value, line))
                .transpose()?
                .unwrap_or(Domain::Global),
            tiling: optional_positive_f2(child, "tiling", [1.0, 1.0])?,
        });
    }
    Ok(MaterialMap {
        base: parse_alias(base, line)?,
        space: field(node, "space")
            .map(|(value, line)| parse_domain(value, line))
            .transpose()?
            .unwrap_or(Domain::Global),
        tiling: optional_positive_f2(node, "tiling", [1.0, 1.0])?,
        layers,
    })
}

fn parse_color_definition(node: &Node, name: Option<&str>) -> Result<ColorDefinition, ParseError> {
    reject_unknown_fields(node, &["base", "exact", "nearest", "source"])?;
    reject_children(node)?;
    reject_alias_pair(node, "base", "exact")?;
    let mut sources = Vec::new();
    if let Some((value, line)) = field(node, "base").or_else(|| field(node, "exact")) {
        sources.push(ColorSource::Exact(parse_authored_color(value, line)?));
    }
    if let Some((value, line)) = field(node, "nearest") {
        sources.push(ColorSource::Nearest(parse_authored_color(value, line)?));
    }
    if let Some((value, line)) = field(node, "source") {
        sources.push(parse_color_source(value, line)?);
    }
    if sources.len() != 1 {
        return Err(ParseError::conflict(
            node.line,
            "Color requires exactly one of base, exact, nearest, or source",
        ));
    }
    Ok(ColorDefinition {
        name: required_declaration_name(node, name)?.to_string(),
        source: sources.remove(0),
    })
}

fn parse_material_surface(node: &Node) -> Result<MaterialSurface, ParseError> {
    reject_unknown_fields(
        node,
        &[
            "color",
            "palette",
            "roughness",
            "metallic",
            "metal",
            "opacity",
            "emissive",
            "emission",
            "normal",
            "normal_strength",
        ],
    )?;
    reject_children(node)?;
    reject_alias_pair(node, "metallic", "metal")?;
    reject_alias_pair(node, "emissive", "emission")?;
    let (color, color_line) = field(node, "color")
        .ok_or_else(|| ParseError::missing(node.line, "Surface requires color"))?;
    let normal = field(node, "normal")
        .map(|(value, line)| parse_scalar_source(value, line))
        .transpose()?;
    if normal.is_none() && field(node, "normal_strength").is_some() {
        return Err(ParseError::missing(
            field(node, "normal_strength").unwrap().1,
            "normal_strength requires normal",
        ));
    }
    Ok(MaterialSurface {
        color: parse_color_source(color, color_line)?,
        palette: field(node, "palette")
            .map(|(value, line)| parse_palette_mode(value, line))
            .transpose()?
            .unwrap_or(PaletteMode::BaseOnly),
        roughness: optional_scalar(node, "roughness", 1.0)?,
        metallic: optional_scalar_alias(node, "metallic", "metal", 0.0)?,
        opacity: optional_scalar(node, "opacity", 1.0)?,
        emissive: optional_scalar_alias(node, "emissive", "emission", 0.0)?,
        normal,
        normal_strength: optional_f32(node, "normal_strength", 0.35)?.clamp(0.0, 8.0),
    })
}

fn parse_material_output(node: &Node) -> Result<MaterialOutput, ParseError> {
    if let Some((_, line)) = field(node, "height") {
        return Err(ParseError::invalid(
            line,
            "material Output uses value for scalar debugging; height is reserved for tile geometry",
        ));
    }
    reject_unknown_fields(node, &["value", "color", "space"])?;
    reject_children(node)?;
    let space = field(node, "space")
        .map(|(value, line)| parse_domain(value, line))
        .transpose()?
        .unwrap_or(Domain::Global);
    match (field(node, "value"), field(node, "color")) {
        (Some((value, line)), None) => Ok(MaterialOutput::Value {
            source: parse_scalar_source(value, line)?,
            space,
        }),
        (None, Some((value, line))) => Ok(MaterialOutput::Color {
            source: parse_color_source(value, line)?,
            space,
        }),
        _ => Err(ParseError::conflict(
            node.line,
            "material Output requires exactly one of value or color",
        )),
    }
}

fn validate_names_and_references(
    recipe: &Recipe,
    root: &Node,
    require_matching_output: bool,
) -> Result<(), ParseError> {
    let field_nodes = child_nodes(root, &["noise", "height", "value"]);
    let pattern_nodes = child_nodes(root, &["pattern"]);
    let mut fields = BTreeSet::new();
    let mut patterns = BTreeSet::new();
    for (index, field) in recipe.fields.iter().enumerate() {
        let name = field.name().to_ascii_lowercase();
        if !fields.insert(name.clone()) {
            return Err(ParseError::duplicate(
                field_nodes.get(index).map_or(root.line, |node| node.line),
                format!("duplicate field name '{}'", field.name()),
            ));
        }
    }
    for (index, pattern) in recipe.patterns.iter().enumerate() {
        let name = pattern.name.to_ascii_lowercase();
        if fields.contains(&name) || !patterns.insert(name) {
            return Err(ParseError::duplicate(
                pattern_nodes.get(index).map_or(root.line, |node| node.line),
                format!("duplicate definition name '{}'", pattern.name),
            ));
        }
    }
    for feature in &recipe.geometry {
        let name = match feature {
            GeometryFeature::Box(geometry_box) => &geometry_box.name,
        };
        fields.insert(format!("@geometry:{}", name.to_ascii_lowercase()));
    }
    let attachment_nodes = child_nodes(root, &["attachment"]);
    let light_nodes = child_nodes(root, &["light"]);
    let particle_nodes = child_nodes(root, &["particles"]);
    let mut attachments = BTreeSet::new();
    for (index, attachment) in recipe.attachments.iter().enumerate() {
        if !attachments.insert(attachment.name.to_ascii_lowercase()) {
            return Err(ParseError::duplicate(
                attachment_nodes
                    .get(index)
                    .map_or(root.line, |node| node.line),
                format!("duplicate Attachment '{}'", attachment.name),
            ));
        }
    }
    for (index, light) in recipe.lights.iter().enumerate() {
        if !attachments.contains(&light.attachment.to_ascii_lowercase()) {
            let node = light_nodes.get(index).copied().unwrap_or(root);
            return Err(ParseError::invalid(
                field_line(node, "attach"),
                format!(
                    "Light '{}' references unknown Attachment '{}'",
                    light.name, light.attachment
                ),
            ));
        }
    }
    for (index, particles) in recipe.particles.iter().enumerate() {
        if !attachments.contains(&particles.attachment.to_ascii_lowercase()) {
            let node = particle_nodes.get(index).copied().unwrap_or(root);
            return Err(ParseError::invalid(
                field_line(node, "attach"),
                format!(
                    "Particles '{}' references unknown Attachment '{}'",
                    particles.name, particles.attachment
                ),
            ));
        }
    }
    for (index, field) in recipe.fields.iter().enumerate() {
        let field_node = field_nodes.get(index).copied().unwrap_or(root);
        match field {
            FieldDefinition::Noise(noise) => {
                if let Domain::PatternLocal(name) = &noise.domain {
                    require_pattern(name, &patterns, field_line(field_node, "space"))?;
                }
                if let Some(IdSource::Pattern(name)) = &noise.key {
                    require_pattern(name, &patterns, field_line(field_node, "key"))?;
                }
            }
            FieldDefinition::Height(height) => {
                validate_scalar_source(
                    &height.source,
                    &fields,
                    &patterns,
                    field_line(field_node, "source"),
                )?;
                for (operation_index, operation) in height.operations.iter().enumerate() {
                    let operation_node = field_node
                        .children
                        .get(operation_index)
                        .unwrap_or(field_node);
                    match operation {
                        HeightOperation::Shape {
                            contrast,
                            bias,
                            plateau,
                            rim,
                        } => {
                            for (name, value) in [
                                ("contrast", contrast),
                                ("bias", bias),
                                ("plateau", plateau),
                                ("rim", rim),
                            ] {
                                validate_scalar_source(
                                    value,
                                    &fields,
                                    &patterns,
                                    field_line(operation_node, name),
                                )?;
                            }
                        }
                        HeightOperation::Combine { source, amount, .. } => {
                            validate_scalar_source(
                                source,
                                &fields,
                                &patterns,
                                field_line(operation_node, "source"),
                            )?;
                            validate_scalar_source(
                                amount,
                                &fields,
                                &patterns,
                                field_line(operation_node, "amount"),
                            )?;
                        }
                        HeightOperation::Clamp { min, max } => {
                            validate_scalar_source(
                                min,
                                &fields,
                                &patterns,
                                field_line(operation_node, "min"),
                            )?;
                            validate_scalar_source(
                                max,
                                &fields,
                                &patterns,
                                field_line(operation_node, "max"),
                            )?;
                        }
                        HeightOperation::Remap { .. }
                        | HeightOperation::Terrace { .. }
                        | HeightOperation::Invert => {}
                    }
                }
            }
            FieldDefinition::Value(value) => {
                validate_scalar_source(
                    &value.source,
                    &fields,
                    &patterns,
                    field_line(field_node, "source"),
                )?;
            }
        }
    }
    for (index, pattern) in recipe.patterns.iter().enumerate() {
        let pattern_node = pattern_nodes.get(index).copied().unwrap_or(root);
        let generator_node = pattern_node.children.first().unwrap_or(pattern_node);
        if let Domain::PatternLocal(name) = &pattern.domain {
            require_pattern(name, &patterns, field_line(generator_node, "space"))?;
        }
        if let Some(IdSource::Pattern(name)) = &pattern.key {
            require_pattern(name, &patterns, field_line(generator_node, "key"))?;
        }
        validate_scalar_source(
            &pattern.bevel,
            &fields,
            &patterns,
            field_line(generator_node, "bevel"),
        )?;
        if let Some(warp) = &pattern.warp {
            validate_scalar_source(
                &warp.source,
                &fields,
                &patterns,
                field_line(generator_node, "warp"),
            )?;
        }
        if let Some(perturb) = &pattern.perturb {
            validate_scalar_source(
                &perturb.source,
                &fields,
                &patterns,
                field_line(generator_node, "perturb"),
            )?;
            validate_scalar_source(
                &perturb.amount,
                &fields,
                &patterns,
                field_line(generator_node, "perturb_amount"),
            )?;
        }
        match &pattern.kind {
            PatternKind::Bricks {
                gap,
                rounding,
                rotation,
                falloff,
                ..
            } => {
                for (name, source) in [
                    ("gap", gap),
                    ("rounding", rounding),
                    ("rotation", rotation),
                    ("falloff", falloff),
                ] {
                    validate_scalar_source(
                        source,
                        &fields,
                        &patterns,
                        field_line(generator_node, name),
                    )?;
                }
            }
            PatternKind::Discs {
                jitter,
                radius,
                falloff,
                ..
            } => {
                for (name, source) in [("jitter", jitter), ("radius", radius), ("falloff", falloff)]
                {
                    validate_scalar_source(
                        source,
                        &fields,
                        &patterns,
                        field_line(generator_node, name),
                    )?;
                }
            }
            PatternKind::Voronoi { .. } => {}
        }
    }
    let colorize_node = child_node(root, "colorize");
    if let (Some(colorize), Some(colorize_node)) = (&recipe.colorize, colorize_node) {
        validate_scalar_source(
            &colorize.source,
            &fields,
            &patterns,
            field_line(colorize_node, "source"),
        )?;
    }
    if require_matching_output {
        let output_node = child_node(root, "output").unwrap_or(root);
        if let Domain::PatternLocal(name) = &recipe.output.space {
            require_pattern(name, &patterns, field_line(output_node, "space"))?;
        }
        validate_scalar_source(
            &recipe.output.height,
            &fields,
            &patterns,
            field_line(output_node, "height"),
        )?;
    }
    if require_matching_output
        && let (Some(colorize), Some(colorize_node)) = (&recipe.colorize, colorize_node)
        && colorize.source != recipe.output.height
    {
        return Err(ParseError::conflict(
            field_line(colorize_node, "source"),
            "Colorize.source and Output.height must reference the same field; height is the single source of truth",
        ));
    }
    if let Some(material_map) = &recipe.material_map {
        let material_map_node = child_node(root, "materialmap").unwrap_or(root);
        if let Domain::PatternLocal(name) = &material_map.space {
            require_pattern(name, &patterns, field_line(material_map_node, "space"))?;
        }
        let layer_nodes = child_nodes(material_map_node, &["layer"]);
        for (index, layer) in material_map.layers.iter().enumerate() {
            let layer_node = layer_nodes.get(index).copied().unwrap_or(material_map_node);
            if let Domain::PatternLocal(name) = &layer.space {
                require_pattern(name, &patterns, field_line(layer_node, "space"))?;
            }
            validate_scalar_source(
                &layer.mask,
                &fields,
                &patterns,
                field_line(layer_node, "mask"),
            )?;
        }
    }
    Ok(())
}

fn validate_material_names_and_references(
    material: &MaterialRecipe,
    root: &Node,
) -> Result<(), ParseError> {
    if let Some(line) = find_input_height_line(root) {
        return Err(ParseError::invalid(
            line,
            "Input.height is not available in materials; tiles own height and material masks",
        ));
    }
    let recipe = Recipe {
        name: material.name.clone(),
        wrap: material.wrap,
        seed: material.seed,
        fields: material.fields.clone(),
        patterns: material.patterns.clone(),
        ..Recipe::default()
    };
    validate_names_and_references(&recipe, root, false)?;

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
    let color_nodes = child_nodes(root, &["color"]);
    let mut colors = BTreeSet::new();
    for (index, color) in material.colors.iter().enumerate() {
        let normalized = color.name.to_ascii_lowercase();
        if !colors.insert(normalized) {
            return Err(ParseError::duplicate(
                color_nodes.get(index).map_or(root.line, |node| node.line),
                format!("duplicate color name '{}'", color.name),
            ));
        }
    }
    for (index, color) in material.colors.iter().enumerate() {
        let node = color_nodes.get(index).copied().unwrap_or(root);
        validate_color_source(
            &color.source,
            &colors,
            &fields,
            &patterns,
            field_line(node, "source"),
        )?;
    }

    let surface_node = child_node(root, "surface").unwrap_or(root);
    validate_color_source(
        &material.surface.color,
        &colors,
        &fields,
        &patterns,
        field_line(surface_node, "color"),
    )?;
    for (name, alias, source) in [
        ("roughness", None, &material.surface.roughness),
        ("metallic", Some("metal"), &material.surface.metallic),
        ("opacity", None, &material.surface.opacity),
        ("emissive", Some("emission"), &material.surface.emissive),
    ] {
        let line = field_line_with_alias(surface_node, name, alias);
        validate_material_scalar(source, &fields, &patterns, line)?;
    }
    if let Some(normal) = &material.surface.normal {
        validate_material_scalar(
            normal,
            &fields,
            &patterns,
            field_line(surface_node, "normal"),
        )?;
    }
    if let Some(output) = &material.output {
        let output_node = child_node(root, "output").unwrap_or(root);
        let space = match output {
            MaterialOutput::Value { space, .. } => space,
            MaterialOutput::Color { space, .. } => space,
        };
        if let Domain::PatternLocal(name) = space {
            require_pattern(name, &patterns, field_line(output_node, "space"))?;
        }
        match output {
            MaterialOutput::Value { source, .. } => validate_material_scalar(
                source,
                &fields,
                &patterns,
                field_line(output_node, "value"),
            )?,
            MaterialOutput::Color { source, .. } => validate_color_source(
                source,
                &colors,
                &fields,
                &patterns,
                field_line(output_node, "color"),
            )?,
        }
    }
    Ok(())
}

fn find_input_height_line(node: &Node) -> Option<usize> {
    node.fields
        .iter()
        .find_map(|(name, (value, line))| {
            (name != "name" && value.to_ascii_lowercase().contains("input.height")).then_some(*line)
        })
        .or_else(|| node.children.iter().find_map(find_input_height_line))
}

fn validate_material_scalar(
    source: &ScalarSource,
    fields: &BTreeSet<String>,
    patterns: &BTreeSet<String>,
    line: usize,
) -> Result<(), ParseError> {
    if scalar_uses_input_height(source) {
        return Err(ParseError::invalid(
            line,
            "Input.height is not available in materials; tiles own height and material masks",
        ));
    }
    validate_scalar_source(source, fields, patterns, line)
}

fn validate_color_source(
    source: &ColorSource,
    colors: &BTreeSet<String>,
    fields: &BTreeSet<String>,
    patterns: &BTreeSet<String>,
    line: usize,
) -> Result<(), ParseError> {
    match source {
        ColorSource::Exact(_) | ColorSource::Nearest(_) => Ok(()),
        ColorSource::Reference(name) => {
            if colors.contains(&name.to_ascii_lowercase()) {
                Ok(())
            } else {
                Err(ParseError::reference(
                    line,
                    format!("unknown color '{name}'"),
                ))
            }
        }
        ColorSource::Mix { a, b, factor } => {
            validate_color_source(a, colors, fields, patterns, line)?;
            validate_color_source(b, colors, fields, patterns, line)?;
            validate_material_scalar(factor, fields, patterns, line)
        }
    }
}

fn scalar_uses_input_height(source: &ScalarSource) -> bool {
    match source {
        ScalarSource::InputHeight => true,
        ScalarSource::Unary { source, .. } => scalar_uses_input_height(source),
        ScalarSource::Binary { left, right, .. } => {
            scalar_uses_input_height(left) || scalar_uses_input_height(right)
        }
        ScalarSource::Clamp { source, min, max } => {
            scalar_uses_input_height(source)
                || scalar_uses_input_height(min)
                || scalar_uses_input_height(max)
        }
        ScalarSource::Mix { a, b, factor } => {
            scalar_uses_input_height(a)
                || scalar_uses_input_height(b)
                || scalar_uses_input_height(factor)
        }
        ScalarSource::Smoothstep { min, max, source } => {
            scalar_uses_input_height(min)
                || scalar_uses_input_height(max)
                || scalar_uses_input_height(source)
        }
        ScalarSource::Constant(_)
        | ScalarSource::Coordinate(_)
        | ScalarSource::Field(_)
        | ScalarSource::Pattern { .. }
        | ScalarSource::Geometry { .. }
        | ScalarSource::RandomId { .. }
        | ScalarSource::Wave { .. } => false,
    }
}

fn validate_scalar_source(
    source: &ScalarSource,
    fields: &BTreeSet<String>,
    patterns: &BTreeSet<String>,
    line: usize,
) -> Result<(), ParseError> {
    match source {
        ScalarSource::Field(name) => {
            if !fields.contains(&name.to_ascii_lowercase()) {
                return Err(ParseError::reference(
                    line,
                    format!("unknown field '{name}'"),
                ));
            }
        }
        ScalarSource::Pattern { name, .. } => require_pattern(name, patterns, line)?,
        ScalarSource::Geometry { name, .. } => {
            if !fields.contains(&format!("@geometry:{}", name.to_ascii_lowercase())) {
                return Err(ParseError::reference(
                    line,
                    format!("unknown Geometry feature '{name}'"),
                ));
            }
        }
        ScalarSource::RandomId { id, .. } => {
            if let IdSource::Pattern(name) = id {
                require_pattern(name, patterns, line)?;
            }
        }
        ScalarSource::Unary { source, .. } => {
            validate_scalar_source(source, fields, patterns, line)?;
        }
        ScalarSource::Binary { left, right, .. } => {
            validate_scalar_source(left, fields, patterns, line)?;
            validate_scalar_source(right, fields, patterns, line)?;
        }
        ScalarSource::Clamp { source, min, max } => {
            validate_scalar_source(source, fields, patterns, line)?;
            validate_scalar_source(min, fields, patterns, line)?;
            validate_scalar_source(max, fields, patterns, line)?;
        }
        ScalarSource::Mix { a, b, factor } => {
            validate_scalar_source(a, fields, patterns, line)?;
            validate_scalar_source(b, fields, patterns, line)?;
            validate_scalar_source(factor, fields, patterns, line)?;
        }
        ScalarSource::Smoothstep { min, max, source } => {
            validate_scalar_source(min, fields, patterns, line)?;
            validate_scalar_source(max, fields, patterns, line)?;
            validate_scalar_source(source, fields, patterns, line)?;
        }
        ScalarSource::Constant(_)
        | ScalarSource::Coordinate(_)
        | ScalarSource::InputHeight
        | ScalarSource::Wave { .. } => {}
    }
    Ok(())
}

fn require_pattern(name: &str, patterns: &BTreeSet<String>, line: usize) -> Result<(), ParseError> {
    if patterns.contains(&name.to_ascii_lowercase()) {
        Ok(())
    } else {
        Err(ParseError::reference(
            line,
            format!("unknown pattern '{name}'"),
        ))
    }
}

fn child_nodes<'a>(root: &'a Node, kinds: &[&str]) -> Vec<&'a Node> {
    root.children
        .iter()
        .filter(|node| {
            let (kind, _) = declaration(&node.name);
            kinds.contains(&kind.as_str())
        })
        .collect()
}

fn child_node<'a>(root: &'a Node, expected_kind: &str) -> Option<&'a Node> {
    root.children.iter().find(|node| {
        let (kind, _) = declaration(&node.name);
        kind == expected_kind
    })
}

fn field_line(node: &Node, name: &str) -> usize {
    field(node, name).map_or(node.line, |(_, line)| line)
}

fn field_line_with_alias(node: &Node, name: &str, alias: Option<&str>) -> usize {
    field(node, name)
        .or_else(|| alias.and_then(|alias| field(node, alias)))
        .map_or(node.line, |(_, line)| line)
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
                    ParseError::invalid(line, format!("invalid number '{}'", &value[start..cursor]))
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
                return Err(ParseError::invalid(
                    line,
                    format!("unexpected character '{character}' in scalar expression"),
                ));
            }
        };
        tokens.push(token);
    }
    if tokens.is_empty() {
        return Err(ParseError::invalid(line, "scalar expression is empty"));
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
                        return Err(ParseError::invalid(
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
                        return Err(ParseError::invalid(
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
            _ => Err(ParseError::unknown(
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
        if channel.eq_ignore_ascii_case("distance") {
            return Ok(ScalarSource::Geometry {
                name: name.to_string(),
                channel: GeometryChannel::Distance,
            });
        }
        let channel = match channel.to_ascii_lowercase().as_str() {
            "height" => PatternChannel::Height,
            "edge" => PatternChannel::Edge,
            "center" => PatternChannel::Center,
            "id" => {
                return Err(ParseError::unknown(
                    line,
                    "pattern IDs are not scalar fields; use Random(Pattern.id, min, max)",
                ));
            }
            _ => {
                return Err(ParseError::invalid(
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
        _ => Err(ParseError::invalid(
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
        ParseError::invalid(line, "expected Id or a stable pattern ID such as Stones.id")
    })?;
    validate_identifier(name, line)?;
    if !channel.eq_ignore_ascii_case("id") {
        return Err(ParseError::invalid(
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
        ParseError::invalid(
            line,
            "space must be Global or a pattern local space such as Stones.local",
        )
    })?;
    validate_identifier(name, line)?;
    if !channel.eq_ignore_ascii_case("local") {
        return Err(ParseError::invalid(
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
        _ => Err(ParseError::invalid(
            line,
            format!("unknown palette family '{value}'"),
        )),
    }
}

fn parse_noise_kind(value: &str, line: usize) -> Result<NoiseKind, ParseError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "value" => Ok(NoiseKind::Value),
        "gradient" | "perlin" => Ok(NoiseKind::Gradient),
        _ => Err(ParseError::invalid(
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
        _ => Err(ParseError::invalid(
            line,
            "noise fractal must be FBm, Ridged, Billow, or Turbulence",
        )),
    }
}

fn parse_color_source(value: &str, line: usize) -> Result<ColorSource, ParseError> {
    let value = value.trim();
    if value.starts_with('#')
        || value
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("F3("))
    {
        return parse_authored_color(value, line).map(ColorSource::Exact);
    }
    if value.ends_with(')')
        && let Ok((call, arguments)) = parse_call(value, line)
    {
        let arguments = split_arguments(arguments);
        return match call.to_ascii_lowercase().as_str() {
            "mix" if arguments.len() == 3 => Ok(ColorSource::Mix {
                a: Box::new(parse_color_source(arguments[0], line)?),
                b: Box::new(parse_color_source(arguments[1], line)?),
                factor: parse_scalar_source(arguments[2], line)?,
            }),
            "nearest" if arguments.len() == 1 => {
                parse_authored_color(arguments[0], line).map(ColorSource::Nearest)
            }
            "exact" if arguments.len() == 1 => {
                parse_authored_color(arguments[0], line).map(ColorSource::Exact)
            }
            "mix" => Err(ParseError::invalid(
                line,
                "color Mix requires two colors and one scalar mask",
            )),
            "nearest" | "exact" => Err(ParseError::invalid(
                line,
                format!("{call} requires exactly one color"),
            )),
            _ => Err(ParseError::unknown(
                line,
                format!("unknown color function '{call}'"),
            )),
        };
    }
    validate_identifier(value, line)?;
    Ok(ColorSource::Reference(value.to_string()))
}

fn parse_authored_color(value: &str, line: usize) -> Result<[u8; 4], ParseError> {
    let value = value.trim().trim_matches('"');
    if value
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("F3("))
    {
        let (call, arguments) = parse_call(value, line)?;
        if !call.eq_ignore_ascii_case("F3") {
            return Err(ParseError::invalid(line, "expected F3(r, g, b)"));
        }
        let arguments = split_arguments(arguments);
        if arguments.len() != 3 {
            return Err(ParseError::invalid(
                line,
                "F3 color requires three normalized components",
            ));
        }
        let components = [
            parse_number(arguments[0], line)?,
            parse_number(arguments[1], line)?,
            parse_number(arguments[2], line)?,
        ];
        if components
            .iter()
            .any(|component| !component.is_finite() || !(0.0..=1.0).contains(component))
        {
            return Err(ParseError::invalid(
                line,
                "F3 color components must be within 0..1",
            ));
        }
        return Ok([
            (components[0] * 255.0).round() as u8,
            (components[1] * 255.0).round() as u8,
            (components[2] * 255.0).round() as u8,
            255,
        ]);
    }
    if let Some(hex) = value.strip_prefix('#') {
        if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ParseError::invalid(
                line,
                "color must be #RRGGBB, #RRGGBBAA, F3(r, g, b), or a named color",
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
            return Err(ParseError::invalid(
                line,
                format!("unknown color '{value}'; use #RRGGBB, F3(r, g, b), or a standard name"),
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
        _ => Err(ParseError::invalid(
            line,
            "wrap must be Clamp, Repeat, or Mirror",
        )),
    }
}

fn parse_recipe_placement(node: &Node) -> Result<RecipePlacement, ParseError> {
    let Some((value, line)) = field(node, "placement") else {
        return Ok(RecipePlacement::Surface);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "surface" => Ok(RecipePlacement::Surface),
        "fixture" => Ok(RecipePlacement::Fixture),
        _ => Err(ParseError::invalid(
            line,
            "placement must be Surface or Fixture",
        )),
    }
}

fn required_scalar(node: &Node, name: &str) -> Result<ScalarSource, ParseError> {
    let (value, line) = field(node, name)
        .ok_or_else(|| ParseError::missing(node.line, format!("{} requires {name}", node.name)))?;
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
        Err(ParseError::conflict(
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

fn optional_i3(node: &Node, name: &str, default: [u32; 3]) -> Result<[u32; 3], ParseError> {
    field(node, name)
        .map(|(value, line)| parse_i3(value, line))
        .unwrap_or(Ok(default))
}

fn optional_f3(node: &Node, name: &str, default: [f32; 3]) -> Result<[f32; 3], ParseError> {
    field(node, name)
        .map(|(value, line)| parse_f3(value, line))
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
        return Err(ParseError::invalid(
            field(node, name).map(|(_, line)| line).unwrap_or(node.line),
            format!("{name} values must be finite and greater than zero"),
        ));
    }
    Ok(result)
}

fn parse_i2(value: &str, line: usize) -> Result<[u32; 2], ParseError> {
    let (call, arguments) = parse_call(value, line)?;
    if !call.eq_ignore_ascii_case("I2") {
        return Err(ParseError::invalid(line, "expected I2(x, y)"));
    }
    let args = split_arguments(arguments);
    if args.len() != 2 {
        return Err(ParseError::invalid(
            line,
            "I2 requires two positive integers",
        ));
    }
    let result = [parse_u32(args[0], line)?, parse_u32(args[1], line)?];
    if result.contains(&0) {
        return Err(ParseError::invalid(
            line,
            "I2 values must be greater than zero",
        ));
    }
    Ok(result)
}

fn parse_f2(value: &str, line: usize) -> Result<[f32; 2], ParseError> {
    let (call, arguments) = parse_call(value, line)?;
    if !call.eq_ignore_ascii_case("F2") {
        return Err(ParseError::invalid(line, "expected F2(x, y)"));
    }
    let args = split_arguments(arguments);
    if args.len() != 2 {
        return Err(ParseError::invalid(line, "F2 requires two numbers"));
    }
    Ok([parse_number(args[0], line)?, parse_number(args[1], line)?])
}

fn parse_i3(value: &str, line: usize) -> Result<[u32; 3], ParseError> {
    let (call, arguments) = parse_call(value, line)?;
    if !call.eq_ignore_ascii_case("I3") {
        return Err(ParseError::invalid(line, "expected I3(x, y, z)"));
    }
    let args = split_arguments(arguments);
    if args.len() != 3 {
        return Err(ParseError::invalid(
            line,
            "I3 requires three positive integers",
        ));
    }
    let result = [
        parse_u32(args[0], line)?,
        parse_u32(args[1], line)?,
        parse_u32(args[2], line)?,
    ];
    if result.contains(&0) {
        return Err(ParseError::invalid(
            line,
            "I3 values must be greater than zero",
        ));
    }
    Ok(result)
}

fn parse_f3(value: &str, line: usize) -> Result<[f32; 3], ParseError> {
    let (call, arguments) = parse_call(value, line)?;
    if !call.eq_ignore_ascii_case("F3") {
        return Err(ParseError::invalid(line, "expected F3(x, y, z)"));
    }
    let args = split_arguments(arguments);
    if args.len() != 3 {
        return Err(ParseError::invalid(line, "F3 requires three numbers"));
    }
    Ok([
        parse_number(args[0], line)?,
        parse_number(args[1], line)?,
        parse_number(args[2], line)?,
    ])
}

fn parse_call(value: &str, line: usize) -> Result<(&str, &str), ParseError> {
    let open = value.find('(').ok_or_else(|| {
        ParseError::invalid(line, format!("expected function expression, got '{value}'"))
    })?;
    if !value.ends_with(')') {
        return Err(ParseError::invalid(
            line,
            "function expression is missing ')'",
        ));
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
        return Err(ParseError::invalid(
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
        Err(ParseError::invalid(line, "expected a quoted string"))
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
        return Err(ParseError::invalid(
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
        .map_err(|_| ParseError::invalid(line, format!("'{value}' is not a number")))
}

fn parse_u32(value: &str, line: usize) -> Result<u32, ParseError> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| ParseError::invalid(line, format!("'{value}' is not a positive integer")))
}

fn parse_u64(value: &str, line: usize) -> Result<u64, ParseError> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| ParseError::invalid(line, format!("'{value}' is not a positive integer")))
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
                _ => Err(ParseError::invalid(
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
            return Err(ParseError::unknown(
                *line,
                format!("unknown field '{key}' in {} block", node.name),
            ));
        }
    }
    Ok(())
}

fn unsupported_field_warnings(node: &Node, allowed: &[&str]) -> Vec<ParseWarning> {
    let mut warnings = node
        .fields
        .iter()
        .filter(|(key, _)| !allowed.contains(&key.as_str()))
        .map(|(key, (_, line))| ParseWarning {
            line: *line,
            column: 1,
            message: format!(
                "field '{key}' is not supported by {} and was ignored",
                node.name
            ),
            source_line: None,
            source_name: None,
        })
        .collect::<Vec<_>>();
    warnings.sort_by_key(|warning| warning.line);
    warnings
}

fn reject_children(node: &Node) -> Result<(), ParseError> {
    if let Some(child) = node.children.first() {
        Err(ParseError::conflict(
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
        assert_eq!(recipe.fields.len(), 2);
        assert!(matches!(
            &recipe
                .patterns
                .iter()
                .find(|pattern| pattern.name == "Kerbs")
                .unwrap()
                .kind,
            PatternKind::Discs { .. }
        ));
        let wall = recipe
            .patterns
            .iter()
            .find(|pattern| pattern.name == "Wall")
            .unwrap();
        assert!(matches!(
            &wall.kind,
            PatternKind::Bricks {
                rounding: ScalarSource::Constant(_),
                ..
            }
        ));
        assert!(matches!(
            &wall.perturb,
            Some(Perturb {
                source: ScalarSource::Binary { .. },
                ..
            })
        ));
        assert!(recipe.colorize.is_none());
    }

    #[test]
    fn parses_generic_repeated_subtractive_box_geometry() {
        let recipe = parse_recipe(
            r#"
Tile
    name = "Carved Wall"
    blocking = true

    Geometry
        Box Recess
            operation = Subtract
            surface = niche-stone
            position = F3(0.12, 0.76, 0.0)
            size = F3(0.76, 1.06, 0.46)
            repeat = I3(1, 2, 1)
            spacing = F3(0.0, 1.5, 0.0)

    Height Joint
        source = Abs(Recess.distance)

    Output
        height = Joint
"#,
        )
        .unwrap();

        assert!(recipe.blocking);
        assert_eq!(recipe.geometry.len(), 1);
        let GeometryFeature::Box(geometry_box) = &recipe.geometry[0];
        assert_eq!(geometry_box.name, "Recess");
        assert_eq!(geometry_box.operation, GeometryOperation::Subtract);
        assert_eq!(geometry_box.surface, "niche-stone");
        assert_eq!(geometry_box.position, [0.12, 0.76, 0.0]);
        assert_eq!(geometry_box.size, [0.76, 1.06, 0.46]);
        assert_eq!(geometry_box.repeat, [1, 2, 1]);
        assert_eq!(geometry_box.spacing, [0.0, 1.5, 0.0]);
        assert!(matches!(
            &recipe.fields[0],
            FieldDefinition::Height(HeightField {
                source: ScalarSource::Unary {
                    source,
                    op: UnaryOperator::Abs,
                },
                ..
            }) if matches!(
                source.as_ref(),
                ScalarSource::Geometry {
                    name,
                    channel: GeometryChannel::Distance,
                } if name == "Recess"
            )
        ));
    }

    #[test]
    fn parses_fixture_placement_and_rejects_unknown_placement() {
        let fixture = parse_recipe(
            r#"
Tile
    name = "Torch"
    placement = Fixture

    Output
        height = 0.0
"#,
        )
        .unwrap();
        assert_eq!(fixture.placement, RecipePlacement::Fixture);

        let error = parse_recipe(
            r#"
Tile
    placement = Floating
    Output
        height = 0.0
"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("Surface or Fixture"));
    }

    #[test]
    fn parses_named_placement_effects() {
        let recipe = parse_recipe(
            r#"
Tile
    Attachment Flame
        position = F3(0.5, 1.4, -0.3)
        direction = F3(0.0, 1.0, 0.0)

    Light Glow
        attach = Flame
        color = #ff9a45
        intensity = 1.7
        range = 5.0
        flicker = 0.2

    Particles Fire
        attach = Flame
        direction = F3(0.0, 1.0, 0.0)
        spread = 0.5
        rate = 24.0
        color_ramp = #fff2a8, #ffc14f, #f0641f, #401008
        lifetime = F2(0.3, 0.8)
        radius = F2(0.02, 0.07)
        speed = F2(0.3, 0.7)
        flame_base = true

    Output
        height = 0.5
"#,
        )
        .unwrap();

        assert_eq!(recipe.attachments[0].name, "Flame");
        assert_eq!(recipe.lights[0].attachment, "Flame");
        assert_eq!(recipe.lights[0].color, [255, 154, 69, 255]);
        assert_eq!(recipe.particles[0].attachment, "Flame");
        assert_eq!(recipe.particles[0].color_ramp.unwrap()[3], [64, 16, 8, 255]);
        assert!(recipe.particles[0].flame_base);
    }

    #[test]
    fn placement_effects_require_a_known_attachment() {
        let error = parse_recipe(
            r#"
Tile
    Light Glow
        attach = Missing

    Output
        height = 0.5
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("unknown Attachment 'Missing'"));
    }

    #[test]
    fn parses_common_warp_and_perturb_for_every_pattern_generator() {
        let recipe = parse_recipe(
            r#"
Tile
    Noise ShapeNoise
        scale = F2(3.0, 3.0)

    Pattern Masonry
        Bricks
            warp = ShapeNoise
            warp_amount = 0.01
            perturb = ShapeNoise
            perturb_amount = 0.02

    Pattern Cells
        Voronoi
            warp = ShapeNoise
            warp_amount = 0.01
            perturb = ShapeNoise
            perturb_amount = 0.02

    Pattern Dots
        Discs
            warp = ShapeNoise
            warp_amount = 0.01
            perturb = ShapeNoise
            perturb_amount = 0.02

    Output
        height = Masonry.height
"#,
        )
        .unwrap();

        assert_eq!(recipe.patterns.len(), 3);
        assert!(
            recipe
                .patterns
                .iter()
                .all(|pattern| pattern.warp.is_some() && pattern.perturb.is_some())
        );
    }

    #[test]
    fn output_can_select_a_pattern_local_context() {
        let recipe = parse_recipe(
            r#"
Tile
    Noise LocalWarp
        key = Id

    Pattern Wall
        Bricks
            perturb = LocalWarp

    Output
        height = LocalWarp
        space = Wall.local
"#,
        )
        .unwrap();

        assert_eq!(
            recipe.output.space,
            Domain::PatternLocal("Wall".to_string())
        );
    }

    #[test]
    fn output_rejects_an_unknown_pattern_local_context() {
        let error = parse_recipe(
            r#"
Tile
    Noise Grain

    Output
        height = Grain
        space = Missing.local
"#,
        )
        .unwrap_err();

        assert_eq!(error.code, ParseErrorCode::UnknownReference);
        assert_eq!(error.line, 7);
    }

    #[test]
    fn generator_specific_leftovers_warn_and_do_not_block_pattern_switching() {
        let recipe = parse_recipe(
            r#"
Tile
    Pattern Shape
        Discs
            columns = 6
            rows = 9
            gap = 0.02
            bevel = 0.2
            cells = I2(6, 9)
            radius = 0.42

    Output
        height = Shape.height
"#,
        )
        .unwrap();
        let pattern = &recipe.patterns[0];

        assert_eq!(pattern.bevel, ScalarSource::Constant(0.2));
        assert_eq!(pattern.warnings.len(), 3);
        assert_eq!(
            pattern
                .warnings
                .iter()
                .map(|warning| warning.line)
                .collect::<Vec<_>>(),
            vec![5, 6, 7]
        );
        assert!(
            pattern
                .warnings
                .iter()
                .all(|warning| warning.stable_code() == "PRW0001")
        );
        assert_eq!(
            pattern.warnings[0].source_line.as_deref(),
            Some("            columns = 6")
        );
    }

    #[test]
    fn malformed_supported_pattern_fields_remain_errors() {
        let error = parse_recipe(
            r#"
Tile
    Pattern Shape
        Discs
            radius = definitely_not_a_number

    Output
        height = Shape.height
"#,
        )
        .unwrap_err();

        assert_eq!(error.code, ParseErrorCode::UnknownReference);
        assert_eq!(error.line, 5);
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
        assert_eq!(
            named.colorize.as_ref().unwrap().base,
            Some([58, 36, 24, 255])
        );
        assert_eq!(named.colorize.as_ref().unwrap().brightness, [-0.18, 0.22]);

        let hex = parse_authored_color("\"#3A2418CC\"", 1).unwrap();
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

    Value Tone
        source = Abs(Sin(V * 6.0 + U + Grain))

    Color Dark
        nearest = #3a2418

    Color Light
        nearest = #9a6840

    Color Oak
        source = Mix(Dark, Light, Tone)

    Surface
        color = Oak
        roughness = Clamp(0.4 + Grain * 0.4, 0.0, 1.0)
        metal = 0.0
        opacity = 1.0
        emission = 0.0
        normal = Tone
        normal_strength = 0.25

Material marble
    Color Marble
        nearest = F3(0.75, 0.75, 0.72)

    Surface
        color = Marble
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
        assert!(document.materials[1].surface.normal.is_none());
        assert_eq!(document.materials[0].colors.len(), 3);
    }

    #[test]
    fn parses_material_color_mix_surface_and_debug_output() {
        let document = parse_material_document(
            r#"
Material stone
    Noise Grain
        scale = F2(4, 4)

    Color Dark
        nearest = #324524

    Color Light
        base = F3(0.6, 0.7, 0.5)

    Color Mixed
        source = Mix(Dark, Light, Smoothstep(0.2, 0.8, Grain))

    Surface
        color = Mixed
        roughness = Grain
        normal = Grain
        normal_strength = 0.2

    Output
        value = Grain
        space = Global
"#,
        )
        .unwrap();
        let material = &document.materials[0];

        assert_eq!(material.colors.len(), 3);
        assert!(matches!(
            material.colors[0].source,
            ColorSource::Nearest([0x32, 0x45, 0x24, 255])
        ));
        assert!(matches!(material.colors[2].source, ColorSource::Mix { .. }));
        assert!(matches!(
            material.output,
            Some(MaterialOutput::Value { .. })
        ));
    }

    #[test]
    fn material_rejects_height_fields_with_value_guidance() {
        let error = parse_material_document(
            r#"
Material stone
    Height Tone
        source = 0.5

    Surface
        color = #777777
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("Height is tile geometry"));
        assert!(error.message.contains("use Value"));
    }

    #[test]
    fn material_rejects_tile_height_coupling() {
        let error = parse_material_document(
            r#"
Material stone
    Surface
        color = #777777
        roughness = Input.height
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("Input.height is not available"));
    }

    #[test]
    fn parses_tile_material_map_layers() {
        let recipe = parse_recipe(
            r#"
Tile
    Pattern Wall
        Bricks

    MaterialMap
        base = materials/mortar
        space = Global
        tiling = F2(2.0, 3.0)

        Layer
            material = materials/stone
            mask = Pow(Wall.height, 1.6)
            space = Wall.local
            tiling = F2(0.5, 1.5)

    Output
        height = Wall.height
"#,
        )
        .unwrap();
        let material_map = recipe.material_map.unwrap();

        assert_eq!(material_map.base, "materials/mortar");
        assert_eq!(material_map.space, Domain::Global);
        assert_eq!(material_map.tiling, [2.0, 3.0]);
        assert_eq!(material_map.layers.len(), 1);
        assert_eq!(material_map.layers[0].material, "materials/stone");
        assert_eq!(
            material_map.layers[0].space,
            Domain::PatternLocal("Wall".to_string())
        );
        assert_eq!(material_map.layers[0].tiling, [0.5, 1.5]);
    }

    #[test]
    fn material_map_rejects_unknown_local_space() {
        let error = parse_recipe(
            r#"
Tile
    MaterialMap
        base = materials/wood
        space = Missing.local

    Output
        height = 0.5
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("unknown pattern 'Missing'"));
    }

    #[test]
    fn material_map_rejects_non_positive_tiling() {
        let error = parse_recipe(
            r#"
Tile
    MaterialMap
        base = materials/wood
        tiling = F2(0.0, 1.0)

    Output
        height = 0.5
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("tiling values must be"));
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
        assert!(recipe.colorize.is_none());
    }

    #[test]
    fn tile_without_output_uses_the_last_scalar_producing_block() {
        let recipe = parse_recipe(
            r#"
Tile
    Noise Grain
        scale = F2(2.0, 2.0)

    Height Surface
        source = Grain
"#,
        )
        .unwrap();
        assert_eq!(
            recipe.output.height,
            ScalarSource::Field("Surface".to_string())
        );
        assert!(recipe.colorize.is_none());
    }

    #[test]
    fn tile_without_output_or_a_scalar_block_has_a_stable_error() {
        let error = parse_recipe(
            r#"Tile
    name = "Empty"
"#,
        )
        .unwrap_err();
        assert_eq!(error.code, ParseErrorCode::MissingRequired);
        assert_eq!(error.line, 1);
        assert!(error.message.contains("scalar-producing"));
    }

    #[test]
    fn rejects_duplicate_material_ids() {
        let error = parse_material_document(
            r#"
Material stone
    Surface
        color = #777777

Material stone
    Surface
        color = #777777
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
            parse_document("Material stone\n    Surface\n        color = #777777\n").unwrap(),
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
        assert_eq!(
            recipe.colorize.as_ref().unwrap().palette,
            PaletteMode::BaseOnly
        );
        assert_eq!(recipe.colorize.as_ref().unwrap().steps, 128);

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

    #[test]
    fn diagnostics_keep_stable_codes_and_reference_lines() {
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
        .unwrap_err()
        .with_source_name("broken.recipe");

        assert_eq!(error.code, ParseErrorCode::UnknownReference);
        assert_eq!(error.stable_code(), "PR0008");
        assert_eq!(error.line, 4);
        assert_eq!(error.column, 9);
        assert_eq!(
            error.source_line.as_deref(),
            Some("        source = Missing.height")
        );

        let diagnostic = error.to_string();
        assert!(diagnostic.contains("error[PR0008]: unknown pattern 'Missing'"));
        assert!(diagnostic.contains("--> broken.recipe:4:9"));
        assert!(diagnostic.contains("4 |         source = Missing.height"));
        assert!(diagnostic.ends_with("|         ^"));
    }

    #[test]
    fn diagnostics_report_the_actual_duplicate_and_unknown_field_lines() {
        let duplicate = parse_material_document(
            "Material stone\n    Surface\n        color = #777777\n\
             Material stone\n    Surface\n        color = #777777\n",
        )
        .unwrap_err();
        assert_eq!(duplicate.code, ParseErrorCode::DuplicateDefinition);
        assert_eq!(duplicate.line, 4);

        let unknown = parse_recipe(
            r#"Tile
    mystery = 1
    Colorize
        source = 0.5
    Output
        height = 0.5
"#,
        )
        .unwrap_err();
        assert_eq!(unknown.code, ParseErrorCode::UnknownConstruct);
        assert_eq!(unknown.line, 2);
        assert_eq!(unknown.source_line.as_deref(), Some("    mystery = 1"));
    }

    #[test]
    fn diagnostics_never_expose_line_zero() {
        let error = parse_document("").unwrap_err();
        assert_eq!(error.code, ParseErrorCode::Document);
        assert_eq!(error.line, 1);
        assert_eq!(error.column, 1);
    }
}
