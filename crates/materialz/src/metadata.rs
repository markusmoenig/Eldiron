//! MaterialZ tile declarations provide controls; historical declarations remain for provenance.
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Float,
    Vec2,
    Vec3,
    Color,
    Bool,
}
impl ValueType {
    pub fn name(self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Color => "color",
            Self::Bool => "bool",
        }
    }
    pub fn width(self) -> usize {
        match self {
            Self::Float | Self::Bool => 1,
            Self::Vec2 => 2,
            Self::Vec3 => 3,
            Self::Color => 4,
        }
    }
}

/// Versioned, structured source catalog for Swift's JSONDecoder and other clients.
/// All control definitions come from executable MaterialZ tile declarations.
pub fn catalog_json() -> String {
    use crate::json::{numbers, string as j};
    let sources = Kernel::ALL.iter().map(|k| {
        let m = k.metadata();
        let groups = m.groups.iter().map(|g| format!("{{\"label\":{},\"expanded\":{}}}",j(&g.label),g.expanded)).collect::<Vec<_>>().join(",");
        let parameters = m.parameters.iter().map(|p| {
            let range = p.range.map(|(a,b)| numbers(&[a,b])).unwrap_or_else(|| "null".into());
            let choices=p.choices.iter().map(|v|j(v)).collect::<Vec<_>>().join(",");
            let tooltip = p.tooltip.as_deref().map(j).unwrap_or_else(|| "null".into());
            format!("{{\"id\":{},\"label\":{},\"group\":{},\"type\":{},\"default\":{},\"range\":{},\"choices\":[{choices}],\"tooltip\":{}}}",j(&p.id),j(&p.label),j(&p.group),j(p.kind.name()),numbers(&p.default),range,tooltip)
        }).collect::<Vec<_>>().join(",");
        let fields=m.output_fields.iter().map(|f|format!("{{\"id\":{},\"label\":{},\"components\":{}}}",j(&f.id),j(&f.label),j(&f.components))).collect::<Vec<_>>().join(",");
        let outputs=m.output_channels.iter().map(|o|format!("{{\"component\":{},\"label\":{}}}",j(&o.component),j(&o.label))).collect::<Vec<_>>().join(",");
        format!("{{\"source_id\":{},\"help_key\":{},\"name\":{},\"category\":{},\"description\":{},\"references\":{},\"requires_input\":{},\"accepts_input\":{},\"creates_distance\":{},\"shape_combinations\":{},\"output_fields\":[{fields}],\"output_channels\":[{}],\"default_channel\":{},\"groups\":[{}],\"parameters\":[{}]}}",j(k.source_id()),j(&k.help_key()),j(&m.name),j(&m.category),j(&m.description),j(&m.references),k.requires_input(),k.accepts_input(),k.is_shape(), if k.is_shape() { "[\"replace\",\"union\",\"subtract\",\"intersect\",\"smooth union\"]" } else { "[]" },outputs,m.default_channel.as_deref().map(j).unwrap_or_else(|| "null".into()),groups,parameters)
    }).collect::<Vec<_>>().join(",");
    format!("{{\"version\":1,\"sources\":[{sources}]}}")
}
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub id: String,
    pub label: String,
    pub group: String,
    pub kind: ValueType,
    pub default: Vec<f32>,
    pub range: Option<(f32, f32)>,
    pub tooltip: Option<String>,
    pub choices: Vec<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub label: String,
    pub expanded: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub struct OutputChannel {
    pub component: String,
    pub label: String,
}
#[derive(Debug, Clone, PartialEq)]
pub struct OutputField {
    pub id: String,
    pub label: String,
    pub mode: f32,
    pub components: String,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    pub name: String,
    pub category: String,
    pub groups: Vec<Group>,
    pub parameters: Vec<Parameter>,
    pub description: String,
    pub references: String,
    pub output_channels: Vec<OutputChannel>,
    pub output_fields: Vec<OutputField>,
    pub default_channel: Option<String>,
    pub clears_distance: bool,
    /// Optional image-pass halo controls (radius, units); execution planning stays below rows.
    pub image_execution: Option<(String, String)>,
}

fn quoted(s: &str) -> Result<Vec<String>> {
    let mut values = Vec::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\'' && c != '"' {
            continue;
        }
        let mut value = String::new();
        let mut closed = false;
        while let Some(next) = chars.next() {
            if next == c {
                closed = true;
                break;
            }
            if next == '\\' {
                value.push(
                    chars
                        .next()
                        .ok_or_else(|| Error("Unfinished escape in metadata".into()))?,
                );
            } else {
                value.push(next);
            }
        }
        if !closed {
            return Err(Error("Unterminated metadata string".into()));
        }
        values.push(value);
    }
    Ok(values)
}
fn number(s: &str) -> Result<f32> {
    s.parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| Error(format!("Invalid metadata number {s:?}")))
}
pub fn parse(source: &str) -> Result<Metadata> {
    let mut m = Metadata {
        name: String::new(),
        category: String::new(),
        groups: vec![],
        parameters: vec![],
        description: String::new(),
        references: String::new(),
        output_channels: vec![],
        output_fields: vec![],
        default_channel: None,
        clears_distance: false,
        image_execution: None,
    };
    let mut group = String::new();
    for line in source.lines().map(str::trim) {
        let Some((key, rest)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if !matches!(
            key,
            "outName"
                | "outCategory"
                | "ParameterGroup"
                | "ParamSlider"
                | "ParamVector2"
                | "ParamVector3"
                | "ParamColor"
                | "ParamBoolean"
                | "ParamToolTip"
                | "ParamDescription"
                | "ParamReferences"
        ) {
            if key.starts_with("Param") {
                return Err(Error(format!("Unsupported metadata declaration {key}")));
            }
            continue;
        }
        let values = quoted(rest)?;
        let get = |i: usize| {
            values
                .get(i)
                .map(String::as_str)
                .ok_or_else(|| Error(format!("Missing field {i} in {key}")))
        };
        match key {
            "outName" => m.name = get(0)?.into(),
            "outCategory" => m.category = get(0)?.into(),
            "ParamDescription" => m.description = get(0)?.into(),
            "ParamReferences" => m.references = get(0)?.into(),
            "ParameterGroup" => {
                group = get(0)?.into();
                m.groups.push(Group {
                    label: group.clone(),
                    expanded: values.get(1).is_none_or(|v| v != "false"),
                });
            }
            "ParamToolTip" => {
                m.parameters
                    .last_mut()
                    .ok_or_else(|| Error("Tooltip without parameter".into()))?
                    .tooltip = Some(get(0)?.into())
            }
            _ => {
                let kind = match key {
                    "ParamSlider" => ValueType::Float,
                    "ParamVector2" => ValueType::Vec2,
                    "ParamVector3" => ValueType::Vec3,
                    "ParamColor" => ValueType::Color,
                    _ => ValueType::Bool,
                };
                let width = kind.width();
                let default = if kind == ValueType::Color {
                    let raw = get(2)?;
                    let inside = raw
                        .split_once('(')
                        .and_then(|(_, s)| s.split_once(')'))
                        .ok_or_else(|| Error("Invalid color default".into()))?
                        .0;
                    let result = inside
                        .split(',')
                        .map(|v| number(v.trim()))
                        .collect::<Result<Vec<_>>>()?;
                    if result.len() != 4 {
                        return Err(Error("Color needs four components".into()));
                    }
                    result
                } else {
                    (0..width)
                        .map(|i| number(get(i + 2)?))
                        .collect::<Result<Vec<_>>>()?
                };
                let range = if matches!(kind, ValueType::Color | ValueType::Bool) {
                    None
                } else {
                    Some((number(get(2 + width)?)?, number(get(3 + width)?)?))
                };
                m.parameters.push(Parameter {
                    id: get(0)?.into(),
                    label: get(1)?.into(),
                    group: group.clone(),
                    kind,
                    default,
                    range,
                    tooltip: None,
                    choices: vec![],
                });
            }
        }
    }
    if m.name.is_empty() {
        return Err(Error("Shader has no outName".into()));
    }
    Ok(m)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kernel {
    Perlin,
    Disc,
    Multiply,
    ColorBlend,
    Transform,
    Circle,
    RoundedRectangle,
    Color,
    Border,
    PeriodicProfile,
    CompareSelect,
    CellData,
    DirectionalGrain,
    HeightToNormal,
    Library(u16),
}
impl Kernel {
    /// Stable identity for help and other presentation resources. Independent of
    /// display names and the generated library index; never serialize that index.
    pub fn source_id(self) -> &'static str {
        match self {
            Self::Perlin => "perlin",
            Self::Disc => "disc",
            Self::Multiply => "multiply",
            Self::ColorBlend => "color_blend",
            Self::Transform => "transform",
            Self::Circle => "circle",
            Self::RoundedRectangle => "rounded_rectangle",
            Self::Color => "color",
            Self::Border => "border",
            Self::PeriodicProfile => "periodic_profile",
            Self::CompareSelect => "compare_select",
            Self::CellData => "cell_data",
            Self::DirectionalGrain => "directional_grain",
            Self::HeightToNormal => "height_to_normal",
            Self::Library(i) => crate::library::TILES[i as usize].3,
        }
    }
    pub fn help_key(self) -> String {
        format!("tile.{}", self.source_id())
    }
    pub const ALL: [Self; 86] = [
        Self::Perlin,
        Self::Disc,
        Self::Multiply,
        Self::ColorBlend,
        Self::Transform,
        Self::Circle,
        Self::RoundedRectangle,
        Self::Color,
        Self::Border,
        Self::PeriodicProfile,
        Self::CompareSelect,
        Self::CellData,
        Self::DirectionalGrain,
        Self::HeightToNormal,
        Self::Library(0),
        Self::Library(1),
        Self::Library(2),
        Self::Library(3),
        Self::Library(4),
        Self::Library(5),
        Self::Library(6),
        Self::Library(7),
        Self::Library(8),
        Self::Library(9),
        Self::Library(10),
        Self::Library(11),
        Self::Library(12),
        Self::Library(13),
        Self::Library(14),
        Self::Library(15),
        Self::Library(16),
        Self::Library(17),
        Self::Library(18),
        Self::Library(19),
        Self::Library(20),
        Self::Library(21),
        Self::Library(22),
        Self::Library(23),
        Self::Library(24),
        Self::Library(25),
        Self::Library(26),
        Self::Library(27),
        Self::Library(28),
        Self::Library(29),
        Self::Library(30),
        Self::Library(31),
        Self::Library(32),
        Self::Library(33),
        Self::Library(34),
        Self::Library(35),
        Self::Library(36),
        Self::Library(37),
        Self::Library(38),
        Self::Library(39),
        Self::Library(40),
        Self::Library(41),
        Self::Library(42),
        Self::Library(43),
        Self::Library(44),
        Self::Library(45),
        Self::Library(46),
        Self::Library(47),
        Self::Library(48),
        Self::Library(49),
        Self::Library(50),
        Self::Library(51),
        Self::Library(52),
        Self::Library(53),
        Self::Library(54),
        Self::Library(55),
        Self::Library(56),
        Self::Library(57),
        Self::Library(58),
        Self::Library(59),
        Self::Library(60),
        Self::Library(61),
        Self::Library(62),
        Self::Library(63),
        Self::Library(64),
        Self::Library(65),
        Self::Library(66),
        Self::Library(67),
        Self::Library(68),
        Self::Library(69),
        Self::Library(70),
        Self::Library(71),
    ];
    pub fn original(self) -> &'static str {
        match self {
            Self::Library(i) => crate::library::TILES[i as usize].1,
            Self::Perlin => include_str!("../shaders/original/noise_briansperlin2d.fs"),
            Self::Disc => include_str!("../shaders/original/shape2d_disc.fs"),
            Self::Multiply => include_str!("../shaders/original/op_multiply.fs"),
            Self::ColorBlend => include_str!("../shaders/original/op_colorblend.fs"),
            Self::Transform => include_str!("../shaders/original/trans_scale_offset.fs"),
            Self::Circle => include_str!("../shaders/native/circle.source"),
            Self::RoundedRectangle => include_str!("../shaders/native/rounded_rectangle.source"),
            Self::Color => include_str!("../shaders/native/color.source"),
            Self::Border => include_str!("../shaders/native/border.source"),
            // New native sources use Tile Language directly as their provenance.
            Self::PeriodicProfile
            | Self::CompareSelect
            | Self::CellData
            | Self::DirectionalGrain
            | Self::HeightToNormal => crate::tilelang::source(self),
        }
    }
    pub fn metadata(self) -> Metadata {
        static CATALOG: std::sync::OnceLock<Vec<Metadata>> = std::sync::OnceLock::new();
        let metadata = CATALOG.get_or_init(|| {
            Self::ALL
                .iter()
                .map(|k| {
                    crate::tilelang::definition(crate::tilelang::source(*k))
                        .expect("Bundled tile declarations are validated by tests")
                })
                .collect()
        });
        let index = match self {
            Self::Library(i) => 14 + i as usize,
            _ => Self::ALL.iter().position(|k| *k == self).unwrap(),
        };
        metadata[index].clone()
    }
    pub fn find(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.metadata().name == name)
    }
    pub fn is_shape(self) -> bool {
        matches!(self, Self::Circle | Self::RoundedRectangle)
            || self.source_id() == "shape2d_polygon"
    }
    pub fn accepts_input(self) -> bool {
        self.requires_input()
            || self.is_shape()
            || matches!(
                self,
                Self::Perlin
                    | Self::Color
                    | Self::ColorBlend
                    | Self::CellData
                    | Self::DirectionalGrain
                    | Self::Library(_)
            )
    }
    pub fn requires_input(self) -> bool {
        if let Self::Library(i) = self {
            return crate::library::TILES[i as usize].2;
        }
        matches!(
            self,
            Self::Multiply
                | Self::Transform
                | Self::Border
                | Self::PeriodicProfile
                | Self::CompareSelect
                | Self::HeightToNormal
        )
    }
}
