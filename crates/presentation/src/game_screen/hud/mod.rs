//! Retained floating HUDs have identities independent of authored screen widgets.
mod input;
pub(in crate::game_screen) mod model;
mod paint;
use super::*;
use model::{ContainerFrame, HudState};
use rusterix::item_interaction::{ItemRef, ItemSource};
use theframework::text::TextLayout;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HudPanelId {
    Container { source: ItemRef, incarnation: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HudPart {
    Body,
    Title,
    Close,
    Slot(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HudControlId {
    pub panel: HudPanelId,
    pub part: HudPart,
}
struct PreparedHud {
    frame: ContainerFrame,
    title_key: (String, GameFontVersion),
    title: Arc<TextLayout>,
    items: FxHashMap<usize, Arc<PreparedContent>>,
    quantities: FxHashMap<usize, (i32, Arc<TextLayout>)>,
}
#[derive(Default)]
pub(super) struct Hud {
    state: HudState,
    prepared: Option<PreparedHud>,
    capture: Option<input::HudCapture>,
    focused: Option<HudControlId>,
    pointer_present: bool,
}
impl Hud {
    pub(super) fn current(
        &self,
        world: &RuntimeView<'_>,
        size: scenevm::prelude::Vec2<i32>,
    ) -> Option<ContainerFrame> {
        self.state.layout(world, size)
    }
    pub(super) fn bind(&mut self, world: &RuntimeView<'_>) {
        self.state.bind(world);
        if world.state == rusterix::ServerState::Off {
            self.prepared = None;
            self.focused = None;
        }
    }
    pub(super) fn receive(
        &mut self,
        world: &RuntimeView<'_>,
        events: &RegionEvents,
        size: scenevm::prelude::Vec2<i32>,
    ) {
        self.bind(world);
        for &(item, owner) in &events.containers {
            self.state
                .open(world, ItemRef { item, owner }, None, size, false);
        }
    }
    pub(super) fn prepare(
        &mut self,
        world: &RuntimeView<'_>,
        size: scenevm::prelude::Vec2<i32>,
        fonts: &mut GameFonts,
        sources: &mut ContentSources,
        revision: ContentRevision,
        animation_frame: usize,
    ) -> Result<(), String> {
        self.bind(world);
        let Some(frame) = self.current(world, size) else {
            self.prepared = None;
            self.focused = None;
            return Ok(());
        };
        self.state.retain_placement(&frame);
        if self.focused.is_some_and(|id| id.panel != frame.id) {
            self.focused = None;
        }
        let title_key = (frame.title.clone(), fonts.version());
        let title = if let Some(old) = &self.prepared
            && old.title_key == title_key
        {
            old.title.clone()
        } else {
            Arc::new(fonts.text.layout(&frame.title, "Roboto", 14., None))
        };
        let mut items = FxHashMap::default();
        let mut quantities = FxHashMap::default();
        for slot in &frame.slots {
            if let Some(item) = slot.source.and_then(|source| source.resolve(world.map?)) {
                let quantity = item.stack_quantity();
                if quantity > 1 {
                    let layout = self
                        .prepared
                        .as_ref()
                        .filter(|old| old.title_key.1 == fonts.version())
                        .and_then(|old| old.quantities.get(&slot.index))
                        .filter(|(old, _)| *old == quantity)
                        .map(|(_, layout)| layout.clone())
                        .unwrap_or_else(|| {
                            Arc::new(
                                fonts
                                    .text
                                    .layout(&quantity.to_string(), "Roboto", 10., None),
                            )
                        });
                    quantities.insert(slot.index, (quantity, layout));
                }
                if let Some(source) = sources.prepare(
                    ContentRecipe::Item {
                        item,
                        frame: animation_frame,
                    },
                    world.assets,
                    revision,
                )? {
                    items.insert(slot.index, source);
                }
            }
        }
        self.prepared = Some(PreparedHud {
            frame,
            title_key,
            title,
            items,
            quantities,
        });
        Ok(())
    }
    pub(super) fn validate_prepared(
        &self,
        world: &RuntimeView<'_>,
        size: scenevm::prelude::Vec2<i32>,
    ) -> Result<(), String> {
        if self.prepared.as_ref().map(|prepared| &prepared.frame)
            != self.current(world, size).as_ref()
        {
            return Err("Prepare HUD geometry after changing its content or layout".into());
        }
        Ok(())
    }
    pub(super) fn after_navigation(&mut self) {
        self.focused = None;
        self.capture = None;
        self.pointer_present = false;
    }
    pub(super) fn clear_focus(&mut self) {
        self.focused = None;
        if let Some(capture) = &mut self.capture {
            capture.cancelled = true;
        }
    }
    pub(super) fn owner(&self) -> Option<ScreenCapture> {
        self.capture
            .as_ref()
            .map(|capture| {
                if capture.cancelled {
                    ScreenCapture::Cancelled
                } else {
                    ScreenCapture::Hud(capture.control)
                }
            })
            .or_else(|| self.focused.map(ScreenCapture::Hud))
    }
}
impl GameScreenController {
    /// Apply presentation-owned effects after semantic input emits them. No
    /// server queue is drained, and no authoritative inventory is mutated here.
    pub fn handle_hud_effect(&mut self, world: &RuntimeView<'_>, effect: &ScreenEffect) -> bool {
        self.bind_runtime(world);
        if let ScreenEffect::ToggleContainer {
            region,
            item,
            owner,
            anchor,
        } = effect
        {
            if world.map.is_some_and(|map| map.id == *region) {
                self.hud.state.open(
                    world,
                    ItemRef {
                        item: *item,
                        owner: Some(*owner),
                    },
                    Some(Rect::new(anchor.x, anchor.y, anchor.width, anchor.height)),
                    self.elements.layout.viewport,
                    true,
                );
            }
            return true;
        }
        false
    }
}
