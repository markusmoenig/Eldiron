//! Logical floating-container state and viewport geometry. No image buffers.
use super::*;
use rusterix::{ServerState, client::rules_ui, world_interaction::item_is_container};
use scenevm::prelude::Vec2;

struct OpenContainer {
    source: ItemRef,
    incarnation: u64,
    position: Vec2<i32>,
    scroll: [f64; 2],
}
#[derive(Default)]
pub(super) struct HudState {
    scope: Option<(Uuid, Uuid)>,
    positions: FxHashMap<ItemRef, Vec2<i32>>,
    open: Option<OpenContainer>,
    serial: u64,
}
#[derive(Clone, PartialEq)]
pub(in crate::game_screen) struct ContainerFrame {
    pub id: HudPanelId,
    pub rect: Rect,
    pub title_bar: Option<Rect>,
    pub close: Option<Rect>,
    pub title_rect: Option<Rect>,
    pub title: String,
    pub clip: Rect,
    pub template: rules_ui::ContainerUiTemplate,
    pub slots: Vec<ContainerSlot>,
    pub scroll: [f64; 2],
    pub max_scroll: [f64; 2],
    pub position: Vec2<i32>,
}
#[derive(Clone, Debug, PartialEq)]
pub(in crate::game_screen) struct ContainerSlot {
    pub index: usize,
    pub rect: Rect,
    pub hit: Rect,
    pub source: Option<ItemSource>,
}
impl ContainerFrame {
    pub fn control(&self, part: HudPart) -> HudControlId {
        HudControlId {
            panel: self.id,
            part,
        }
    }
    pub fn controls(&self) -> Vec<(HudControlId, Rect)> {
        let mut controls = vec![(self.control(HudPart::Body), self.rect)];
        controls.extend(
            self.slots
                .iter()
                .map(|slot| (self.control(HudPart::Slot(slot.index)), slot.hit)),
        );
        if let Some(title) = self.title_bar {
            controls.push((self.control(HudPart::Title), title));
        }
        if let Some(close) = self.close {
            controls.push((self.control(HudPart::Close), close));
        }
        controls
            .into_iter()
            .filter_map(|(id, bounds)| {
                let hit = crate::screen::rect(bounds).intersection(crate::screen::rect(self.rect));
                (hit.width > 0. && hit.height > 0.)
                    .then(|| (id, Rect::new(hit.x, hit.y, hit.width, hit.height)))
            })
            .collect()
    }
    pub fn source(&self) -> ItemRef {
        match self.id {
            HudPanelId::Container { source, .. } => source,
        }
    }
}
impl HudState {
    pub fn bind(&mut self, world: &RuntimeView<'_>) {
        let scope = world.map.map(|map| (world.session_id, map.id));
        if self.scope != scope {
            self.close();
            if self.scope.map(|s| s.0) != scope.map(|s| s.0) {
                self.positions.clear();
            }
            self.scope = scope;
        }
        if world.state == ServerState::Off
            || self.open.as_ref().is_some_and(|open| {
                world
                    .map
                    .and_then(|map| open.source.resolve(map))
                    .is_none_or(|item| !item_is_container(item))
            })
        {
            self.close();
        }
    }
    pub fn close(&mut self) {
        if let Some(open) = self.open.take() {
            self.positions.insert(open.source, open.position);
        }
    }
    pub fn open(
        &mut self,
        world: &RuntimeView<'_>,
        source: ItemRef,
        anchor: Option<Rect>,
        size: Vec2<i32>,
        toggle: bool,
    ) {
        self.bind(world);
        if world.state == ServerState::Off
            || !world
                .map
                .and_then(|m| source.resolve(m))
                .is_some_and(item_is_container)
        {
            return;
        }
        if toggle && self.open.as_ref().is_some_and(|open| open.source == source) {
            self.close();
            return;
        }
        self.close();
        let position = self.positions.get(&source).copied().unwrap_or_else(|| {
            anchor
                .map(|r| Vec2::new((r.x + r.width + 12.).round() as i32, r.y.round() as i32))
                .unwrap_or(Vec2::new(size.x / 2 - 96, size.y / 2 - 64))
        });
        self.serial = self
            .serial
            .checked_add(1)
            .expect("HUD incarnation exhausted");
        self.open = Some(OpenContainer {
            source,
            incarnation: self.serial,
            position,
            scroll: [0.; 2],
        });
    }
    pub fn layout(&self, world: &RuntimeView<'_>, size: Vec2<i32>) -> Option<ContainerFrame> {
        let map = world.map?;
        if self.scope != Some((world.session_id, map.id)) || world.state == ServerState::Off {
            return None;
        }
        let open = self.open.as_ref()?;
        let item = open
            .source
            .resolve(map)
            .filter(|item| item_is_container(item))?;
        if size.x < 8 || size.y < 8 {
            return None;
        }
        let template = rules_ui::container_template_for_item(world.assets, item);
        let geometry = screen_model::ContainerGeometry::new(
            open.position,
            &template,
            screen_model::container_slot_count(item),
            size,
        );
        let mut bounds = geometry.rect;
        bounds.width = bounds.width.min((size.x - 4) as f32);
        bounds.height = bounds.height.min((size.y - 4) as f32);
        let padding = (template.padding.max(0) as f32)
            .min(bounds.width * 0.5)
            .min(bounds.height * 0.5);
        let title_height = if template.title {
            26f32.min(bounds.height)
        } else {
            0.
        };
        let clip = Rect::new(
            bounds.x + padding,
            bounds.y + padding + title_height,
            (bounds.width - padding * 2.).max(0.),
            (bounds.height - padding * 2. - title_height).max(0.),
        );
        let content = geometry.content_size();
        let max_scroll = [
            (content[0] - clip.width as f64).max(0.),
            (content[1] - clip.height as f64).max(0.),
        ];
        let scroll = [
            open.scroll[0].clamp(0., max_scroll[0]),
            open.scroll[1].clamp(0., max_scroll[1]),
        ];
        let slots = geometry
            .visible_slots_offset(clip, scroll)
            .filter_map(|(index, rect)| {
                let hit = crate::screen::rect(rect).intersection(crate::screen::rect(clip));
                if hit.width <= 0. || hit.height <= 0. {
                    return None;
                }
                Some(ContainerSlot {
                    index,
                    rect,
                    hit: Rect::new(hit.x, hit.y, hit.width, hit.height),
                    source: item
                        .container
                        .as_ref()
                        .and_then(|items| items.get(index))
                        .map(|item| ItemSource::Container {
                            container: open.source,
                            item: item.id,
                        }),
                })
            })
            .collect();
        Some(ContainerFrame {
            id: HudPanelId::Container {
                source: open.source,
                incarnation: open.incarnation,
            },
            rect: bounds,
            title_bar: template
                .title
                .then(|| Rect::new(bounds.x, bounds.y, bounds.width, title_height)),
            close: template.title.then(|| {
                Rect::new(
                    bounds.x + (bounds.width - 27.).max(0.),
                    bounds.y + 3.,
                    22f32.min(bounds.width),
                    20f32.min(bounds.height),
                )
            }),
            title_rect: template.title.then(|| {
                Rect::new(
                    bounds.x + padding,
                    bounds.y + 3.,
                    (bounds.width - padding * 2. - 30.).max(1.),
                    20.,
                )
            }),
            title: item
                .attributes
                .get_str("name")
                .unwrap_or("Container")
                .to_string(),
            clip,
            template,
            slots,
            scroll,
            max_scroll,
            position: Vec2::new(bounds.x.round() as i32, bounds.y.round() as i32),
        })
    }
    pub fn retain_placement(&mut self, frame: &ContainerFrame) {
        if let Some(open) = self.open.as_mut().filter(|open| {
            frame.id
                == HudPanelId::Container {
                    source: open.source,
                    incarnation: open.incarnation,
                }
        }) {
            open.position = frame.position;
            open.scroll = frame.scroll;
        }
    }
    pub fn move_to(&mut self, id: HudPanelId, position: Vec2<i32>, size: Vec2<i32>) {
        if let Some(open) = self.open.as_mut().filter(|open| {
            id == HudPanelId::Container {
                source: open.source,
                incarnation: open.incarnation,
            }
        }) {
            open.position = Vec2::new(
                position.x.clamp(2, (size.x - 24).max(2)),
                position.y.clamp(2, (size.y - 24).max(2)),
            );
        }
    }
    pub fn scroll(&mut self, frame: &ContainerFrame, delta: [f32; 2], precise: bool) {
        if !delta.into_iter().all(f32::is_finite) {
            return;
        }
        if let Some(open) = &mut self.open {
            let scale = if precise {
                1.
            } else {
                frame.template.slot_size.max(8) as f64 + frame.template.gap.max(0) as f64
            };
            for axis in 0..2 {
                open.scroll[axis] = (frame.scroll[axis] - delta[axis] as f64 * scale)
                    .clamp(0., frame.max_scroll[axis]);
            }
        }
    }
}
