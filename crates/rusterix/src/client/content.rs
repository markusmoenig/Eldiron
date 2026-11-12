//! CPU source-art recipes. These never allocate a widget-sized destination,
//! draw text, access a server or require a graphics device.
use super::widget::{Widget, profile::ProfileWidget};
use crate::avatar_builder::AvatarRuntimeBuilder;
use crate::{Assets, AvatarDirection, AvatarShadingOptions, Entity, Item, Texture};
use std::hash::{Hash, Hasher};
mod source;
pub use source::ResolvedContent;
pub(crate) use source::TexturePlan;

#[derive(Clone, Copy, Debug)]
pub struct AvatarSourceSettings<'a> {
    pub avatar: Option<&'a str>,
    pub animation: Option<&'a str>,
    pub direction: AvatarDirection,
    pub frame: usize,
    pub weapons: bool,
}

/// Fitting is presentation policy, independent of source pixel dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFit {
    Stretch,
    Contain,
}

pub struct ContentArtwork {
    pub texture: Texture,
    pub fit: SourceFit,
    /// Profile cropping applies to generated avatars, never explicit portraits.
    pub avatar: bool,
}

#[derive(Clone, Copy)]
pub enum ContentRecipe<'a> {
    Portrait {
        entity: &'a Entity,
        frame: usize,
    },
    Avatar {
        entity: &'a Entity,
        settings: AvatarSourceSettings<'a>,
    },
    Profile {
        entity: &'a Entity,
        settings: AvatarSourceSettings<'a>,
        artwork_frame: usize,
    },
    Item {
        item: &'a Item,
        frame: usize,
    },
}

impl ContentRecipe<'_> {
    /// Conservative recipe identity. Concrete-source caches use `resolve` and
    /// its selected-phase fingerprint instead of this absolute-frame identity.
    /// The owner must also invalidate on asset edits/publication. Entity motion,
    /// health and display names do not change an avatar source; appearance and
    /// equipment do. ValueContainer fingerprints avoid serializing every item
    /// on an unchanged frame.
    pub fn fingerprint(&self) -> u64 {
        let mut h = rustc_hash::FxHasher::default();
        match *self {
            Self::Portrait { entity, frame } => {
                3u8.hash(&mut h);
                frame.hash(&mut h);
                if let Some(source) = entity.attributes.get("portrait_tile_id") {
                    crate::fingerprint::hash_serialized(&mut h, source);
                }
            }
            Self::Avatar { entity, settings }
            | Self::Profile {
                entity, settings, ..
            } => {
                if let Self::Profile { artwork_frame, .. } = *self {
                    1u8.hash(&mut h);
                    artwork_frame.hash(&mut h);
                } else {
                    0u8.hash(&mut h);
                }
                entity.creator_id.hash(&mut h);
                entity.id.hash(&mut h);
                settings.avatar.hash(&mut h);
                settings.animation.hash(&mut h);
                settings.direction.hash(&mut h);
                settings.frame.hash(&mut h);
                settings.weapons.hash(&mut h);
                // All supported avatar/profile bindings, procedural controls and
                // legacy marker overrides. Canonical order survives deserialization.
                let mut attrs = entity
                    .attributes
                    .iter()
                    .filter(|(name, _)| {
                        name.starts_with("avatar")
                            || name.starts_with("appearance_")
                            || name.starts_with("profile_")
                            || name.starts_with("portrait_")
                            || name.ends_with("_color")
                            || name.ends_with("_index")
                            || matches!(name.as_str(), "source" | "tile_id")
                    })
                    .collect::<Vec<_>>();
                attrs.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
                for (name, value) in attrs {
                    name.hash(&mut h);
                    crate::fingerprint::hash_serialized(&mut h, value);
                }
                // Order matters: equipped marker overrides use declaration order.
                for (slot, item) in &entity.equipped {
                    slot.hash(&mut h);
                    crate::fingerprint::hash_item_appearance(&mut h, item);
                }
                // Some procedural equipment policies inspect the inventory too.
                crate::fingerprint::hash_character_items(&mut h, entity);
            }
            Self::Item { item, frame } => {
                2u8.hash(&mut h);
                frame.hash(&mut h);
                crate::fingerprint::hash_item_appearance(&mut h, item);
            }
        }
        h.finish()
    }

    pub fn build(&self, assets: &Assets) -> Option<ContentArtwork> {
        self.resolve(assets).build()
    }
}
