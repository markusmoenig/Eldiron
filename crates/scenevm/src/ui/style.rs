use rustc_hash::FxHashMap;
use uuid::Uuid;
use vek::Vec4;

use crate::{Atom, VM};

/// Style parameters packed into a tiny atlas tile (2x1).
/// `radius_px` and `border_px` are in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StyleParams {
    pub fill: Vec4<f32>,
    pub border: Vec4<f32>,
    pub radius_px: f32,
    pub border_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StyleId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct StyleKey {
    fill: [u8; 4],
    border: [u8; 4],
    radius_q: u8,
    border_q: u8,
}

struct StyleRecord {
    id: StyleId,
    tile_id: Uuid,
}

pub struct StyleRegistry {
    styles: FxHashMap<StyleKey, StyleRecord>,
    dirty: bool,
}

impl StyleRegistry {
    pub fn new() -> Self {
        Self {
            styles: FxHashMap::default(),
            dirty: false,
        }
    }

    pub fn ensure_style(&mut self, vm: &mut VM, params: StyleParams) -> StyleId {
        let key = to_key(params);
        if let Some(rec) = self.styles.get(&key) {
            return rec.id;
        }
        // Pack fill/border into color frames (2x1), radius/border_norm into material.
        let tile_id = Uuid::new_v4();
        let fill = to_u8(params.fill);
        let border = to_u8(params.border);
        let mut color_pixels = Vec::from(fill);
        color_pixels.extend_from_slice(&border);

        // material: texel0.r = widget_type (1=button), texel0.g = radius_px (clamped to 0-255),
        //           texel0.b = 255 marks "style" tile, texel0.a = border_px (clamped to 0-255)
        let widget_type = 1u8; // 1 = button/rounded rect
        let radius = params.radius_px.clamp(0.0, 255.0).round() as u8;
        let border = params.border_px.clamp(0.0, 255.0).round() as u8;
        let mat_tex0 = [widget_type, radius, 255, border];
        let mat_tex1 = [widget_type, radius, 255, border]; // Both texels need same data
        let mut mat_pixels = Vec::from(mat_tex0);
        mat_pixels.extend_from_slice(&mat_tex1);

        vm.execute(Atom::AddTile {
            id: tile_id,
            width: 2,
            height: 1,
            frames: vec![color_pixels],
            material_frames: Some(vec![mat_pixels]),
        });

        let id = StyleId(tile_id);
        self.styles.insert(key, StyleRecord { id, tile_id });
        // Build atlas immediately so tile frames are valid for subsequent geometry.
        vm.execute(Atom::BuildAtlas);
        self.dirty = false;
        id
    }

    pub fn tile_id(&self, id: StyleId) -> Option<Uuid> {
        self.styles.values().find(|r| r.id == id).map(|r| r.tile_id)
    }

    pub fn build_if_dirty(&mut self, vm: &mut VM) {
        if self.dirty {
            vm.execute(Atom::BuildAtlas);
            self.dirty = false;
        }
    }
}

fn to_u8(v: Vec4<f32>) -> [u8; 4] {
    let clamp = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    [clamp(v.x), clamp(v.y), clamp(v.z), clamp(v.w)]
}

fn to_key(p: StyleParams) -> StyleKey {
    StyleKey {
        fill: to_u8(p.fill),
        border: to_u8(p.border),
        radius_q: p.radius_px.clamp(0.0, 255.0).round() as u8,
        border_q: p.border_px.clamp(0.0, 255.0).round() as u8,
    }
}
