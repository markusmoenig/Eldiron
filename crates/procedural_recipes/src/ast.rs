use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecipeDocument {
    Tile(Recipe),
    Materials(MaterialDocument),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub material: Option<String>,
    pub size: [u32; 2],
    pub coverage: [u32; 2],
    pub wrap: WrapMode,
    pub seed: u64,
    pub pixelate: u32,
    pub animation: Animation,
    pub fields: Vec<FieldDefinition>,
    pub patterns: Vec<PatternDefinition>,
    pub colorize: Colorize,
    pub output: Output,
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            name: "Untitled Tile".to_string(),
            material: None,
            size: [64, 64],
            coverage: [1, 1],
            wrap: WrapMode::Repeat,
            seed: 1,
            pixelate: 1,
            animation: Animation::default(),
            fields: Vec::new(),
            patterns: Vec::new(),
            colorize: Colorize::default(),
            output: Output::default(),
        }
    }
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
}

impl FieldDefinition {
    pub fn name(&self) -> &str {
        match self {
            Self::Noise(field) => &field.name,
            Self::Height(field) => &field.name,
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
    pub kind: PatternKind,
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
pub enum PatternKind {
    Bricks {
        columns: u32,
        rows: u32,
        stagger: f32,
        gap: ScalarSource,
        rounding: ScalarSource,
        rotation: ScalarSource,
        size_variation: [f32; 2],
        perturb: Option<ScalarSource>,
        perturb_amount: ScalarSource,
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
}

impl Default for Output {
    fn default() -> Self {
        Self {
            height: ScalarSource::Constant(0.0),
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
    pub colorize: Colorize,
    pub data: MaterialData,
    pub normal: MaterialNormal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialData {
    pub roughness: ScalarSource,
    pub metallic: ScalarSource,
    pub opacity: ScalarSource,
    pub emissive: ScalarSource,
}

impl Default for MaterialData {
    fn default() -> Self {
        Self {
            roughness: ScalarSource::Constant(0.5),
            metallic: ScalarSource::Constant(0.0),
            opacity: ScalarSource::Constant(1.0),
            emissive: ScalarSource::Constant(0.0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialNormal {
    pub source: ScalarSource,
    pub strength: f32,
}

impl Default for MaterialNormal {
    fn default() -> Self {
        Self {
            source: ScalarSource::InputHeight,
            strength: 0.35,
        }
    }
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
