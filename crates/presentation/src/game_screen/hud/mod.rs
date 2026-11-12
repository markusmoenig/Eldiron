//! Retained floating HUDs have identities independent of authored screen widgets.
mod input;
pub(in crate::game_screen) mod model;
mod paint;
mod style;
use super::*;
use model::{HudState, PanelFrame};
use rusterix::item_interaction::{ItemRef, ItemSource};
use theframework::text::TextLayout;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HudPanelId {
    Container { source: ItemRef, incarnation: u64 },
    Inventory { entity: u32, incarnation: u64 },
    Equipment { entity: u32, incarnation: u64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HudPart {
    Body,
    Title,
    Close,
    Slot(usize),
    Category(usize),
    Sort,
    SortOption(usize),
    SortPopup,
    PreviousPage,
    NextPage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HudControlId {
    pub panel: HudPanelId,
    pub part: HudPart,
}
struct PreparedHud {
    frame: PanelFrame,
    title_key: (String, String, u32, GameFontVersion),
    labels: Vec<(model::PanelLabel, Arc<TextLayout>)>,
    avatar: Option<Arc<PreparedContent>>,
    title: Arc<TextLayout>,
    items: FxHashMap<usize, Arc<PreparedContent>>,
    quantities: FxHashMap<usize, (i32, Arc<TextLayout>)>,
}
#[derive(Default)]
pub(super) struct Hud {
    state: HudState,
    prepared: Vec<PreparedHud>,
    capture: Option<input::HudCapture>,
    focused: Option<HudControlId>,
    pointer_present: bool,
}
impl Hud {
    pub(super) fn select_item(&mut self, id: HudControlId, item: u32) {
        self.state.select_item(id, item);
    }
    pub(super) fn popup_open(&self) -> bool {
        self.state.popup_open()
    }

    pub(super) fn frames(
        &self,
        world: &RuntimeView<'_>,
        elements: &ScreenElements,
    ) -> Vec<PanelFrame> {
        self.state.frames(world, elements)
    }
    pub(super) fn bind(&mut self, world: &RuntimeView<'_>) {
        self.state.bind(world);
        if world.state == rusterix::ServerState::Off {
            self.prepared.clear();
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
        elements: &ScreenElements,
        fonts: &mut GameFonts,
        sources: &mut ContentSources,
        revision: ContentRevision,
        animation_frame: usize,
    ) -> Result<(), String> {
        self.bind(world);
        let frames = self.frames(world, elements);
        if self
            .focused
            .is_some_and(|id| !frames.iter().any(|frame| id.panel == frame.id))
        {
            self.focused = None;
        }
        let old_panels = std::mem::take(&mut self.prepared);
        for frame in frames {
            self.state.retain_placement(&frame);
            let old = old_panels.iter().find(|old| old.frame.id == frame.id);
            let title_key = (
                frame.title.clone(),
                frame.style.font.clone(),
                frame.style.title_size.to_bits(),
                fonts.version(),
            );
            let title = old
                .filter(|old| old.title_key == title_key)
                .map(|old| old.title.clone())
                .unwrap_or_else(|| {
                    Arc::new(fonts.text.layout(
                        &frame.title,
                        &frame.style.font,
                        frame.style.title_size,
                        None,
                    ))
                });
            let mut items = FxHashMap::default();
            let mut quantities = FxHashMap::default();
            for slot in &frame.slots {
                if let Some(item) = slot
                    .source
                    .and_then(|source| world.map.and_then(|map| source.resolve(map)))
                {
                    let quantity = item.stack_quantity();
                    if quantity > 1 {
                        let layout = old
                            .filter(|old| {
                                old.title_key.3 == fonts.version()
                                    && old.frame.style.font == frame.style.font
                                    && old.frame.style.quantity_size == frame.style.quantity_size
                            })
                            .and_then(|old| old.quantities.get(&slot.index))
                            .filter(|(old, _)| *old == quantity)
                            .map(|(_, layout)| layout.clone())
                            .unwrap_or_else(|| {
                                Arc::new(fonts.text.layout(
                                    &quantity.to_string(),
                                    &frame.style.font,
                                    frame.style.quantity_size,
                                    None,
                                ))
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
            let labels = frame
                .labels
                .iter()
                .chain(frame.buttons.iter().map(|button| &button.label))
                .map(|label| {
                    let layout = old
                        .filter(|old| {
                            old.title_key.3 == fonts.version()
                                && old.frame.style.font == frame.style.font
                        })
                        .and_then(|old| {
                            old.labels.iter().find(|(spec, _)| {
                                spec.text == label.text && spec.size == label.size
                            })
                        })
                        .map(|(_, layout)| layout.clone())
                        .unwrap_or_else(|| {
                            Arc::new(fonts.text.layout(
                                &label.text,
                                &frame.style.font,
                                label.size,
                                None,
                            ))
                        });
                    (label.clone(), layout)
                })
                .collect();
            let avatar = if let Some((entity, _)) = frame.avatar {
                if let Some(actor) = world
                    .map
                    .and_then(|map| map.entities.iter().find(|actor| actor.id == entity))
                {
                    sources.prepare(
                        ContentRecipe::Avatar {
                            entity: actor,
                            settings: rusterix::client::content::AvatarSourceSettings {
                                avatar: None,
                                animation: None,
                                direction: rusterix::AvatarDirection::Front,
                                frame: 0,
                                weapons: true,
                            },
                        },
                        world.assets,
                        revision,
                    )?
                } else {
                    None
                }
            } else {
                None
            };
            self.prepared.push(PreparedHud {
                frame,
                title_key,
                title,
                labels,
                avatar,
                items,
                quantities,
            });
        }
        Ok(())
    }
    pub(super) fn validate_prepared(
        &self,
        world: &RuntimeView<'_>,
        elements: &ScreenElements,
    ) -> Result<(), String> {
        if self
            .prepared
            .iter()
            .map(|prepared| &prepared.frame)
            .collect::<Vec<_>>()
            != self.frames(world, elements).iter().collect::<Vec<_>>()
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
        self.hud.bind(world);
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
        if let ScreenEffect::Command(rusterix::client::command::ClientCommandBinding::Ui(command)) =
            effect
        {
            if self.hud.state.toggle(command) {
                self.choice_controller
                    .cancel(&mut self.elements.choice_widgets);
                self.hud.clear_focus();
                return true;
            }
        }
        false
    }
}
