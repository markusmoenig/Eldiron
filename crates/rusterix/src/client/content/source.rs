//! Resolved artwork descriptors borrow existing pixels. Their identity is cheap
//! to inspect; only cache misses materialize/copy a source image.
use super::*;
use crate::map::pixelsource::ResolvedTile;

pub(crate) enum TexturePlan<'a> {
    Tile(ResolvedTile<'a>, usize),
    Texture(&'a Texture),
    Square(&'a Texture),
}
impl<'a> TexturePlan<'a> {
    pub(crate) fn tile(tile: ResolvedTile<'a>, frame: usize) -> Option<Self> {
        if let ResolvedTile::Borrowed(source) = &tile {
            source.textures.get(source.frame_index(frame))?;
        }
        Some(Self::Tile(tile, frame))
    }
    fn fingerprint(&self) -> u64 {
        let mut h = rustc_hash::FxHasher::default();
        match self {
            Self::Tile(tile, frame) => {
                0u8.hash(&mut h);
                tile.frame_token(*frame).hash(&mut h);
            }
            Self::Texture(texture) => {
                1u8.hash(&mut h);
                (*texture as *const Texture as usize).hash(&mut h);
            }
            Self::Square(texture) => {
                2u8.hash(&mut h);
                (*texture as *const Texture as usize).hash(&mut h);
            }
        }
        h.finish()
    }
    pub(crate) fn build(&self) -> Option<(Texture, SourceFit)> {
        match self {
            Self::Tile(tile, frame) => {
                Some((tile.frame_texture(*frame)?.into_owned(), SourceFit::Stretch))
            }
            Self::Texture(texture) => Some(((*texture).clone(), SourceFit::Stretch)),
            Self::Square(texture) => {
                let size = texture.width.max(texture.height).max(1);
                let mut pixels = vec![0; size.checked_mul(size)?.checked_mul(4)?];
                let offset = [(size - texture.width) / 2, (size - texture.height) / 2];
                for y in 0..texture.height {
                    for x in 0..texture.width {
                        let source = (y * texture.width + x) * 4;
                        if let Some(pixel) = texture.data.get(source..source + 4) {
                            let target = ((y + offset[1]) * size + x + offset[0]) * 4;
                            pixels[target..target + 4].copy_from_slice(pixel);
                        }
                    }
                }
                Some((Texture::new(pixels, size, size), SourceFit::Contain))
            }
        }
    }
}
enum Artwork<'a> {
    Texture(TexturePlan<'a>),
    Item(&'a Item),
    Avatar(crate::avatar_builder::AvatarPreview<'a>),
    Missing,
}
pub struct ResolvedContent<'a> {
    assets: &'a Assets,
    artwork: Artwork<'a>,
    fingerprint: u64,
}
impl ResolvedContent<'_> {
    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
    }
    pub fn build(&self) -> Option<ContentArtwork> {
        match &self.artwork {
            Artwork::Texture(source) => source.build().map(|(texture, fit)| ContentArtwork {
                texture,
                fit,
                avatar: false,
            }),
            Artwork::Item(item) => {
                Widget::generated_item_source_texture(self.assets, item).map(|(texture, fit)| {
                    ContentArtwork {
                        texture,
                        fit,
                        avatar: false,
                    }
                })
            }
            Artwork::Avatar(source) => source.build().map(|output| ContentArtwork {
                texture: Texture::new(output.rgba, output.size as usize, output.size as usize),
                fit: SourceFit::Stretch,
                avatar: true,
            }),
            Artwork::Missing => None,
        }
    }
}
impl<'e> ContentRecipe<'e> {
    /// Resolve a concrete source/phase before a presentation cache lookup. The
    /// asset publication revision remains the caller's independent key domain.
    pub fn resolve<'a>(&self, assets: &'a Assets) -> ResolvedContent<'a>
    where
        'e: 'a,
    {
        let texture = match *self {
            Self::CommandIcon {
                command,
                state,
                frame,
            } => Widget::command_icon_texture(assets, Some(command), state, frame)
                .map(TexturePlan::Texture),
            Self::Texture { texture } => Some(TexturePlan::Texture(texture)),
            Self::Portrait { entity, frame } => Widget::portrait_tile_source(entity, assets)
                .and_then(|source| TexturePlan::tile(source, frame)),
            Self::Profile {
                entity,
                artwork_frame,
                ..
            } => ProfileWidget::explicit_profile_source(entity, assets, artwork_frame),
            Self::Item { item, frame } => Widget::item_texture_plan(assets, item, frame),
            Self::Avatar { .. } => None,
        };
        if let Some(texture) = texture {
            let fingerprint = (0u8, texture.fingerprint());
            return ResolvedContent {
                assets,
                artwork: Artwork::Texture(texture),
                fingerprint: crate::procedural_avatar::fingerprint(&fingerprint),
            };
        }
        let (artwork, fingerprint) = match *self {
            Self::CommandIcon { command, state, .. } => (
                Artwork::Missing,
                Self::CommandIcon {
                    command,
                    state,
                    frame: 0,
                }
                .fingerprint(),
            ),
            Self::Texture { .. } => unreachable!("borrowed source textures resolve above"),
            Self::Item { item, .. } => (
                Artwork::Item(item),
                Self::Item { item, frame: 0 }.fingerprint(),
            ),
            Self::Portrait { entity, .. } => (
                Artwork::Missing,
                Self::Portrait { entity, frame: 0 }.fingerprint(),
            ),
            Self::Profile {
                entity, settings, ..
            }
            | Self::Avatar { entity, settings } => {
                let base = Self::Avatar {
                    entity,
                    settings: AvatarSourceSettings {
                        frame: 0,
                        ..settings
                    },
                }
                .fingerprint();
                if let Some(avatar) = settings
                    .avatar
                    .and_then(|name| assets.avatars.get(name))
                    .or_else(|| AvatarRuntimeBuilder::find_avatar_for_entity(entity, assets))
                {
                    let source = AvatarRuntimeBuilder::resolve_preview(
                        entity,
                        avatar,
                        assets,
                        settings.animation,
                        settings.direction,
                        settings.frame,
                        AvatarShadingOptions::default(),
                        settings.weapons,
                    );
                    let fingerprint =
                        crate::procedural_avatar::fingerprint(&(base, source.fingerprint()));
                    (Artwork::Avatar(source), fingerprint)
                } else {
                    (Artwork::Missing, base)
                }
            }
        };
        ResolvedContent {
            assets,
            artwork,
            fingerprint: crate::procedural_avatar::fingerprint(&(1u8, fingerprint)),
        }
    }
}
