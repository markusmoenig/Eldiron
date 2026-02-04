// rusteria/src/textures/patterns.rs
//! Global pattern bank: compute-once tileable textures stored as `TexStorage`.
//! Call `ensure_patterns_initialized()` once at startup, or any accessor will lazily init.

use super::TexStorage;
use once_cell::sync::OnceCell;
use std::path::Path;
use strum::{EnumCount, EnumIter, IntoStaticStr};

/// Global storage of precomputed patterns.
static PATTERNS: OnceCell<Vec<TexStorage>> = OnceCell::new();
static PATTERNS_NORMAL: OnceCell<Vec<TexStorage>> = OnceCell::new();

/// Enum of all available patterns, matches the build order in `build_patterns()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumCount, EnumIter, IntoStaticStr)]
pub enum PatternKind {
    ValueNoise,
    FbmValueNoise,
    PerlinNoise,
    FbmPerlinNoise,
    Bricks,
    Tiles,
    Blocks,
}

impl PatternKind {
    pub fn to_index(self) -> usize {
        match self {
            PatternKind::ValueNoise => 0,
            PatternKind::FbmValueNoise => 1,
            PatternKind::PerlinNoise => 2,
            PatternKind::FbmPerlinNoise => 3,
            PatternKind::Bricks => 4,
            PatternKind::Tiles => 5,
            PatternKind::Blocks => 6,
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        Some(match i {
            0 => PatternKind::ValueNoise,
            1 => PatternKind::FbmValueNoise,
            2 => PatternKind::PerlinNoise,
            3 => PatternKind::FbmPerlinNoise,
            4 => PatternKind::Bricks,
            5 => PatternKind::Tiles,
            6 => PatternKind::Blocks,
            _ => return None,
        })
    }
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "value" => Some(PatternKind::ValueNoise),
            "fbm_value" => Some(PatternKind::FbmValueNoise),
            "perlin" => Some(PatternKind::PerlinNoise),
            "fbm_perlin" => Some(PatternKind::FbmPerlinNoise),
            "bricks" => Some(PatternKind::Bricks),
            "tiles" => Some(PatternKind::Tiles),
            "blocks" => Some(PatternKind::Blocks),
            _ => None,
        }
    }
    pub fn display_name(self) -> &'static str {
        match self {
            PatternKind::ValueNoise => "value",
            PatternKind::FbmValueNoise => "fbm_value",
            PatternKind::PerlinNoise => "perlin",
            PatternKind::FbmPerlinNoise => "fbm_perlin",
            PatternKind::Bricks => "bricks",
            PatternKind::Tiles => "tiles",
            PatternKind::Blocks => "blocks",
        }
    }
}

/// Returns true if patterns have already been computed and stored.
#[inline]
pub fn patterns_computed() -> bool {
    PATTERNS.get().is_some()
}

/// Ensure the global patterns vector is initialized. Safe to call multiple times.
pub fn ensure_patterns_initialized() {
    let _ = PATTERNS.get_or_init(|| build_patterns());
}

/// Get an immutable slice of all precomputed patterns. Lazily initializes on first call.
#[inline]
pub fn patterns() -> &'static [TexStorage] {
    PATTERNS.get_or_init(|| build_patterns()).as_slice()
}

/// Get a specific pattern by id. Panics if out of range.
#[inline]
pub fn pattern(id: usize) -> &'static TexStorage {
    let vec = PATTERNS.get_or_init(|| build_patterns());
    &vec[id]
}

/// Get a specific pattern by id.
pub fn pattern_safe(id: usize) -> Option<&'static TexStorage> {
    let vec = PATTERNS.get_or_init(|| build_patterns());
    vec.get(id)
}

/// Returns true if normal patterns have already been computed and stored.
#[inline]
pub fn patterns_normal_computed() -> bool {
    PATTERNS_NORMAL.get().is_some()
}

/// Ensure the global normal patterns vector is initialized. Safe to call multiple times.
pub fn ensure_patterns_normal_initialized() {
    let _ = PATTERNS_NORMAL.get_or_init(|| build_patterns());
}

/// Get an immutable slice of all precomputed normal patterns. Lazily initializes on first call.
#[inline]
pub fn patterns_normal() -> &'static [TexStorage] {
    PATTERNS_NORMAL.get_or_init(|| build_patterns()).as_slice()
}

/// Get a specific normal pattern by id. Panics if out of range.
#[inline]
pub fn pattern_normal(id: usize) -> &'static TexStorage {
    let vec = PATTERNS_NORMAL.get_or_init(|| build_patterns());
    &vec[id]
}

/// Get a specific normal pattern by id.
pub fn pattern_normal_safe(id: usize) -> Option<&'static TexStorage> {
    let vec = PATTERNS_NORMAL.get_or_init(|| build_patterns());
    vec.get(id)
}

fn build_patterns() -> Vec<TexStorage> {
    const PATTERN_COUNT: usize = PatternKind::COUNT;
    let mut out: Vec<TexStorage> = (0..PATTERN_COUNT).map(|_| TexStorage::new(1, 1)).collect();
    let mut normals: Vec<TexStorage> = (0..PATTERN_COUNT).map(|_| TexStorage::new(1, 1)).collect();

    for file in crate::Embedded::iter() {
        let path_str = file.as_ref();
        if !path_str.ends_with(".png") {
            continue;
        }

        let stem_lower = Path::new(path_str)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let is_normal = stem_lower.ends_with("_normal");
        let base_stem = if is_normal {
            &stem_lower[..stem_lower.len().saturating_sub("_normal".len())]
        } else {
            &stem_lower
        };

        if let Some(kind) = PatternKind::from_name(base_stem) {
            if let Some(bytes) = crate::Embedded::get(path_str) {
                if let Ok(tex) = TexStorage::from_png_bytes(bytes.data.as_ref()) {
                    if is_normal {
                        normals[kind.to_index()] = tex;
                    } else {
                        out[kind.to_index()] = tex;
                    }
                }
            }
        }
    }

    let _ = PATTERNS_NORMAL.set(normals);

    out
}
