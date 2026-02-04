use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VertexBlendPreset {
    /// [0.0, 0.0, 0.0, 0.0]
    Solid,

    /// [0.5, 0.5, 0.5, 0.5]
    FullBlend,

    /// [1.0, 1.0, 1.0, 1.0]
    Secondary,

    // ─────────────────────────────
    // Edge blends
    // ─────────────────────────────
    /// Top edge toward secondary
    /// [1.0, 1.0, 0.0, 0.0]
    Top,

    /// Bottom edge
    /// [0.0, 0.0, 1.0, 1.0]
    Bottom,

    /// Left edge
    /// [1.0, 0.0, 0.0, 1.0]
    Left,

    /// Right edge
    /// [0.0, 1.0, 1.0, 0.0]
    Right,

    // ─────────────────────────────
    // Soft edge blends (half strength)
    // ─────────────────────────────
    /// [0.5, 0.5, 0.0, 0.0]
    TopSoft,

    /// [0.0, 0.0, 0.5, 0.5]
    BottomSoft,

    /// [0.5, 0.0, 0.0, 0.5]
    LeftSoft,

    /// [0.0, 0.5, 0.5, 0.0]
    RightSoft,

    // ─────────────────────────────
    // Corners
    // ─────────────────────────────
    /// [1.0, 0.0, 0.0, 0.0]
    TopLeft,

    /// [0.0, 1.0, 0.0, 0.0]
    TopRight,

    /// [0.0, 0.0, 1.0, 0.0]
    BottomRight,

    /// [0.0, 0.0, 0.0, 1.0]
    BottomLeft,

    // ─────────────────────────────
    // Soft corners
    // ─────────────────────────────
    /// [0.5, 0.0, 0.0, 0.0]
    TopLeftSoft,

    /// [0.0, 0.5, 0.0, 0.0]
    TopRightSoft,

    /// [0.0, 0.0, 0.5, 0.0]
    BottomRightSoft,

    /// [0.0, 0.0, 0.0, 0.5]
    BottomLeftSoft,
}

impl VertexBlendPreset {
    /// Returns vertex weights in [TL, TR, BR, BL]
    pub fn weights(self) -> [f32; 4] {
        match self {
            Self::Solid => [0.0, 0.0, 0.0, 0.0],
            Self::FullBlend => [0.5, 0.5, 0.5, 0.5],
            Self::Secondary => [1.0, 1.0, 1.0, 1.0],

            Self::Top => [1.0, 1.0, 0.0, 0.0],
            Self::Bottom => [0.0, 0.0, 1.0, 1.0],
            Self::Left => [1.0, 0.0, 0.0, 1.0],
            Self::Right => [0.0, 1.0, 1.0, 0.0],

            Self::TopSoft => [0.5, 0.5, 0.0, 0.0],
            Self::BottomSoft => [0.0, 0.0, 0.5, 0.5],
            Self::LeftSoft => [0.5, 0.0, 0.0, 0.5],
            Self::RightSoft => [0.0, 0.5, 0.5, 0.0],

            Self::TopLeft => [1.0, 0.0, 0.0, 0.0],
            Self::TopRight => [0.0, 1.0, 0.0, 0.0],
            Self::BottomRight => [0.0, 0.0, 1.0, 0.0],
            Self::BottomLeft => [0.0, 0.0, 0.0, 1.0],

            Self::TopLeftSoft => [0.5, 0.0, 0.0, 0.0],
            Self::TopRightSoft => [0.0, 0.5, 0.0, 0.0],
            Self::BottomRightSoft => [0.0, 0.0, 0.5, 0.0],
            Self::BottomLeftSoft => [0.0, 0.0, 0.0, 0.5],
        }
    }

    /// Transform preset based on surface orientation in world space.
    /// For non-horizontal surfaces (walls, slopes), flip Top/Bottom since UV-up points world-up.
    ///
    /// surface_normal: the normal vector of the surface
    pub fn orient_to_world(self, surface_normal: vek::Vec3<f32>) -> Self {
        // Only keep Top/Bottom as-is for very flat horizontal surfaces (floors/ceilings)
        // For everything else (walls, slopes), flip Top/Bottom
        let is_flat_horizontal = surface_normal.y.abs() > 0.9;

        let mut result = self;

        if !is_flat_horizontal {
            // On non-horizontal surfaces (walls, slopes), flip Top/Bottom
            result = match result {
                Self::Top => Self::Bottom,
                Self::Bottom => Self::Top,
                Self::TopSoft => Self::BottomSoft,
                Self::BottomSoft => Self::TopSoft,
                Self::TopLeft => Self::BottomLeft,
                Self::TopRight => Self::BottomRight,
                Self::BottomLeft => Self::TopLeft,
                Self::BottomRight => Self::TopRight,
                Self::TopLeftSoft => Self::BottomLeftSoft,
                Self::TopRightSoft => Self::BottomRightSoft,
                Self::BottomLeftSoft => Self::TopLeftSoft,
                Self::BottomRightSoft => Self::TopRightSoft,
                _ => result,
            };
        }

        result
    }

    pub fn preview_vertex_blend(
        &self,
        weights: [f32; 4], // [TL, TR, BR, BL] in 0.0..1.0
        size: usize,
    ) -> Vec<u8> {
        let mut out = Vec::with_capacity(size * size * 4);

        let tl = weights[0];
        let tr = weights[1];
        let br = weights[2];
        let bl = weights[3];

        for y in 0..size {
            let v = if size > 1 {
                y as f32 / (size - 1) as f32
            } else {
                0.0
            };

            // interpolate vertically later
            let left = lerp(tl, bl, v);
            let right = lerp(tr, br, v);

            for x in 0..size {
                let u = if size > 1 {
                    x as f32 / (size - 1) as f32
                } else {
                    0.0
                };

                let w = lerp(left, right, u).clamp(0.0, 1.0);

                // Primary = black, Secondary = white
                let c = (w * 255.0).round() as u8;

                out.push(c); // R
                out.push(c); // G
                out.push(c); // B
                out.push(255); // A
            }
        }

        out
    }

    #[inline]
    pub fn to_index(self) -> usize {
        match self {
            VertexBlendPreset::Solid => 0,
            VertexBlendPreset::FullBlend => 1,
            VertexBlendPreset::Secondary => 2,

            VertexBlendPreset::Top => 3,
            VertexBlendPreset::Bottom => 4,
            VertexBlendPreset::Left => 5,
            VertexBlendPreset::Right => 6,

            VertexBlendPreset::TopSoft => 7,
            VertexBlendPreset::BottomSoft => 8,
            VertexBlendPreset::LeftSoft => 9,
            VertexBlendPreset::RightSoft => 10,

            VertexBlendPreset::TopLeft => 11,
            VertexBlendPreset::TopRight => 12,
            VertexBlendPreset::BottomRight => 13,
            VertexBlendPreset::BottomLeft => 14,

            VertexBlendPreset::TopLeftSoft => 15,
            VertexBlendPreset::TopRightSoft => 16,
            VertexBlendPreset::BottomRightSoft => 17,
            VertexBlendPreset::BottomLeftSoft => 18,
        }
    }

    #[inline]
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(VertexBlendPreset::Solid),
            1 => Some(VertexBlendPreset::FullBlend),
            2 => Some(VertexBlendPreset::Secondary),

            3 => Some(VertexBlendPreset::Top),
            4 => Some(VertexBlendPreset::Bottom),
            5 => Some(VertexBlendPreset::Left),
            6 => Some(VertexBlendPreset::Right),

            7 => Some(VertexBlendPreset::TopSoft),
            8 => Some(VertexBlendPreset::BottomSoft),
            9 => Some(VertexBlendPreset::LeftSoft),
            10 => Some(VertexBlendPreset::RightSoft),

            11 => Some(VertexBlendPreset::TopLeft),
            12 => Some(VertexBlendPreset::TopRight),
            13 => Some(VertexBlendPreset::BottomRight),
            14 => Some(VertexBlendPreset::BottomLeft),

            15 => Some(VertexBlendPreset::TopLeftSoft),
            16 => Some(VertexBlendPreset::TopRightSoft),
            17 => Some(VertexBlendPreset::BottomRightSoft),
            18 => Some(VertexBlendPreset::BottomLeftSoft),

            _ => None,
        }
    }

    /// Canonical iteration order (sorted by importance / frequency).
    pub const ALL: &'static [VertexBlendPreset] = &[
        // ── Most common ─────────────────────────────
        VertexBlendPreset::Solid,
        VertexBlendPreset::FullBlend,
        VertexBlendPreset::Secondary,
        // ── Primary edges ───────────────────────────
        VertexBlendPreset::Top,
        VertexBlendPreset::Bottom,
        VertexBlendPreset::Left,
        VertexBlendPreset::Right,
        // ── Soft edges ──────────────────────────────
        VertexBlendPreset::TopSoft,
        VertexBlendPreset::BottomSoft,
        VertexBlendPreset::LeftSoft,
        VertexBlendPreset::RightSoft,
        // ── Corners ─────────────────────────────────
        VertexBlendPreset::TopLeft,
        VertexBlendPreset::TopRight,
        VertexBlendPreset::BottomRight,
        VertexBlendPreset::BottomLeft,
        // ── Soft corners ────────────────────────────
        VertexBlendPreset::TopLeftSoft,
        VertexBlendPreset::TopRightSoft,
        VertexBlendPreset::BottomRightSoft,
        VertexBlendPreset::BottomLeftSoft,
    ];
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
