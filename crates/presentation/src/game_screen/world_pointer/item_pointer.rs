//! UI-origin item gestures bind the displayed slot and original item address.
//! Continuations cannot activate whichever control happens to be under release.
use super::*;
use crate::game_screen::router::PointerCapture;
use rusterix::item_interaction::{self, ItemClick, ItemDestination, ItemRef, ItemSource};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ItemSlot {
    button: u32,
    rect: Rect,
    source: ItemSource,
    index: Option<usize>,
    slot: Option<String>,
    drag_drop: bool,
}
#[derive(Clone, Debug)]
pub(in crate::game_screen) struct ItemReceipt {
    slot: ItemSlot,
    binding: Binding,
    intent: Option<String>,
    point: [f32; 2],
}
pub(super) struct ItemCapture {
    slot: ItemSlot,
    binding: Binding,
    point: [f32; 2],
    intent: Option<String>,
    immediate: bool,
    dragging: bool,
    cancelled: bool,
}
impl ItemCapture {
    pub(super) fn cancel(&mut self) {
        self.cancelled = true;
    }
    pub(super) fn owner(&self) -> ScreenCapture {
        if self.cancelled {
            ScreenCapture::Cancelled
        } else {
            ScreenCapture::Button(self.slot.button)
        }
    }
}
impl GameScreenController {
    pub(super) fn item_slots(&self, map: &Map) -> Vec<ItemSlot> {
        screen_model::ordered_button_ids(&self.elements.button_widgets)
            .into_iter()
            .filter_map(|id| {
                let widget = &self.elements.button_widgets[&id];
                if screen_model::widget_name_is_hidden(&widget.name, &self.elements.widgets_to_hide)
                    || !visible_control_rect(&self.elements, widget.rect)
                {
                    return None;
                }
                let actor = screen_model::resolve_party_entity(map, widget.party.as_deref())?;
                let item = if let Some(index) = widget.inventory_index {
                    actor.inventory.get(index)?.as_ref()?
                } else {
                    actor.get_equipped_item(widget.equipped_slot.as_deref()?)?
                };
                Some(ItemSlot {
                    button: id,
                    rect: widget.rect,
                    source: ItemSource::Direct(ItemRef {
                        item: item.id,
                        owner: Some(actor.id),
                    }),
                    index: widget.inventory_index,
                    slot: widget.equipped_slot.clone(),
                    drag_drop: widget.drag_drop,
                })
            })
            .collect()
    }
    fn item_source_current(&self, world: &RuntimeView<'_>, capture: &ItemCapture) -> bool {
        !capture.cancelled
            && self.item_slot_current(
                world,
                capture.binding,
                &capture.slot,
                capture.intent.as_deref(),
            )
    }
    fn item_slot_current(
        &self,
        world: &RuntimeView<'_>,
        binding: Binding,
        slot: &ItemSlot,
        intent: Option<&str>,
    ) -> bool {
        self.ui_frame_matches(world)
            && !self.pointer_modal()
            && self.world.displayed.as_ref().is_some_and(|frame| {
                frame.binding == binding && !frame.modal && frame.items.contains(slot)
            })
            && world.map.is_some_and(|map| {
                self.item_slots(map).contains(slot) && slot.source.resolve(map).is_some()
            })
            && screen_model::selected_intent(
                &self.elements.activated_widgets,
                &self.elements.button_widgets,
                &self.intent,
            )
            .as_deref()
                == intent
    }
    fn item_source_visible(&self, point: [f32; 2], capture: &ItemCapture) -> bool {
        inside_frame(capture.binding, point)
            && rect(capture.slot.rect).contains(point)
            && self.world.displayed.as_ref().is_some_and(|frame| {
                front_control(&frame.controls, point)
                    == Some(Some(DropOwner::Button(capture.slot.button)))
            })
            && front_control(&self.pointer_controls(), point)
                == Some(Some(DropOwner::Button(capture.slot.button)))
    }
}
pub(super) fn receipt(
    screen: &GameScreenController,
    world: &RuntimeView<'_>,
    button: u32,
    point: Option<[f32; 2]>,
) -> Result<Option<Arc<ItemReceipt>>, ()> {
    let widget = screen.elements.button_widgets.get(&button).ok_or(())?;
    if widget.inventory_index.is_none() && widget.equipped_slot.is_none() {
        return Ok(None);
    }
    let current = world.map.and_then(|map| {
        screen
            .item_slots(map)
            .into_iter()
            .find(|slot| slot.button == button)
    });
    let displayed = screen
        .world
        .displayed
        .as_ref()
        .and_then(|frame| frame.items.iter().find(|slot| slot.button == button))
        .cloned();
    if current.is_none() && displayed.is_none() {
        return Ok(None);
    }
    if !screen.ui_frame_matches(world) || current != displayed {
        return Err(());
    }
    let slot = displayed.ok_or(())?;
    let binding = screen.world.displayed.as_ref().ok_or(())?.binding;
    let clipped = rect(slot.rect).intersection(Uv::new(
        0.,
        0.,
        binding.size.0 as f32,
        binding.size.1 as f32,
    ));
    let receipt = Arc::new(ItemReceipt {
        slot,
        binding,
        // A pointer commits the visible location originally pressed. Keyboard
        // activation uses the focused control's clipped center instead.
        point: point.unwrap_or([
            clipped.x + clipped.width / 2.,
            clipped.y + clipped.height / 2.,
        ]),
        intent: screen_model::selected_intent(
            &screen.elements.activated_widgets,
            &screen.elements.button_widgets,
            &screen.intent,
        ),
    });
    if valid_receipt(screen, world, &receipt) {
        Ok(Some(receipt))
    } else {
        Err(())
    }
}
pub(super) fn valid_receipt(
    screen: &GameScreenController,
    world: &RuntimeView<'_>,
    receipt: &ItemReceipt,
) -> bool {
    if !screen.item_slot_current(
        world,
        receipt.binding,
        &receipt.slot,
        receipt.intent.as_deref(),
    ) {
        return false;
    }
    let point = receipt.point;
    if !inside_frame(receipt.binding, point) || !rect(receipt.slot.rect).contains(point) {
        return false;
    }
    screen.world.displayed.as_ref().is_some_and(|frame| {
        front_control(&frame.controls, point) == Some(Some(DropOwner::Button(receipt.slot.button)))
    }) && front_control(&screen.pointer_controls(), point)
        == Some(Some(DropOwner::Button(receipt.slot.button)))
}
fn front_control(controls: &[UiControl], point: [f32; 2]) -> Option<Option<DropOwner>> {
    controls
        .iter()
        .rev()
        .find(|control| control.rect.contains(point))
        .map(|control| control.owner)
}
pub(super) fn begin(
    screen: &mut GameScreenController,
    world: &RuntimeView<'_>,
    button: u32,
    point: [f32; 2],
) -> Option<ScreenInputResponse> {
    let widget = screen.elements.button_widgets.get(&button)?;
    if !widget.drag_drop || widget.inventory_index.is_none() && widget.equipped_slot.is_none() {
        return None;
    }
    let current = world.map.and_then(|map| {
        screen
            .item_slots(map)
            .into_iter()
            .find(|slot| slot.button == button)
    });
    let displayed = screen
        .world
        .displayed
        .as_ref()
        .and_then(|frame| frame.items.iter().find(|slot| slot.button == button))
        .cloned();
    if current.is_none() && displayed.is_none() {
        return None;
    }
    screen.world.hover = None;
    screen.world.look = None;
    screen.hovered_item = None;
    screen.world.pointer = Some(point);
    if !screen.ui_frame_matches(world)
        || screen.pointer_modal()
        || current != displayed
        || displayed.is_none()
    {
        screen.pointer_capture = Some(PointerCapture::Cancelled);
        return Some(ScreenInputResponse {
            cancel_controls: true,
            ..ScreenInputResponse::handled(true)
        });
    }
    let slot = displayed.unwrap();
    let frame = screen.world.displayed.as_ref().unwrap();
    if front_control(&frame.controls, point) != Some(Some(DropOwner::Button(button))) {
        screen.pointer_capture = Some(PointerCapture::Cancelled);
        return Some(ScreenInputResponse {
            cancel_controls: true,
            ..ScreenInputResponse::handled(true)
        });
    }
    screen.world.ui_item = Some(ItemCapture {
        slot,
        binding: frame.binding,
        point,
        intent: screen_model::selected_intent(
            &screen.elements.activated_widgets,
            &screen.elements.button_widgets,
            &screen.intent,
        ),
        immediate: frame.immediate,
        dragging: false,
        cancelled: false,
    });
    screen.pointer_capture = Some(PointerCapture::Item(button));
    Some(ScreenInputResponse {
        cancel_controls: true,
        ..ScreenInputResponse::handled(true)
    })
}
pub(super) fn input(
    screen: &mut GameScreenController,
    world: &RuntimeView<'_>,
    views: Option<&SceneViews>,
    event: &InputEvent,
) -> Option<ScreenInputResponse> {
    screen.world.ui_item.as_ref()?;
    match event {
        InputEvent::Focus(false) => {
            screen.world.ui_item = None;
            return None;
        }
        InputEvent::PointerLeft => {
            screen.world.ui_item.as_mut()?.cancel();
            screen.world.pointer = None;
            return Some(ScreenInputResponse::handled(true));
        }
        InputEvent::Key {
            physical: PhysicalKey::Escape,
            pressed: true,
            ..
        } => {
            screen.world.ui_item.as_mut()?.cancel();
            screen.pointer_capture = Some(PointerCapture::Cancelled);
            screen.pressed_button = None;
            return Some(ScreenInputResponse::handled(true));
        }
        InputEvent::PointerMoved { position } | InputEvent::PointerButton { position, .. } => {
            screen.cursor = *position;
            screen.world.pointer = Some(*position);
            screen.world.hover = None;
            screen.world.look = None;
            screen.hovered_item = None;
            let valid = screen.item_source_current(world, screen.world.ui_item.as_ref().unwrap());
            let capture = screen.world.ui_item.as_mut().unwrap();
            capture.cancelled |= !valid;
            capture.dragging |= (position[0] - capture.point[0]).powi(2)
                + (position[1] - capture.point[1]).powi(2)
                >= 36.;
            if matches!(
                event,
                InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed: false,
                    ..
                }
            ) {
                let capture = screen.world.ui_item.take().unwrap();
                screen.pointer_capture = None;
                screen.pressed_button = None;
                screen.elements.release_button_state();
                screen.geometry = None;
                // Releasing transient button state must not reinterpret the
                // intent/slot that was validated before that UI cleanup.
                let effects = if valid && !capture.cancelled {
                    if capture.dragging {
                        screen
                            .release_item_drag(
                                world,
                                views,
                                capture.slot.source,
                                capture.binding,
                                *position,
                            )
                            .map(ScreenEffect::Action)
                            .into_iter()
                            .collect()
                    } else if screen.item_source_visible(*position, &capture) {
                        match item_interaction::click(
                            world.map.unwrap(),
                            capture.slot.source,
                            capture.intent.as_deref(),
                            0.,
                            capture.intent.is_none(),
                        ) {
                            Some(ItemClick::Action {
                                action,
                                uses_intent,
                            }) => {
                                if uses_intent && capture.immediate {
                                    screen.consume_targeting_intent();
                                }
                                vec![ScreenEffect::Action(action)]
                            }
                            Some(ItemClick::ToggleContainer(address)) => address
                                .owner
                                .map(|owner| {
                                    vec![ScreenEffect::ToggleContainer {
                                        region: capture.binding.region,
                                        item: address.item,
                                        owner,
                                        anchor: rect(capture.slot.rect),
                                    }]
                                })
                                .unwrap_or_default(),
                            None => Vec::new(),
                        }
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                return Some(ScreenInputResponse {
                    effects,
                    ..ScreenInputResponse::handled(true)
                });
            }
            Some(ScreenInputResponse::handled(true))
        }
        _ => None,
    }
}
pub(super) fn dragged<'a>(
    screen: &GameScreenController,
    world: &RuntimeView<'a>,
) -> Option<&'a rusterix::Item> {
    let capture = screen.world.ui_item.as_ref()?;
    if !capture.dragging || !screen.item_source_current(world, capture) {
        return None;
    }
    capture.slot.source.resolve(world.map?)
}
impl GameScreenController {
    pub(super) fn release_item_drag(
        &self,
        world: &RuntimeView<'_>,
        views: Option<&SceneViews>,
        source: ItemSource,
        binding: Binding,
        point: [f32; 2],
    ) -> Option<EntityAction> {
        if !self.ui_frame_matches(world) || self.pointer_modal() {
            return None;
        }
        let frame = self.world.displayed.as_ref()?;
        if frame.binding != binding || frame.modal || !inside_frame(binding, point) {
            return None;
        }
        let map = world.map?;
        let front = front_control(&frame.controls, point);
        let current = front_control(&self.pointer_controls(), point);
        if let Some(Some(owner)) = front {
            if current != front {
                return None;
            }
            let target = frame.drops.iter().find(|target| target.source == owner)?;
            let live = self
                .pointer_drops(map)
                .into_iter()
                .find(|target| target.source == owner)?;
            if live != *target {
                return None;
            }
            return item_interaction::transfer(map, source, target.destination.clone());
        }
        if front.is_some() || current.is_some() {
            return None;
        }
        if let Query::Hit { hit, .. } = self.query_world(world, views, point, false) {
            let world = hit.world?;
            return item_interaction::transfer(
                map,
                source,
                ItemDestination::World(Vec2::new(world.x, world.z)),
            );
        }
        None
    }
}
