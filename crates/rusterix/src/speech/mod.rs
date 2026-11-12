//! CPU speech ownership. Only explicit Say batches create utterances; renderer
//! preparation observes simulation time without advancing it or draining events.
pub mod style;
use crate::{Map, server::Say, world_text::TextAnchor};
use std::sync::Arc;
use theframework_core::{TheTime, Uuid};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpeechTarget {
    Entity(u32),
    Item(u32),
}
impl SpeechTarget {
    pub fn anchor(self) -> TextAnchor {
        match self {
            Self::Entity(id) => TextAnchor::Entity(id),
            Self::Item(id) => TextAnchor::Item(id),
        }
    }
    fn exists(self, map: &Map) -> bool {
        match self {
            Self::Entity(id) => map.entities.iter().any(|e| e.id == id),
            Self::Item(id) => map.items.iter().any(|i| i.id == id),
        }
    }
}
pub struct Utterance {
    pub id: Uuid,
    pub target: SpeechTarget,
    pub text: Arc<str>,
    pub category: String,
    started: i64,
}
#[derive(Default)]
pub struct SpeechMessages {
    region: Option<Uuid>,
    last: Option<i64>,
    seconds: i64,
    messages: Vec<Utterance>,
}
impl SpeechMessages {
    pub fn messages(&self) -> &[Utterance] {
        &self.messages
    }
    pub fn clear(&mut self) {
        self.messages.clear();
    }
    /// Unwrap midnight. An explicit backwards time edit resets old speech;
    /// repeated frame preparation and paused clocks cannot extend its lifetime.
    pub fn synchronize(&mut self, map: Option<&Map>, time: &TheTime) {
        let region = map.map(|m| m.id);
        let now = time.hours as i64 * 3600 + time.minutes as i64 * 60 + time.seconds as i64;
        if self.region != region {
            self.messages.clear();
            self.region = region;
            self.last = None;
        }
        if let Some(last) = self.last {
            let delta = now - last;
            if delta < -43200 {
                self.seconds = self.seconds.saturating_add(delta + 86400);
            } else if delta < 0 {
                self.messages.clear();
                self.seconds = now;
            } else {
                self.seconds = self.seconds.saturating_add(delta);
            }
        } else {
            self.seconds = now;
        }
        self.last = Some(now);
        self.messages
            .retain(|m| map.is_some_and(|map| m.target.exists(map)));
    }
    pub fn receive(&mut self, map: &Map, time: &TheTime, messages: &[Say]) {
        self.synchronize(Some(map), time);
        for (entity, item, text, category) in messages {
            let target = item
                .map(SpeechTarget::Item)
                .or_else(|| entity.map(SpeechTarget::Entity));
            let Some(target) = target.filter(|t| t.exists(map)) else {
                continue;
            };
            if text.trim().is_empty() {
                continue;
            }
            self.messages.retain(|m| m.target != target);
            self.messages.push(Utterance {
                id: Uuid::new_v4(),
                target,
                text: Arc::from(text.as_str()),
                category: category.clone(),
                started: self.seconds,
            });
        }
    }
    pub fn is_live(&self, message: &Utterance, duration_ticks: i64, ticks_per_minute: u32) -> bool {
        let tick = |seconds: i64| seconds as i128 * ticks_per_minute.max(1) as i128 / 60;
        tick(self.seconds) - tick(message.started) < duration_ticks.max(1) as i128
    }
    pub fn expire(&mut self, duration_ticks: i64, ticks_per_minute: u32) {
        let seconds = self.seconds;
        let tick = |seconds: i64| seconds as i128 * ticks_per_minute.max(1) as i128 / 60;
        self.messages
            .retain(|m| tick(seconds) - tick(m.started) < duration_ticks.max(1) as i128);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn map() -> Map {
        let mut map = Map::default();
        let mut entity = crate::Entity::new();
        entity.id = 7;
        map.entities.push(entity);
        let mut item = crate::Item::new();
        item.id = 7;
        map.items.push(item);
        map
    }
    #[test]
    fn speech_is_addressed_typed_replaced_once_and_expired_by_simulation_time() {
        let mut state = SpeechMessages::default();
        let map = map();
        let time = TheTime::new_time(12, 0).unwrap();
        let text = "東京\r\nمرحبا e\u{301}";
        state.receive(
            &map,
            &time,
            &[
                (Some(7), None, text.into(), "greeting".into()),
                (None, Some(7), "Item".into(), "".into()),
                (Some(99), None, "unknown".into(), "".into()),
            ],
        );
        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.messages()[0].text.as_ref(), text);
        let item = state.messages()[1].id;
        for _ in 0..100 {
            state.synchronize(Some(&map), &time);
            state.expire(4, 4);
        }
        assert_eq!(state.messages().len(), 2);
        state.receive(
            &map,
            &time,
            &[(Some(7), None, "Replacement".into(), "".into())],
        );
        assert_eq!(state.messages().len(), 2);
        assert_eq!(state.messages()[0].id, item);
        state.synchronize(Some(&map), &TheTime::new_time(12, 1).unwrap());
        state.expire(4, 4);
        assert!(state.messages().is_empty());
    }
    #[test]
    fn speech_unwraps_midnight_and_revokes_removed_targets_other_regions_and_clock_restores() {
        let mut state = SpeechMessages::default();
        let mut map = map();
        let message = [(Some(7), None, "Hi".into(), "".into())];
        state.receive(
            &map,
            &TheTime {
                hours: 23,
                minutes: 59,
                seconds: 45,
            },
            &message,
        );
        state.synchronize(Some(&map), &TheTime::default());
        state.expire(4, 4);
        assert_eq!(state.messages().len(), 1);
        state.synchronize(
            Some(&map),
            &TheTime {
                hours: 0,
                minutes: 0,
                seconds: 45,
            },
        );
        state.expire(4, 4);
        assert!(state.messages().is_empty());
        let time = TheTime::new_time(12, 0).unwrap();
        state.receive(&map, &time, &message);
        map.entities.clear();
        state.synchronize(Some(&map), &time);
        assert!(state.messages().is_empty());
        map = self::map();
        state.receive(&map, &time, &message);
        map.id = Uuid::new_v4();
        state.synchronize(Some(&map), &time);
        assert!(state.messages().is_empty());
        state.receive(&map, &time, &message);
        state.synchronize(Some(&map), &TheTime::new_time(11, 59).unwrap());
        assert!(state.messages().is_empty());
    }
}
