//! Bounded CPU passage ownership, using elapsed presentation time supplied by
//! the application. Layout publishes page counts; it cannot redeliver messages.
use super::*;
use crate::{Assets, Map};
use std::time::Duration;
use theframework_core::TheTime;

pub fn lifetime(text: &str) -> Duration {
    Duration::from_secs_f32((text.chars().count() as f32 * 0.07 + 5.).clamp(6., 18.))
}
pub struct WorldPassage {
    pub id: Uuid,
    pub region: Uuid,
    pub message: WorldText,
    pub page: usize,
    pub pages: usize,
    opened: Duration,
}
#[derive(Default)]
pub struct WorldTexts {
    elapsed: Duration,
    passages: Vec<WorldPassage>,
}
impl WorldTexts {
    pub fn passages(&self) -> &[WorldPassage] {
        &self.passages
    }
    pub fn advance(&mut self, elapsed: Duration) {
        self.elapsed = self.elapsed.saturating_add(elapsed);
    }
    pub fn clear(&mut self) -> bool {
        let changed = !self.passages.is_empty();
        self.passages.clear();
        changed
    }
    pub fn receive(
        &mut self,
        map: &Map,
        assets: &Assets,
        time: &TheTime,
        messages: &[(u32, WorldText)],
    ) {
        for (receiver, message) in messages {
            let Some(message) = resolve_for_player(map, assets, time, *receiver, message.clone())
            else {
                continue;
            };
            self.passages
                .retain(|p| p.region == map.id && p.message.anchor != message.anchor);
            if self.passages.len() >= 4 {
                self.passages.remove(0);
            }
            self.passages.push(WorldPassage {
                id: Uuid::new_v4(),
                region: map.id,
                message,
                opened: self.elapsed,
                page: 0,
                pages: 1,
            });
        }
    }
    pub fn expire(&mut self, region: Option<Uuid>) {
        self.passages.retain(|p| {
            Some(p.region) == region
                && (p.pages > 1
                    || self.elapsed.saturating_sub(p.opened) < lifetime(&p.message.text))
        });
    }
    pub fn set_page_count(&mut self, id: Uuid, pages: usize) {
        if let Some(p) = self.passages.iter_mut().find(|p| p.id == id) {
            p.pages = pages.max(1);
            p.page = p.page.min(p.pages - 1);
        }
    }
    pub fn activate(&mut self, id: Uuid) -> bool {
        let Some(index) = self.passages.iter().position(|p| p.id == id) else {
            return false;
        };
        let p = &mut self.passages[index];
        if p.page + 1 < p.pages {
            p.page += 1;
        } else {
            self.passages.remove(index);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn world_passages_require_private_delivery_bound_replace_expire_and_keep_explicit_pages() {
        let mut map = Map::default();
        let mut player = crate::Entity::new();
        player.id = 1;
        player.set_attribute("player", crate::Value::Bool(true));
        map.entities.push(player);
        let assets = Assets::default();
        let mut texts = WorldTexts::default();
        let message = |id| WorldText {
            anchor: TextAnchor::Entity(id),
            text: "東京\r\nمرحبا e\u{301}".into(),
        };
        texts.receive(&map, &assets, &TheTime::default(), &[(999, message(1))]);
        assert!(texts.passages().is_empty());
        for id in 1..=5 {
            texts.receive(&map, &assets, &TheTime::default(), &[(1, message(id))]);
        }
        assert_eq!(texts.passages().len(), 4);
        assert_eq!(texts.passages()[0].message.anchor, TextAnchor::Entity(2));
        let old = texts.passages()[3].id;
        texts.receive(&map, &assets, &TheTime::default(), &[(1, message(5))]);
        assert_eq!(texts.passages().len(), 4);
        assert_ne!(texts.passages()[3].id, old);
        let paged = texts.passages()[0].id;
        texts.set_page_count(paged, 2);
        texts.advance(Duration::from_secs(30));
        texts.expire(Some(map.id));
        assert_eq!(texts.passages().len(), 1);
        assert!(texts.activate(paged));
        assert_eq!(texts.passages()[0].page, 1);
        assert!(texts.activate(paged));
        assert!(texts.passages().is_empty());
        assert!(!texts.activate(paged));
        texts.receive(&map, &assets, &TheTime::default(), &[(1, message(1))]);
        texts.expire(Some(Uuid::new_v4()));
        assert!(texts.passages().is_empty());
    }
}
