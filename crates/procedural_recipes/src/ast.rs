use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecipeDocument {
    Tile(Recipe),
    Materials(MaterialDocument),
    Sdfs(SdfDocument),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SdfDocument {
    pub recipes: Vec<SdfRecipe>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SdfRecipe {
    pub id: String,
    pub name: String,
    pub shapes: Vec<SdfShape>,
    pub output: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SdfShape {
    pub name: String,
    pub kind: SdfShapeKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SdfShapeKind {
    Ellipse {
        position: [f32; 2],
        size: [f32; 2],
        rotation: f32,
    },
    RoundedRectangle {
        position: [f32; 2],
        size: [f32; 2],
        radius: f32,
        rotation: f32,
    },
    Capsule {
        from: [f32; 2],
        to: [f32; 2],
        radius: f32,
    },
    Union {
        a: String,
        b: String,
    },
    Subtract {
        a: String,
        b: String,
    },
    Intersect {
        a: String,
        b: String,
    },
    Expand {
        source: String,
        amount: f32,
    },
    Contract {
        source: String,
        amount: f32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub blocking: bool,
    pub material: Option<String>,
    pub material_map: Option<MaterialMap>,
    pub size: [u32; 2],
    pub coverage: [u32; 2],
    pub wrap: WrapMode,
    pub seed: u64,
    pub pixelate: u32,
    pub animation: Animation,
    pub fields: Vec<FieldDefinition>,
    pub patterns: Vec<PatternDefinition>,
    pub geometry: Vec<GeometryFeature>,
    pub colorize: Option<Colorize>,
    pub output: Output,
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            name: "Untitled Tile".to_string(),
            blocking: false,
            material: None,
            material_map: None,
            size: [64, 64],
            coverage: [1, 1],
            wrap: WrapMode::Repeat,
            seed: 1,
            pixelate: 1,
            animation: Animation::default(),
            fields: Vec::new(),
            patterns: Vec::new(),
            geometry: Vec::new(),
            colorize: None,
            output: Output::default(),
        }
    }
}

/// Placement-local geometry authored by a Tile recipe.
///
/// A caller supplies a placement basis (for example a wall face or ceiling cell). The same
/// primitive/operation program can therefore be evaluated by Source, Creator, or another host
/// without adding domain-specific feature types to the recipe language.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GeometryFeature {
    Box(BoxGeometry),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeometryOperation {
    #[default]
    Add,
    Subtract,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoxGeometry {
    pub name: String,
    pub operation: GeometryOperation,
    /// Tile recipe applied to the emitted solid or newly exposed subtraction faces.
    pub surface: String,
    /// Placement-local minimum in world units.
    pub position: [f32; 3],
    /// Placement-local dimensions in world units.
    pub size: [f32; 3],
    /// Number of instances along each placement-local axis.
    pub repeat: [u32; 3],
    /// Translation between repeated instances.
    pub spacing: [f32; 3],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WrapMode {
    Clamp,
    #[default]
    Repeat,
    Mirror,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Animation {
    pub frames: u32,
    pub fps: f32,
    pub looping: bool,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            frames: 1,
            fps: 12.0,
            looping: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FieldDefinition {
    Noise(NoiseField),
    Height(HeightField),
    Value(ValueField),
}

impl FieldDefinition {
    pub fn name(&self) -> &str {
        match self {
            Self::Noise(field) => &field.name,
            Self::Height(field) => &field.name,
            Self::Value(field) => &field.name,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoiseField {
    pub name: String,
    pub domain: Domain,
    pub key: Option<IdSource>,
    pub kind: NoiseKind,
    pub fractal: FractalKind,
    pub scale: [f32; 2],
    pub octaves: u32,
    pub persistence: f32,
    pub seed: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValueField {
    pub name: String,
    pub source: ScalarSource,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoiseKind {
    #[default]
    Value,
    Gradient,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FractalKind {
    #[default]
    Fbm,
    Ridged,
    Billow,
    Turbulence,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeightField {
    pub name: String,
    pub source: ScalarSource,
    pub operations: Vec<HeightOperation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HeightOperation {
    Shape {
        contrast: ScalarSource,
        bias: ScalarSource,
        plateau: ScalarSource,
        rim: ScalarSource,
    },
    Combine {
        mode: CombineMode,
        source: ScalarSource,
        amount: ScalarSource,
    },
    Clamp {
        min: ScalarSource,
        max: ScalarSource,
    },
    Remap {
        from: [f32; 2],
        to: [f32; 2],
    },
    Terrace {
        steps: u32,
        smoothness: f32,
    },
    Invert,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombineMode {
    Add,
    Subtract,
    Multiply,
    Min,
    Max,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PatternDefinition {
    pub name: String,
    pub domain: Domain,
    pub key: Option<IdSource>,
    pub warp: Option<Warp>,
    pub bevel: ScalarSource,
    pub perturb: Option<Perturb>,
    pub kind: PatternKind,
    #[serde(skip)]
    pub warnings: Vec<ParseWarning>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Domain {
    Global,
    PatternLocal(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Warp {
    pub source: ScalarSource,
    pub amount: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Perturb {
    pub source: ScalarSource,
    pub amount: ScalarSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseWarning {
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub source_line: Option<String>,
    pub source_name: Option<String>,
}

impl ParseWarning {
    pub const fn stable_code(&self) -> &'static str {
        "PRW0001"
    }

    pub fn with_source_name(mut self, source_name: impl Into<String>) -> Self {
        self.source_name = Some(source_name.into());
        self
    }
}

impl std::fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "warning[{}]: {}", self.stable_code(), self.message)?;
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PatternKind {
    Bricks {
        columns: u32,
        rows: u32,
        stagger: f32,
        gap: ScalarSource,
        rounding: ScalarSource,
        rotation: ScalarSource,
        size_variation: [f32; 2],
        falloff: ScalarSource,
        seed: u64,
    },
    Voronoi {
        cells: [u32; 2],
        jitter: f32,
        falloff: f32,
        seed: u64,
    },
    Discs {
        cells: [u32; 2],
        jitter: ScalarSource,
        radius: ScalarSource,
        falloff: ScalarSource,
        seed: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternChannel {
    Height,
    Edge,
    Center,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeometryChannel {
    /// Signed wall-local distance to a feature boundary, in world units.
    Distance,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum IdSource {
    Current,
    Pattern(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScalarSource {
    Constant(f32),
    Coordinate(CoordinateChannel),
    InputHeight,
    Field(String),
    Pattern {
        name: String,
        channel: PatternChannel,
    },
    Geometry {
        name: String,
        channel: GeometryChannel,
    },
    RandomId {
        id: IdSource,
        min: f32,
        max: f32,
        seed: u64,
    },
    Wave {
        min: f32,
        max: f32,
        cycles: f32,
        phase: f32,
    },
    Unary {
        op: UnaryOperator,
        source: Box<ScalarSource>,
    },
    Binary {
        op: BinaryOperator,
        left: Box<ScalarSource>,
        right: Box<ScalarSource>,
    },
    Clamp {
        source: Box<ScalarSource>,
        min: Box<ScalarSource>,
        max: Box<ScalarSource>,
    },
    Mix {
        a: Box<ScalarSource>,
        b: Box<ScalarSource>,
        factor: Box<ScalarSource>,
    },
    Smoothstep {
        min: Box<ScalarSource>,
        max: Box<ScalarSource>,
        source: Box<ScalarSource>,
    },
}

impl ScalarSource {
    pub fn constant(value: f32) -> Self {
        Self::Constant(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Negate,
    Abs,
    Invert,
    Sin,
    Cos,
    Fract,
    Sqrt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoordinateChannel {
    U,
    V,
    Radius,
    Angle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Min,
    Max,
    Pow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Colorize {
    pub source: ScalarSource,
    /// Controls whether every ramp step is palette-mapped or only the base anchor is mapped.
    pub palette: PaletteMode,
    /// Optional author-selected color around which a palette-constrained ramp is built.
    pub base: Option<[u8; 4]>,
    /// HSL lightness offsets applied from the darkest to the brightest ramp step.
    pub brightness: [f32; 2],
    /// HSL saturation offsets applied from the darkest to the brightest ramp step.
    pub saturation: [f32; 2],
    /// HSL hue offsets applied from the darkest to the brightest ramp step.
    pub hue: [f32; 2],
    pub family: ColorFamily,
    pub ramp_range: [f32; 2],
    pub saturation_range: [f32; 2],
    pub steps: u32,
    pub range: ColorRange,
    pub dither: bool,
}

impl Default for Colorize {
    fn default() -> Self {
        Self {
            source: ScalarSource::Constant(0.0),
            palette: PaletteMode::Strict,
            base: None,
            brightness: [-0.22, 0.22],
            saturation: [-0.08, 0.08],
            hue: [0.0, 0.0],
            family: ColorFamily::Any,
            ramp_range: [0.0, 1.0],
            saturation_range: [0.0, 1.0],
            steps: 4,
            range: ColorRange::Auto,
            dither: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteMode {
    #[default]
    Strict,
    BaseOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ColorRange {
    Auto,
    Fixed([f32; 2]),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Output {
    pub height: ScalarSource,
    pub space: Domain,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            height: ScalarSource::Constant(0.0),
            space: Domain::Global,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialDocument {
    pub materials: Vec<MaterialRecipe>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialRecipe {
    pub id: String,
    pub name: String,
    pub wrap: WrapMode,
    pub seed: u64,
    pub fields: Vec<FieldDefinition>,
    pub patterns: Vec<PatternDefinition>,
    pub colors: Vec<ColorDefinition>,
    pub surface: MaterialSurface,
    pub output: Option<MaterialOutput>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorDefinition {
    pub name: String,
    pub source: ColorSource,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ColorSource {
    Exact([u8; 4]),
    Nearest([u8; 4]),
    Reference(String),
    Mix {
        a: Box<ColorSource>,
        b: Box<ColorSource>,
        factor: ScalarSource,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialSurface {
    pub color: ColorSource,
    pub palette: PaletteMode,
    pub roughness: ScalarSource,
    pub metallic: ScalarSource,
    pub opacity: ScalarSource,
    pub emissive: ScalarSource,
    pub normal: Option<ScalarSource>,
    pub normal_strength: f32,
}

impl Default for MaterialSurface {
    fn default() -> Self {
        Self {
            color: ColorSource::Exact([128, 128, 128, 255]),
            palette: PaletteMode::BaseOnly,
            roughness: ScalarSource::Constant(1.0),
            metallic: ScalarSource::Constant(0.0),
            opacity: ScalarSource::Constant(1.0),
            emissive: ScalarSource::Constant(0.0),
            normal: None,
            normal_strength: 0.35,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MaterialOutput {
    Value { source: ScalarSource, space: Domain },
    Color { source: ColorSource, space: Domain },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialMap {
    pub base: String,
    pub space: Domain,
    pub tiling: [f32; 2],
    pub layers: Vec<MaterialLayer>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialLayer {
    pub material: String,
    pub mask: ScalarSource,
    pub space: Domain,
    pub tiling: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorFamily {
    #[default]
    Any,
    Neutral,
    Warm,
    Cool,
    Earth,
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Purple,
    Magenta,
}
