//! Avatar definitions retain content signatures until mutable registry access.
//! In-place frame/anchor edits, entry APIs, replacement and cloned registries all
//! invalidate correctly; callers need no explicit revision bookkeeping.
use crate::Avatar;
use rustc_hash::FxHashMap;
use std::{
    ops::{Deref, DerefMut},
    sync::RwLock,
};

#[derive(Default)]
pub struct AvatarAssets {
    data: FxHashMap<String, Avatar>,
    signatures: RwLock<FxHashMap<usize, u64>>,
}

impl AvatarAssets {
    pub(crate) fn cached_signature(&self, avatar: &Avatar, calculate: impl FnOnce() -> u64) -> u64 {
        let key = avatar as *const Avatar as usize;
        if let Some(signature) = self.signatures.read().unwrap().get(&key).copied() {
            return signature;
        }
        let signature = calculate();
        // AvatarBuilder also accepts standalone editable definitions. They have
        // no registry mutation boundary and must continue to validate by content.
        if self.data.values().any(|owned| std::ptr::eq(owned, avatar)) {
            self.signatures.write().unwrap().insert(key, signature);
        }
        signature
    }
}

impl Clone for AvatarAssets {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            signatures: Default::default(),
        }
    }
}
impl Deref for AvatarAssets {
    type Target = FxHashMap<String, Avatar>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl DerefMut for AvatarAssets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.signatures.get_mut().unwrap().clear();
        &mut self.data
    }
}
impl<'a> IntoIterator for &'a AvatarAssets {
    type Item = (&'a String, &'a Avatar);
    type IntoIter = std::collections::hash_map::Iter<'a, String, Avatar>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}
impl<'a> IntoIterator for &'a mut AvatarAssets {
    type Item = (&'a String, &'a mut Avatar);
    type IntoIter = std::collections::hash_map::IterMut<'a, String, Avatar>;
    fn into_iter(self) -> Self::IntoIter {
        self.signatures.get_mut().unwrap().clear();
        self.data.iter_mut()
    }
}
impl FromIterator<(String, Avatar)> for AvatarAssets {
    fn from_iter<T: IntoIterator<Item = (String, Avatar)>>(items: T) -> Self {
        Self {
            data: items.into_iter().collect(),
            signatures: Default::default(),
        }
    }
}

#[cfg(all(test, feature = "graphics"))]
mod tests {
    use super::*;
    use crate::{
        Texture,
        avatar::{AvatarAnimation, AvatarAnimationFrame, AvatarPerspective},
        avatar_builder::AvatarRuntimeBuilder,
    };

    fn signature(assets: &AvatarAssets) -> u64 {
        assets.cached_signature(&assets["body"], || {
            AvatarRuntimeBuilder::avatar_definition_signature(&assets["body"])
        })
    }
    #[test]
    fn signatures_reuse_unchanged_pixels_and_invalidate_every_mutable_access_route() {
        let mut assets = AvatarAssets::default();
        let mut avatar = Avatar::default();
        avatar.animations.push(AvatarAnimation {
            perspectives: vec![AvatarPerspective {
                frames: vec![AvatarAnimationFrame::new(Texture::from_color([
                    30, 60, 90, 255,
                ]))],
                ..Default::default()
            }],
            ..Default::default()
        });
        assets.insert("body".into(), avatar);
        let original = signature(&assets);
        assert_eq!(
            assets.cached_signature(&assets["body"], || panic!("unchanged avatar rehashed")),
            original
        );
        assets.get_mut("body").unwrap().animations[0].perspectives[0].frames[0]
            .texture
            .data[0] = 31;
        let edited = signature(&assets);
        assert_ne!(original, edited);
        for (_, avatar) in &mut assets {
            avatar.animations[0].perspectives[0].frames[0].weapon_main_anchor = Some((3, 4));
        }
        let anchor = signature(&assets);
        assert_ne!(edited, anchor);
        assets
            .entry("body".into())
            .and_modify(|a| a.animations[0].speed = 2.);
        assert_ne!(signature(&assets), anchor);
        let mut cloned = assets.clone();
        assert_eq!(signature(&cloned), signature(&assets));
        cloned.get_mut("body").unwrap().name = "Changed clone".into();
        assert_ne!(signature(&cloned), signature(&assets));
        assets.remove("body");
        assets.insert("body".into(), Avatar::default());
        assert_ne!(signature(&assets), original);
        let mut external = Avatar::default();
        let first = assets.cached_signature(&external, || {
            AvatarRuntimeBuilder::avatar_definition_signature(&external)
        });
        external.name = "Standalone edit".into();
        let second = assets.cached_signature(&external, || {
            AvatarRuntimeBuilder::avatar_definition_signature(&external)
        });
        assert_ne!(first, second);
    }
}
