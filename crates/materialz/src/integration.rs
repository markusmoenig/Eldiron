//! GPU-independent editable assets and persistent instance parameters for hosts.
use crate::{
    format,
    ir::Program,
    metadata::{Kernel, Metadata, ValueType},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt};

pub type Result<T> = std::result::Result<T, Diagnostic>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub asset: String,
    pub material: Option<String>,
    pub row: Option<String>,
    pub tile: Option<String>,
    pub parameter: Option<String>,
}
impl Location {
    pub fn asset(asset: &str) -> Self {
        Self {
            asset: asset.into(),
            material: None,
            row: None,
            tile: None,
            parameter: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub operation: String,
    pub location: Location,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}
impl Diagnostic {
    pub fn new(operation: &str, location: Location, message: impl ToString) -> Self {
        Self {
            severity: Severity::Error,
            operation: operation.into(),
            location,
            message: message.to_string(),
        }
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}]: {}",
            self.operation, self.location.asset, self.message
        )
    }
}
impl std::error::Error for Diagnostic {}

/// Scoped to the owning asset, never to row order or a GPU buffer offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParameterKey {
    pub asset: String,
    pub tile: String,
    pub parameter: String,
}
impl ParameterKey {
    pub fn new(asset: &str, tile: &str, parameter: &str) -> Self {
        Self {
            asset: asset.into(),
            tile: tile.into(),
            parameter: parameter.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterOverride {
    pub key: ParameterKey,
    pub value: Vec<f32>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaletteBinding {
    pub key: ParameterKey,
    pub index: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceRecord {
    pub id: String,
    pub overrides: Vec<ParameterOverride>,
    pub palette_bindings: Vec<PaletteBinding>,
    /// Row-major affine UV mapping: [xx, xy, yx, yy, offset_x, offset_y].
    pub uv_transform: [f32; 6],
}
impl InstanceRecord {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.into(),
            overrides: vec![],
            palette_bindings: vec![],
            uv_transform: [1., 0., 0., 1., 0., 0.],
        }
    }
    pub fn set(&mut self, key: ParameterKey, value: Vec<f32>) {
        if let Some(p) = self.overrides.iter_mut().find(|p| p.key == key) {
            p.value = value;
        } else {
            self.overrides.push(ParameterOverride { key, value });
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct EditableDocument {
    pub id: String,
    pub project: format::Project,
}
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub key: ParameterKey,
    pub location: Location,
    /// Current editor position only; never persist this as binding identity.
    pub tile_index: usize,
    pub source_id: String,
    pub definition: crate::metadata::Parameter,
    pub binding: format::Binding,
}
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub id: String,
    pub help_key: String,
    pub metadata: Metadata,
    pub source: String,
    pub requires_input: bool,
    pub accepts_input: bool,
}

/// Complete, flat dependency closure. All documents remain editable on reopen.
#[derive(Debug, Clone)]
pub struct EditableMaterialAsset {
    pub root: EditableDocument,
    pub selected_material: String,
    pub dependencies: Vec<EditableDocument>,
    pub instances: Vec<InstanceRecord>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    version: u32,
    source_library: String,
    root: DocumentRecord,
    selected_material: String,
    dependencies: Vec<DocumentRecord>,
    instances: Vec<InstanceRecord>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentRecord {
    id: String,
    recipe: String,
}

/// Includes release provenance and a fingerprint of the actual built-in definitions.
/// Bundle compatibility depends on the schema and definition fingerprint; a release
/// number alone does not change the material language.
pub fn source_library_version() -> String {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION
        .get_or_init(|| {
            let mut hash = 0xcbf29ce484222325u64;
            for k in Kernel::ALL {
                for byte in k
                    .source_id()
                    .bytes()
                    .chain(crate::tilelang::source(k).bytes())
                {
                    hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
                }
            }
            format!("materialz-{}-{hash:016x}", env!("CARGO_PKG_VERSION"))
        })
        .clone()
}
pub fn source_catalog() -> Vec<SourceInfo> {
    Kernel::ALL
        .into_iter()
        .map(|k| SourceInfo {
            id: k.source_id().into(),
            help_key: k.help_key(),
            metadata: k.metadata(),
            source: crate::tilelang::source(k).into(),
            requires_input: k.requires_input(),
            accepts_input: k.accepts_input(),
        })
        .collect()
}
// Compilation combines documents; authoring identities remain scoped to their
// owning asset. Length-prefixing avoids delimiter collisions in arbitrary IDs.
fn scoped_tile(asset: &str, tile: &str) -> String {
    format!("{}:{asset}{tile}", asset.len())
}
impl EditableDocument {
    pub fn parse(id: &str, source: &str) -> Result<Self> {
        if id.is_empty() {
            return Err(Diagnostic::new(
                "parse",
                Location::asset(id),
                "Asset ID cannot be empty",
            ));
        }
        Ok(Self {
            id: id.into(),
            project: format::parse(source)
                .map_err(|e| Diagnostic::new("parse", Location::asset(id), e))?,
        })
    }
    pub fn text(&self) -> String {
        format::write(&self.project)
    }
}
impl EditableMaterialAsset {
    /// Parses authoring data without lowering algorithms or creating a GPU device.
    pub fn parse(id: &str, source: &str, selected_material: &str) -> Result<Self> {
        Ok(Self {
            root: EditableDocument::parse(id, source)?,
            selected_material: selected_material.into(),
            dependencies: vec![],
            instances: vec![],
        })
    }
    pub fn document(&self) -> &format::Project {
        &self.root.project
    }
    pub fn source_catalog(&self) -> Vec<SourceInfo> {
        source_catalog()
    }
    pub fn documents(&self) -> impl Iterator<Item = &EditableDocument> {
        std::iter::once(&self.root).chain(self.dependencies.iter())
    }
    pub fn save(&self) -> Result<String> {
        let record = |d: &EditableDocument| DocumentRecord {
            id: d.id.clone(),
            recipe: d.text(),
        };
        serde_json::to_string_pretty(&Bundle {
            version: 1,
            source_library: source_library_version(),
            root: record(&self.root),
            selected_material: self.selected_material.clone(),
            dependencies: self.dependencies.iter().map(record).collect(),
            instances: self.instances.clone(),
        })
        .map_err(|e| self.error("save", e))
    }
    pub fn reopen(text: &str) -> Result<Self> {
        let bundle: Bundle = serde_json::from_str(text)
            .map_err(|e| Diagnostic::new("reopen", Location::asset(""), e))?;
        let current_library = source_library_version();
        let definitions_match =
            bundle.source_library.rsplit('-').next() == current_library.rsplit('-').next();
        if bundle.version != 1 || !definitions_match {
            return Err(Diagnostic::new(
                "reopen",
                Location::asset(&bundle.root.id),
                "Unsupported bundle or source-library version",
            ));
        }
        Ok(Self {
            root: EditableDocument::parse(&bundle.root.id, &bundle.root.recipe)?,
            selected_material: bundle.selected_material,
            dependencies: bundle
                .dependencies
                .iter()
                .map(|d| EditableDocument::parse(&d.id, &d.recipe))
                .collect::<Result<_>>()?,
            instances: bundle.instances,
        })
    }
    pub fn parameter_metadata(&self) -> Result<Vec<ParameterInfo>> {
        let mut parameters = vec![];
        for d in self.documents() {
            for m in &d.project.materials {
                // Material-level color defaults have persistent keys too. Hosts may
                // bind them to a palette without manufacturing hidden Color rows.
                for (name, fallback) in
                    [("base color", vec![1., 1., 1.]), ("emission", vec![0.; 3])]
                {
                    let mut value = m
                        .defaults
                        .iter()
                        .find(|(id, _)| id == name)
                        .map_or(fallback, |(_, v)| v.clone());
                    value.resize(4, 1.);
                    parameters.push(ParameterInfo {
                        key: ParameterKey::new(&d.id, &format!("@material:{}", m.id), name),
                        location: Location {
                            asset: d.id.clone(),
                            material: Some(m.id.clone()),
                            row: None,
                            tile: None,
                            parameter: Some(name.into()),
                        },
                        tile_index: usize::MAX,
                        source_id: "@material".into(),
                        definition: crate::metadata::Parameter {
                            id: name.into(),
                            label: name.into(),
                            group: "Material".into(),
                            kind: ValueType::Color,
                            default: value.clone(),
                            range: None,
                            tooltip: None,
                            choices: vec![],
                        },
                        binding: format::Binding::Constant(value),
                    });
                }
                for r in &m.rows {
                    for (tile_index, t) in r.tiles.iter().enumerate() {
                        let location = Location {
                            asset: d.id.clone(),
                            material: Some(m.id.clone()),
                            row: Some(r.id.clone()),
                            tile: Some(t.id.clone()),
                            parameter: None,
                        };
                        let kernel = Kernel::find(&t.kernel).ok_or_else(|| {
                            Diagnostic::new(
                                "metadata",
                                location.clone(),
                                format!("Unknown source {:?}", t.kernel),
                            )
                        })?;
                        for definition in kernel.metadata().parameters {
                            let binding = t
                                .parameters
                                .iter()
                                .find(|(id, _)| id == &definition.id)
                                .map(|(_, b)| b.clone())
                                .unwrap_or_else(|| {
                                    format::Binding::Constant(definition.default.clone())
                                });
                            let mut location = location.clone();
                            location.parameter = Some(definition.id.clone());
                            parameters.push(ParameterInfo {
                                key: ParameterKey::new(&d.id, &t.id, &definition.id),
                                location,
                                tile_index,
                                source_id: kernel.source_id().into(),
                                definition,
                                binding,
                            });
                        }
                    }
                }
            }
        }
        Ok(parameters)
    }
    pub fn locate(&self, key: &ParameterKey) -> Result<Location> {
        self.parameter_metadata()?
            .into_iter()
            .find(|p| p.key == *key)
            .map(|p| p.location)
            .ok_or_else(|| {
                Diagnostic::new(
                    "resolve",
                    Location {
                        asset: key.asset.clone(),
                        material: None,
                        row: None,
                        tile: Some(key.tile.clone()),
                        parameter: Some(key.parameter.clone()),
                    },
                    "Unresolved persistent parameter",
                )
            })
    }
    pub fn error(&self, operation: &str, message: impl ToString) -> Diagnostic {
        let mut location = Location::asset(&self.root.id);
        location.material = Some(self.selected_material.clone());
        Diagnostic::new(operation, location, message)
    }
    /// Returns the resolved document with validated overrides; never edits the asset.
    pub fn resolved(&self, instance: &InstanceRecord) -> Result<format::Project> {
        let mut ids = HashSet::new();
        let mut project = format::Project {
            result: Some(self.selected_material.clone()),
            materials: vec![],
        };
        for d in self.documents() {
            if d.id.is_empty() || !ids.insert(&d.id) {
                return Err(self.error("validate", "Empty or duplicate asset identity"));
            }
            format::validate_tile_ids(&d.project)
                .map_err(|e| Diagnostic::new("validate", Location::asset(&d.id), e))?;
            for m in &d.project.materials {
                let mut material = m.clone();
                for t in material.rows.iter_mut().flat_map(|r| &mut r.tiles) {
                    t.id = scoped_tile(&d.id, &t.id);
                }
                project.materials.push(material);
            }
        }
        if instance.id.is_empty() || instance.uv_transform.iter().any(|v| !v.is_finite()) {
            return Err(self.error("validate", "Instance needs an ID and finite UV transform"));
        }
        let mut palette_keys = HashSet::new();
        for binding in &instance.palette_bindings {
            let location = self.locate(&binding.key)?;
            let info = self
                .parameter_metadata()?
                .into_iter()
                .find(|p| p.key == binding.key)
                .unwrap();
            if !palette_keys.insert(&binding.key)
                || info.definition.kind != ValueType::Color
                || matches!(info.binding, format::Binding::From(_))
            {
                return Err(Diagnostic::new(
                    "validate",
                    location,
                    "Palette bindings require distinct, editable color constants",
                ));
            }
        }
        let mut keys = HashSet::new();
        for value in &instance.overrides {
            let location = self.locate(&value.key)?;
            if !keys.insert(&value.key) {
                return Err(Diagnostic::new("update", location, "Duplicate override"));
            }
            let info = self
                .parameter_metadata()?
                .into_iter()
                .find(|p| p.key == value.key)
                .unwrap();
            if matches!(info.binding, format::Binding::From(_)) {
                return Err(Diagnostic::new(
                    "update",
                    location,
                    "Parameter is connected; edit its source explicitly",
                ));
            }
            if value.value.len() != info.definition.kind.width()
                || value.value.iter().any(|v| !v.is_finite())
                || (info.definition.kind == ValueType::Bool && !matches!(value.value[0], 0. | 1.))
            {
                return Err(Diagnostic::new(
                    "update",
                    location,
                    "Parameter value has the wrong type or nonfinite components",
                ));
            }
            if let Some(id) = value.key.tile.strip_prefix("@material:") {
                let m = project
                    .materials
                    .iter_mut()
                    .find(|m| m.id == id)
                    .ok_or_else(|| self.error("resolve", "Missing material"))?;
                let rgb = value.value[..3].to_vec();
                if let Some((_, old)) = m
                    .defaults
                    .iter_mut()
                    .find(|(name, _)| name == &value.key.parameter)
                {
                    *old = rgb;
                } else {
                    m.defaults.push((value.key.parameter.clone(), rgb));
                }
                continue;
            }
            let t = project
                .materials
                .iter_mut()
                .flat_map(|m| &mut m.rows)
                .flat_map(|r| &mut r.tiles)
                .find(|t| t.id == scoped_tile(&value.key.asset, &value.key.tile))
                .ok_or_else(|| self.error("resolve", "Missing tile"))?;
            if let Some((_, b)) = t
                .parameters
                .iter_mut()
                .find(|(id, _)| id == &value.key.parameter)
            {
                *b = format::Binding::Constant(value.value.clone());
            } else {
                t.parameters.push((
                    value.key.parameter.clone(),
                    format::Binding::Constant(value.value.clone()),
                ));
            }
        }
        Ok(project)
    }
    pub fn program(&self, instance: &InstanceRecord) -> Result<Program> {
        let project = self.resolved(instance)?;
        for m in &project.materials {
            for r in &m.rows {
                for (tile_index, t) in r.tiles.iter().enumerate() {
                    let asset = self
                        .documents()
                        .find(|d| d.project.materials.iter().any(|v| v.id == m.id))
                        .map(|d| d.id.as_str())
                        .unwrap_or(&self.root.id);
                    let location = Location {
                        asset: asset.into(),
                        material: Some(m.id.clone()),
                        row: Some(r.id.clone()),
                        tile: self
                            .documents()
                            .find(|d| d.id == asset)
                            .and_then(|d| d.project.materials.iter().find(|v| v.id == m.id))
                            .and_then(|m| m.rows.iter().find(|v| v.id == r.id))
                            .and_then(|r| r.tiles.get(tile_index))
                            .map(|t| t.id.clone()),
                        parameter: None,
                    };
                    let k = Kernel::find(&t.kernel).ok_or_else(|| {
                        Diagnostic::new(
                            "validate",
                            location.clone(),
                            format!("Unknown source {:?}", t.kernel),
                        )
                    })?;
                    let metadata = k.metadata();
                    let code = t
                        .code
                        .as_deref()
                        .unwrap_or_else(|| crate::tilelang::source(k));
                    let declaration = crate::tilelang::definition(code)
                        .map_err(|e| Diagnostic::new("validate", location.clone(), e))?;
                    if declaration != metadata {
                        return Err(Diagnostic::new(
                            "validate",
                            location,
                            "Custom source declarations do not match the source definition",
                        ));
                    }
                    crate::tilelang::Algorithm::compile(code, &metadata, k)
                        .map_err(|e| Diagnostic::new("validate", location.clone(), e))?;
                }
            }
        }
        Program::lower(&project).map_err(|e| self.error("validate", e))
    }
    pub fn validate(&self) -> Result<()> {
        self.program(&InstanceRecord::new("validation"))?;
        let mut ids = HashSet::new();
        for instance in &self.instances {
            if !ids.insert(&instance.id) {
                return Err(self.error("validate", "Duplicate instance identity"));
            }
            for binding in &instance.palette_bindings {
                self.locate(&binding.key)?;
            }
            self.program(instance)?;
        }
        Ok(())
    }
}
