#[derive(Clone, Copy)]
pub struct IsoPaintBrushSample<'a> {
    pub brush: &'a str,
    pub shape: &'a str,
    pub color: [u8; 4],
    pub palette: &'a [[u8; 4]],
    pub opacity: f32,
    pub radius: i32,
    pub seed: u32,
}

#[derive(Clone, Copy)]
enum BrushLayerKind {
    Fill,
    Grain,
    Fleck,
    Blade,
    Crack,
}

impl BrushLayerKind {
    fn label(self) -> &'static str {
        match self {
            BrushLayerKind::Fill => "Fill",
            BrushLayerKind::Grain => "Grain",
            BrushLayerKind::Fleck => "Flecks",
            BrushLayerKind::Blade => "Blades",
            BrushLayerKind::Crack => "Crack Lines",
        }
    }
}

#[derive(Clone, Copy)]
struct BrushLayer {
    kind: BrushLayerKind,
    shape: &'static str,
    opacity: f32,
    density: f32,
    scale: i32,
    seed: u32,
    use_input_color: bool,
    colors: &'static [[u8; 4]],
}

#[derive(Clone, Copy)]
pub struct BrushPreset {
    pub key: &'static str,
    pub default_shape: &'static str,
    pub default_color: [u8; 4],
    layers: &'static [BrushLayer],
}

#[derive(Clone, Copy)]
pub struct BrushLayerInfo {
    pub label: &'static str,
    pub shape: &'static str,
    pub opacity: f32,
    pub density: f32,
    pub scale: i32,
    pub use_input_color: bool,
    pub colors: &'static [[u8; 4]],
}

const EMPTY_COLORS: [[u8; 4]; 0] = [];
const TERRAIN_COLORS: [[u8; 4]; 5] = [
    [104, 93, 69, 255],
    [72, 88, 57, 255],
    [145, 129, 83, 255],
    [47, 55, 43, 255],
    [169, 154, 103, 255],
];
const BRICK_COLORS: [[u8; 4]; 5] = [
    [132, 115, 78, 255],
    [154, 133, 87, 255],
    [104, 91, 69, 255],
    [176, 150, 98, 255],
    [65, 59, 48, 255],
];
const MOSS_COLORS: [[u8; 4]; 5] = [
    [48, 73, 43, 255],
    [78, 108, 55, 255],
    [30, 46, 34, 255],
    [115, 115, 72, 255],
    [93, 130, 69, 255],
];
const GRASS_COLORS: [[u8; 4]; 5] = [
    [45, 77, 42, 255],
    [82, 124, 58, 255],
    [127, 119, 64, 255],
    [30, 49, 34, 255],
    [111, 151, 74, 255],
];
const DIRT_COLORS: [[u8; 4]; 5] = [
    [58, 46, 34, 255],
    [76, 58, 40, 255],
    [38, 31, 25, 255],
    [96, 74, 48, 255],
    [24, 21, 18, 255],
];
const CRACK_COLORS: [[u8; 4]; 5] = [
    [12, 13, 13, 255],
    [25, 26, 25, 255],
    [52, 54, 52, 255],
    [94, 92, 84, 255],
    [151, 145, 123, 255],
];
const PUDDLE_COLORS: [[u8; 4]; 5] = [
    [32, 56, 72, 255],
    [42, 78, 102, 255],
    [58, 110, 142, 255],
    [100, 148, 164, 255],
    [184, 222, 232, 255],
];
const MATERIAL_LAYERS: [BrushLayer; 1] = [BrushLayer {
    kind: BrushLayerKind::Fill,
    shape: "inherit",
    opacity: 1.0,
    density: 1.0,
    scale: 1,
    seed: 0x1020_3040,
    use_input_color: true,
    colors: &EMPTY_COLORS,
}];

const BRICK_LAYERS: [BrushLayer; 2] = [
    BrushLayer {
        kind: BrushLayerKind::Fill,
        shape: "solid",
        opacity: 1.0,
        density: 1.0,
        scale: 4,
        seed: 0x2060_baa1,
        use_input_color: false,
        colors: &BRICK_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Grain,
        shape: "inherit",
        opacity: 0.45,
        density: 0.45,
        scale: 2,
        seed: 0x8722_11af,
        use_input_color: false,
        colors: &BRICK_COLORS,
    },
];

const MOSS_LAYERS: [BrushLayer; 4] = [
    BrushLayer {
        kind: BrushLayerKind::Fill,
        shape: "dirt",
        opacity: 0.82,
        density: 1.0,
        scale: 5,
        seed: 0x7812_99bb,
        use_input_color: false,
        colors: &MOSS_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Grain,
        shape: "dirt",
        opacity: 0.78,
        density: 0.54,
        scale: 2,
        seed: 0xa991_2b5d,
        use_input_color: false,
        colors: &MOSS_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Fleck,
        shape: "speckle",
        opacity: 0.85,
        density: 0.28,
        scale: 1,
        seed: 0x5ce7_3301,
        use_input_color: false,
        colors: &MOSS_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Blade,
        shape: "speckle",
        opacity: 0.65,
        density: 0.36,
        scale: 1,
        seed: 0x3779_114a,
        use_input_color: false,
        colors: &MOSS_COLORS,
    },
];

const CRACK_LAYERS: [BrushLayer; 3] = [
    BrushLayer {
        kind: BrushLayerKind::Crack,
        shape: "scratch",
        opacity: 1.0,
        density: 1.0,
        scale: 1,
        seed: 0x7109_8123,
        use_input_color: false,
        colors: &CRACK_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Fleck,
        shape: "jagged",
        opacity: 0.42,
        density: 0.20,
        scale: 1,
        seed: 0x8810_4011,
        use_input_color: false,
        colors: &CRACK_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Grain,
        shape: "soft",
        opacity: 0.22,
        density: 0.30,
        scale: 3,
        seed: 0xcaae_1401,
        use_input_color: false,
        colors: &CRACK_COLORS,
    },
];

const GRASS_LAYERS: [BrushLayer; 4] = [
    BrushLayer {
        kind: BrushLayerKind::Grain,
        shape: "dirt",
        opacity: 0.42,
        density: 0.45,
        scale: 4,
        seed: 0x6642_7731,
        use_input_color: false,
        colors: &GRASS_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Blade,
        shape: "speckle",
        opacity: 0.92,
        density: 0.70,
        scale: 1,
        seed: 0x3941_7431,
        use_input_color: false,
        colors: &GRASS_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Fleck,
        shape: "speckle",
        opacity: 0.80,
        density: 0.24,
        scale: 1,
        seed: 0x88fc_200a,
        use_input_color: false,
        colors: &GRASS_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Grain,
        shape: "soft",
        opacity: 0.28,
        density: 0.30,
        scale: 6,
        seed: 0xf20c_811a,
        use_input_color: false,
        colors: &TERRAIN_COLORS,
    },
];

const DIRT_LAYERS: [BrushLayer; 3] = [
    BrushLayer {
        kind: BrushLayerKind::Fill,
        shape: "dirt",
        opacity: 0.86,
        density: 1.0,
        scale: 5,
        seed: 0x4871_aa2c,
        use_input_color: false,
        colors: &DIRT_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Grain,
        shape: "dirt",
        opacity: 0.70,
        density: 0.52,
        scale: 2,
        seed: 0x1567_4321,
        use_input_color: false,
        colors: &DIRT_COLORS,
    },
    BrushLayer {
        kind: BrushLayerKind::Fleck,
        shape: "speckle",
        opacity: 0.52,
        density: 0.20,
        scale: 1,
        seed: 0x3308_cafe,
        use_input_color: false,
        colors: &DIRT_COLORS,
    },
];

const PUDDLE_LAYERS: [BrushLayer; 1] = [BrushLayer {
    kind: BrushLayerKind::Fill,
    shape: "inherit",
    opacity: 1.0,
    density: 1.0,
    scale: 1,
    seed: 0xaacc_2871,
    use_input_color: true,
    colors: &PUDDLE_COLORS,
}];

pub fn preset_for_key(key: &str) -> BrushPreset {
    match key {
        "brick" => BrushPreset {
            key: "brick",
            default_shape: "solid",
            default_color: [143, 120, 75, 255],
            layers: &BRICK_LAYERS,
        },
        "moss" => BrushPreset {
            key: "moss",
            default_shape: "dirt",
            default_color: [78, 108, 55, 255],
            layers: &MOSS_LAYERS,
        },
        "crack" => BrushPreset {
            key: "crack",
            default_shape: "scratch",
            default_color: [42, 43, 41, 255],
            layers: &CRACK_LAYERS,
        },
        "grass" => BrushPreset {
            key: "grass",
            default_shape: "speckle",
            default_color: [75, 119, 57, 255],
            layers: &GRASS_LAYERS,
        },
        "puddle" => BrushPreset {
            key: "puddle",
            default_shape: "wash",
            default_color: [52, 94, 122, 255],
            layers: &PUDDLE_LAYERS,
        },
        "dirt" => BrushPreset {
            key: "dirt",
            default_shape: "dirt",
            default_color: [58, 46, 34, 255],
            layers: &DIRT_LAYERS,
        },
        _ => BrushPreset {
            key: "material",
            default_shape: "solid",
            default_color: [128, 110, 83, 255],
            layers: &MATERIAL_LAYERS,
        },
    }
}

pub fn default_shape_for_brush(brush: &str) -> &'static str {
    preset_for_key(brush).default_shape
}

pub fn default_preview_color(brush: &str) -> [u8; 4] {
    preset_for_key(brush).default_color
}

pub fn preset_layer_descriptions(brush: &str) -> Vec<String> {
    preset_for_key(brush)
        .layers
        .iter()
        .map(|layer| {
            format!(
                "{}  {:.0}%  {}",
                layer.kind.label(),
                (layer.opacity * 100.0).round(),
                layer.shape
            )
        })
        .collect()
}

pub fn preset_layer_infos(brush: &str) -> Vec<BrushLayerInfo> {
    preset_for_key(brush)
        .layers
        .iter()
        .map(|layer| BrushLayerInfo {
            label: layer.kind.label(),
            shape: layer.shape,
            opacity: layer.opacity,
            density: layer.density,
            scale: layer.scale,
            use_input_color: layer.use_input_color,
            colors: layer.colors,
        })
        .collect()
}

pub fn hash_u32(x: i32, y: i32, seed: u32) -> u32 {
    let mut value =
        seed ^ (x as u32).wrapping_mul(0x9e37_79b9) ^ (y as u32).wrapping_mul(0x85eb_ca6b);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

pub fn noise01(x: i32, y: i32, seed: u32) -> f32 {
    hash_u32(x, y, seed) as f32 / u32::MAX as f32
}

pub fn shape_alpha(shape: &str, ox: i32, oy: i32, radius: i32, base_alpha: u8, seed: u32) -> u8 {
    if base_alpha == 0 {
        return 0;
    }

    let radius = radius.max(1) as f32;
    let fx = ox as f32 / radius;
    let fy = oy as f32 / radius;
    let distance = (fx * fx + fy * fy).sqrt();
    if distance > 1.18 {
        return 0;
    }

    let noise = noise01(ox, oy, seed);
    let coarse = noise01(ox / 3, oy / 3, seed ^ 0xa53c_9e1d);
    let alpha = base_alpha as f32;
    let shaped = match shape {
        "inherit" => alpha,
        "soft" => {
            if distance > 1.0 {
                0.0
            } else {
                let edge = ((1.0 - distance) / 0.55).clamp(0.0, 1.0);
                alpha * edge * edge * (3.0 - 2.0 * edge)
            }
        }
        "dirt" => {
            let edge = 0.72 + coarse * 0.34;
            if distance > edge || noise < 0.18 {
                0.0
            } else {
                let grain = (0.35 + noise * 0.65).clamp(0.0, 1.0);
                alpha * grain
            }
        }
        "speckle" => {
            if distance > 1.0 || noise < 0.62 {
                0.0
            } else {
                alpha * (0.45 + coarse * 0.55)
            }
        }
        "jagged" => {
            let edge = 0.58 + coarse * 0.48;
            if distance > edge || noise < 0.12 {
                0.0
            } else {
                alpha
            }
        }
        "scratch" => {
            if distance > 1.0 || noise < 0.20 {
                0.0
            } else {
                let jitter = (coarse - 0.5) * radius * 0.28;
                let line = (oy as f32 + ox as f32 * 0.23 + jitter).abs();
                let width = (radius * 0.17).max(1.0);
                if line > width {
                    0.0
                } else {
                    alpha * (1.0 - line / (width + 0.001)).max(0.35)
                }
            }
        }
        "wash" => {
            if distance > 1.0 {
                0.0
            } else {
                let edge = (1.0 - distance).clamp(0.0, 1.0);
                let falloff = edge.sqrt();
                alpha * falloff * (0.38 + noise * 0.42)
            }
        }
        _ => {
            if distance > 1.0 {
                0.0
            } else {
                alpha
            }
        }
    };

    shaped.round().clamp(0.0, 255.0) as u8
}

fn adjust_color(color: [u8; 4], shade: f32) -> [u8; 4] {
    [
        (color[0] as f32 * shade).clamp(0.0, 255.0) as u8,
        (color[1] as f32 * shade).clamp(0.0, 255.0) as u8,
        (color[2] as f32 * shade).clamp(0.0, 255.0) as u8,
        color[3],
    ]
}

fn pick_palette(noise: f32, colors: &[[u8; 4]], fallback: [u8; 4]) -> [u8; 4] {
    if colors.is_empty() {
        return fallback;
    }
    let index = if noise < 0.52 {
        0
    } else if noise < 0.74 {
        1
    } else if noise < 0.90 {
        2
    } else {
        3
    };
    colors[index.min(colors.len().saturating_sub(1))]
}

fn blend_over(dst: Option<[u8; 4]>, src: [u8; 4]) -> Option<[u8; 4]> {
    if src[3] == 0 {
        return dst;
    }
    let Some(dst) = dst else {
        return Some(src);
    };
    let src_a = src[3] as u32;
    let dst_a = dst[3] as u32;
    let inv_a = 255 - src_a;
    let out_a = (src_a + (dst_a * inv_a) / 255).min(255);
    if out_a == 0 {
        return None;
    }
    let denom = out_a * 255;
    Some([
        ((src[0] as u32 * src_a * 255 + dst[0] as u32 * dst_a * inv_a) / denom).min(255) as u8,
        ((src[1] as u32 * src_a * 255 + dst[1] as u32 * dst_a * inv_a) / denom).min(255) as u8,
        ((src[2] as u32 * src_a * 255 + dst[2] as u32 * dst_a * inv_a) / denom).min(255) as u8,
        out_a as u8,
    ])
}

fn layer_alpha(
    params: &IsoPaintBrushSample<'_>,
    layer: &BrushLayer,
    ox: i32,
    oy: i32,
    global_alpha: u8,
) -> u8 {
    let layer_mask = if layer.shape == "inherit" {
        255
    } else {
        shape_alpha(
            layer.shape,
            ox,
            oy,
            params.radius,
            255,
            params.seed ^ layer.seed,
        )
    };
    ((global_alpha as f32) * (layer_mask as f32 / 255.0) * layer.opacity)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn layer_color(
    params: &IsoPaintBrushSample<'_>,
    layer: &BrushLayer,
    ox: i32,
    oy: i32,
) -> Option<[u8; 4]> {
    let scale = layer.scale.max(1);
    let n = noise01(ox / scale, oy / scale, params.seed ^ layer.seed);
    let fine = noise01(ox, oy, params.seed ^ layer.seed ^ 0x6d2b_79f5);
    let coarse = noise01(ox / 5, oy / 5, params.seed ^ layer.seed ^ 0x1b87_3593);
    let color = if layer.use_input_color {
        params.color
    } else if !params.palette.is_empty() {
        pick_palette(n, params.palette, params.color)
    } else {
        pick_palette(n, layer.colors, params.color)
    };

    match layer.kind {
        BrushLayerKind::Fill if layer.use_input_color => Some(color),
        BrushLayerKind::Fill => {
            let shade = 0.78 + coarse * 0.44;
            Some(adjust_color(color, shade))
        }
        BrushLayerKind::Grain => {
            if fine > layer.density {
                return None;
            }
            let shade = 0.58 + n * 0.78;
            Some(adjust_color(color, shade))
        }
        BrushLayerKind::Fleck => {
            if fine < 1.0 - layer.density {
                return None;
            }
            let shade = if n > 0.5 { 1.28 } else { 0.58 };
            Some(adjust_color(color, shade))
        }
        BrushLayerKind::Blade => {
            let jitter = ((coarse - 0.5) * 4.0).round() as i32;
            let stripe = (oy + ox / 3 + jitter).rem_euclid(9);
            if stripe > 1 || fine > layer.density {
                return None;
            }
            Some(adjust_color(color, 1.10 + n * 0.35))
        }
        BrushLayerKind::Crack => {
            let jitter = (coarse - 0.5) * params.radius as f32 * 0.34;
            let main = (oy as f32 + ox as f32 * 0.28 + jitter).abs();
            let branch = (oy as f32 - ox as f32 * 0.58 - jitter * 0.7).abs();
            let width = (params.radius as f32 * 0.13).max(1.0);
            if main > width && !(fine > 0.74 && branch < width * 0.7) {
                return None;
            }
            Some(adjust_color(color, 0.55 + n * 0.35))
        }
    }
}

pub fn sample_pixel(params: &IsoPaintBrushSample<'_>, ox: i32, oy: i32) -> Option<[u8; 4]> {
    let base_alpha = (params.color[3] as f32 * params.opacity.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, 255.0) as u8;
    let global_alpha = shape_alpha(
        params.shape,
        ox,
        oy,
        params.radius,
        base_alpha,
        params.seed ^ 0xc001_d00d,
    );
    if global_alpha == 0 {
        return None;
    }

    let preset = preset_for_key(params.brush);
    let mut out = None;
    for layer in preset.layers {
        let alpha = layer_alpha(params, layer, ox, oy, global_alpha);
        if alpha == 0 {
            continue;
        }
        let Some(mut color) = layer_color(params, layer, ox, oy) else {
            continue;
        };
        color[3] = alpha;
        out = blend_over(out, color);
    }

    out
}
