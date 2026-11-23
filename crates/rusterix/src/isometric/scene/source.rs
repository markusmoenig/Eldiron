use crate::{
    Tile,
    isometric::{AvatarSprite, Projection, paint, surface},
};
use uuid::Uuid;
use vek::Vec2;

/// Presentation identity remains independent of painter ties and runtime ID reuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpriteOwner {
    Sector(u32),
    Item {
        index: usize,
        id: u32,
        creator_id: Uuid,
    },
    Entity {
        index: usize,
        id: u32,
        creator_id: Uuid,
    },
    TerrainStamp(Uuid),
}

/// A borrowed sprite instance, including reference support/occlusion policy.
pub struct Sprite<'a> {
    pub owner: Option<SpriteOwner>,
    pub tile: Option<&'a Tile>,
    pub avatar: Option<&'a AvatarSprite>,
    pub stamp: Option<&'a paint::TerrainStamp>,
    pub scale: f32,
    pub pos: Vec2<f32>,
    pub height: f32,
    pub frame: usize,
    pub opacity: f32,
    pub tie: u64,
    pub support: Option<&'a crate::Sector>,
    pub radius: f32,
    pub floor: Option<&'a crate::Sector>,
}

/// Immutable source pixels and source-space geometry for one selected frame.
pub struct SpriteImage<'a> {
    pub data: &'a [u8],
    pub width: usize,
    pub height: usize,
    pub anchor: Vec2<f32>,
    pub unit: f32,
    pub surface: Option<&'a surface::SurfaceFrame>,
    pub bitmap: bool,
    pub prelit: bool,
    pub ground: bool,
}

/// View-space foot metadata shared by reference and native terrain clipping.
#[derive(Clone, Copy, Debug)]
pub struct TerrainContact {
    pub foot_y: f32,
    pub foot_height: f32,
    pub foot_depth: f32,
    pub unit: f32,
}
impl TerrainContact {
    pub fn hides(
        &self,
        depth: f32,
        height: f32,
        pixel_y: f32,
        surface_height: Option<f32>,
    ) -> bool {
        if let Some(surface_height) = surface_height {
            // Height increases toward the camera along the ray for this pixel.
            return depth.is_finite() && height > surface_height + 0.06;
        }
        height > self.foot_height + 0.06
            && depth
                > self.foot_depth + (self.foot_y - pixel_y - 0.5) / self.unit * 0.6666667 + 0.02
    }
}
impl<'a> Sprite<'a> {
    pub fn terrain_contact(&self, projection: Projection) -> Option<TerrainContact> {
        (self.support.is_none() && self.floor.is_none()).then(|| TerrainContact {
            foot_y: projection.screen(self.pos, self.height).y,
            foot_height: self.height,
            foot_depth: self.pos.y - self.pos.x + self.height * 0.8164966,
            unit: projection.unit,
        })
    }

    /// Stable reference order shared by GPU submission and contact protection.
    pub fn painter_cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.pos.y - self.pos.x + self.height * 0.8164966)
            .total_cmp(&(other.pos.y - other.pos.x + other.height * 0.8164966))
            .then(self.tie.cmp(&other.tie))
    }

    pub fn image(&self) -> Option<SpriteImage<'a>> {
        self.image_info(true)
    }

    // Broad-phase overlap needs dimensions only. Distant archived tiles can stay compressed.
    pub(in crate::isometric) fn image_info(&self, pixels: bool) -> Option<SpriteImage<'a>> {
        if let Some(stamp) = self.stamp {
            return Some(SpriteImage {
                data: if pixels {
                    &stamp.sprite.texture.data
                } else {
                    &[]
                },
                width: stamp.sprite.texture.width,
                height: stamp.sprite.texture.height,
                anchor: Vec2::from(stamp.sprite.anchor),
                unit: paint::PIXELS_PER_UNIT,
                surface: stamp.sprite.surface.as_ref(),
                bitmap: false,
                prelit: false,
                ground: false,
            });
        }
        if let Some(avatar) = self.avatar {
            return Some(SpriteImage {
                data: if pixels { &avatar.pixels } else { &[] },
                width: avatar.size[0],
                height: avatar.size[1],
                anchor: Vec2::from(avatar.anchor),
                unit: avatar.pixels_per_unit,
                surface: avatar.surface.as_deref(),
                bitmap: false,
                prelit: false,
                ground: false,
            });
        }
        let tile = self.tile?;
        let (textures, anchor, unit) = if let Some(layout) = &tile.isometric {
            (
                if layout.frames.is_empty() {
                    &tile.textures
                } else {
                    &layout.frames
                },
                Vec2::new(layout.anchor[0] as f32, layout.anchor[1] as f32),
                layout.pixels_per_unit,
            )
        } else {
            let image = tile.textures.first()?;
            (
                &tile.textures,
                Vec2::new(image.width as f32 * 0.5, image.height as f32),
                image.height as f32 / 2.,
            )
        };
        let texture = textures.get(self.frame)?;
        Some(SpriteImage {
            data: if pixels { &texture.data } else { &[] },
            width: texture.width,
            height: texture.height,
            anchor,
            unit,
            surface: tile
                .isometric
                .as_ref()
                .and_then(|s| s.surfaces.get(self.frame)),
            bitmap: tile.isometric.as_ref().is_some_and(|s| s.bitmap),
            prelit: tile.isometric.as_ref().is_some_and(|s| s.prelit),
            ground: tile.isometric.as_ref().is_some_and(|s| s.ground),
        })
    }

    pub fn image_placement(
        &self,
        image: &SpriteImage<'_>,
        projection: Projection,
    ) -> (Vec2<f32>, f32) {
        let scale = projection.unit / image.unit.max(0.001) * self.scale.max(0.01);
        (
            projection.screen(self.pos, self.height) - image.anchor * scale,
            scale,
        )
    }
}
