//! Retained pointer ownership against acknowledged scene frames. CPU target
//! positions and transforms survive preparation of a newer, unpresented frame.
use super::*;
use crate::{ViewImage, ViewPick};
use rusterix::{
    EntityAction, PlayerCamera, ServerState,
    world_interaction::{self, CellTargets, WorldClick, WorldHit},
};
use scenevm::prelude::{Vec2, Vec3};
use theframework::platform::{InputEvent, PhysicalKey, PointerButton};
mod item_pointer;
pub(super) use item_pointer::{ItemControl, ItemReceipt};
mod mouse_look;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Binding {
    session: Uuid,
    region: Option<Uuid>,
    active: bool,
    revision: ContentRevision,
    size: (i32, i32),
}
impl Binding {
    fn new(world: &RuntimeView<'_>, revision: ContentRevision, size: Vec2<i32>) -> Option<Self> {
        if size.x <= 0 || size.y <= 0 {
            return None;
        }
        Some(Self {
            session: world.session_id,
            region: world.map.map(|map| map.id),
            active: world.state != ServerState::Off,
            revision,
            size: (size.x, size.y),
        })
    }
    fn same_source(self, other: Self) -> bool {
        self.session == other.session
            && self.active == other.active
            && self.region == other.region
            && self.revision == other.revision
    }
}
enum Projection {
    Flat {
        origin: Vec2<f32>,
        grid: f32,
        logical: [f32; 2],
    },
    Ray(ViewImage),
}
struct ViewFrame {
    key: Uuid,
    rect: Rect,
    camera: PlayerCamera,
    source_size: [f32; 2],
    projection: Projection,
    look: Option<mouse_look::LookSettings>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum DropOwner {
    Button(u32),
    Profile(Uuid),
    Hud(super::hud::HudControlId),
}
#[derive(Clone, PartialEq)]
struct DropTarget {
    source: DropOwner,
    rect: Rect,
    destination: rusterix::item_interaction::ItemDestination,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct CommandTarget {
    pub button: u32,
    pub slot: String,
    pub rect: Rect,
}
struct Frame {
    binding: Binding,
    views: Vec<ViewFrame>,
    cells: Option<Arc<CellTargets>>,
    occluders: Vec<Uv>,
    drops: Vec<DropTarget>,
    modal: bool,
    author_modal: bool,
    immediate: bool,
    items: Vec<item_pointer::ItemSlot>,
    controls: Vec<UiControl>,
    commands: Vec<CommandTarget>,
}
#[derive(Clone)]
struct UiControl {
    action: Option<super::hud::model::HudAction>,
    owner: Option<DropOwner>,
    rect: Uv,
}
struct Capture {
    view: Uuid,
    binding: Binding,
    point: [f32; 2],
    hit: WorldHit,
    item: Option<u32>,
    dragging: bool,
    cancelled: bool,
    rect: Rect,
    camera: PlayerCamera,
}
pub(super) struct WorldPointer {
    bound: Option<Binding>,
    staged: Option<Frame>,
    displayed: Option<Frame>,
    hover: Option<WorldHit>,
    capture: Option<Capture>,
    focused: bool,
    pointer: Option<[f32; 2]>,
    look: Option<mouse_look::LookCapture>,
    ui_item: Option<item_pointer::ItemCapture>,
}
impl Default for WorldPointer {
    fn default() -> Self {
        Self {
            bound: None,
            staged: None,
            displayed: None,
            hover: None,
            capture: None,
            focused: true,
            pointer: None,
            look: None,
            ui_item: None,
        }
    }
}
impl WorldPointer {
    fn invalidate(&mut self) {
        if let Some(item) = &mut self.ui_item {
            item.cancel();
        }
        self.look = None;
        self.staged = None;
        self.displayed = None;
        self.hover = None;
        if let Some(capture) = &mut self.capture {
            capture.cancelled = true;
        }
    }
    fn cancel(&mut self) {
        if let Some(item) = &mut self.ui_item {
            item.cancel();
        }
        self.cancel_world();
    }
    fn cancel_world(&mut self) {
        self.look = None;
        self.hover = None;
        if let Some(capture) = &mut self.capture {
            capture.cancelled = true;
        }
    }
    pub(super) fn sync(
        &mut self,
        world: &RuntimeView<'_>,
        revision: ContentRevision,
        size: Vec2<i32>,
    ) -> bool {
        let binding = Binding::new(world, revision, size);
        let source_changed = match (self.bound, binding) {
            (Some(old), Some(new)) => !old.same_source(new),
            (None, None) => false,
            _ => true,
        };
        if self.bound != binding {
            self.invalidate();
        }
        if world.state != ServerState::Running {
            self.cancel_world();
        }
        self.bound = binding;
        source_changed
    }
    pub(super) fn push_view(
        &mut self,
        key: Uuid,
        widget: &rusterix::client::widget::game::GameWidget,
        output: ViewImage,
        map: &Map,
    ) -> Result<(), String> {
        let Some(frame) = &mut self.staged else {
            return Ok(());
        };
        let projection = if screen_model::is_2d_camera(&widget.camera) {
            // Isometric preparation is explicitly rejected by its pending GPU
            // sprite producer before this receipt can be published.
            if widget.camera.is_isometric_2d() {
                return Err("isometric input needs the GPU sprite producer's frame receipt".into());
            }
            frame
                .cells
                .get_or_insert_with(|| Arc::new(CellTargets::capture_visible(map)));
            let logical = widget.scene_surface_size();
            Projection::Flat {
                origin: widget.top_left,
                grid: widget.grid_size,
                logical: [logical.0 as f32, logical.1 as f32],
            }
        } else {
            Projection::Ray(output)
        };
        frame.views.push(ViewFrame {
            key,
            rect: widget.rect,
            camera: widget.camera.clone(),
            source_size: [
                output.descriptor.width as f32,
                output.descriptor.height as f32,
            ],
            projection,
            look: mouse_look::LookSettings::from_widget(widget),
        });
        Ok(())
    }
    fn owner(&self) -> Option<ScreenCapture> {
        if let Some(item) = &self.ui_item {
            return Some(item.owner());
        }
        self.capture.as_ref().map(|capture| {
            if capture.cancelled {
                ScreenCapture::Cancelled
            } else {
                ScreenCapture::World(capture.view)
            }
        })
    }
}
fn inside_frame(binding: Binding, point: [f32; 2]) -> bool {
    point.iter().all(|v| v.is_finite())
        && point[0] >= 0.
        && point[1] >= 0.
        && point[0] < binding.size.0 as f32
        && point[1] < binding.size.1 as f32
}
enum Query {
    Outside,
    Unavailable,
    Hit { view: Uuid, hit: WorldHit },
}
impl GameScreenController {
    pub(super) fn item_receipt(
        &self,
        world: &RuntimeView<'_>,
        button: u32,
        point: Option<[f32; 2]>,
    ) -> Result<Option<Arc<ItemReceipt>>, ()> {
        item_pointer::receipt(self, world, button, point)
    }
    pub(super) fn item_receipt_valid(
        &self,
        world: &RuntimeView<'_>,
        receipt: &ItemReceipt,
    ) -> bool {
        item_pointer::valid_receipt(self, world, receipt)
    }
    pub(super) fn begin_item_pointer(
        &mut self,
        world: &RuntimeView<'_>,
        button: u32,
        point: [f32; 2],
    ) -> Option<ScreenInputResponse> {
        let widget = self.elements.button_widgets.get(&button)?;
        if !widget.drag_drop || widget.inventory_index.is_none() && widget.equipped_slot.is_none() {
            return None;
        }
        item_pointer::begin(self, world, ItemControl::Button(button), point)
    }
    pub(super) fn captured_item_input(
        &mut self,
        world: &RuntimeView<'_>,
        views: Option<&SceneViews>,
        event: &InputEvent,
    ) -> Option<ScreenInputResponse> {
        item_pointer::input(self, world, views, event)
    }
    pub(super) fn cancel_item_pointer(&mut self) {
        if let Some(item) = &mut self.world.ui_item {
            item.cancel();
        }
    }
    pub(super) fn dragged_ui_item<'a>(
        &self,
        world: &RuntimeView<'a>,
    ) -> Option<&'a rusterix::Item> {
        item_pointer::dragged(self, world)
    }
    pub fn invalidate_world_input(&mut self) {
        self.hud.cancel_commands();
        self.world.invalidate();
        self.world.pointer = None;
        self.hovered_item = None;
    }
    pub fn resume_world_input(&mut self) {
        self.world.focused = true;
        self.invalidate_world_input();
    }
    /// Called only by the host's successful presentation acknowledgement.
    pub fn acknowledge_world_frame(&mut self, world: &RuntimeView<'_>, views: Option<&SceneViews>) {
        if let Some(frame) = &mut self.world.staged {
            frame.views.sort_by_key(|view| view.key);
        }
        self.world.displayed = self.world.staged.take();
        self.world.hover = if let Some(point) = self
            .world
            .pointer
            .filter(|_| !self.captures_world_input() && self.world.capture.is_none())
        {
            let can_drag = self
                .world
                .displayed
                .as_ref()
                .is_some_and(|f| !f.drops.is_empty());
            match self.query_world(
                world,
                views,
                point,
                can_drag && self.intent.trim().is_empty(),
            ) {
                Query::Hit { hit, .. } => Some(hit),
                _ => None,
            }
        } else {
            None
        };
        self.hovered_item = self.world.hover.and_then(|hit| hit.item);
        self.refresh_mouse_look(world, self.captures_world_input());
    }
    pub fn world_pointer_hit(&self) -> Option<WorldHit> {
        self.world.hover
    }
    pub(super) fn pointer_visual_position(&self) -> Option<[f32; 2]> {
        self.world
            .pointer
            .filter(|point| self.world.focused && self.inside_screen(*point))
    }
    pub(super) fn pointer_targets_world(&self, world: &RuntimeView<'_>) -> bool {
        self.pointer_visual_position()
            .is_some_and(|point| self.pointer_view(world, point).is_ok())
    }
    pub(super) fn pointer_cursor_target(
        &self,
        world: &RuntimeView<'_>,
    ) -> Option<(bool, bool, bool)> {
        if !self.frame_matches(world)
            || self.pointer_visual_position().is_none()
            || self.pointer_modal()
        {
            return None;
        }
        let (hit, pressed) = if let Some(capture) = &self.world.capture {
            if capture.cancelled || !self.world_source_current(capture) {
                return None;
            }
            (capture.hit, true)
        } else {
            if !self.pointer_targets_world(world) {
                return None;
            }
            (self.world.hover?, false)
        };
        let map = world.map?;
        Some((
            hit.entity
                .is_some_and(|id| map.entities.iter().any(|entity| entity.id == id)),
            hit.item
                .is_some_and(|id| map.items.iter().any(|item| item.id == id)),
            pressed,
        ))
    }
    pub(super) fn dragged_world_item(&self, world: &RuntimeView<'_>) -> Option<u32> {
        let capture = self.world.capture.as_ref()?;
        if !capture.dragging
            || capture.cancelled
            || !self.frame_matches(world)
            || !self.world_source_current(capture)
            || self.pointer_modal()
        {
            return None;
        }
        capture.item
    }
    pub fn world_captures_pointer(&self) -> bool {
        self.world.capture.is_some()
    }
    pub fn route_world_first(&self, event: &InputEvent) -> bool {
        (self.world.capture.is_some() || self.world.ui_item.is_some())
            && match event {
                InputEvent::PointerMoved { .. }
                | InputEvent::PointerButton { .. }
                | InputEvent::PointerLeft => true,
                InputEvent::Key {
                    physical: PhysicalKey::Escape,
                    pressed: true,
                    ..
                } => !self.pointer_modal(),
                _ => false,
            }
    }
    pub(super) fn world_capture_owner(&self) -> Option<ScreenCapture> {
        self.world.owner()
    }
    pub(super) fn author_modal(&self) -> bool {
        self.choice_controller.open_id().is_some() || self.message_blocks_input()
    }
    pub(super) fn pointer_modal(&self) -> bool {
        self.author_modal() || self.hud.popup_open()
    }
    fn pointer_occluders(&self, world: &RuntimeView<'_>) -> Vec<Uv> {
        self.pointer_controls(world)
            .into_iter()
            .map(|control| control.rect)
            .collect()
    }
    fn pointer_controls(&self, world: &RuntimeView<'_>) -> Vec<UiControl> {
        let model = &self.elements;
        let mut controls = self
            .world_texts
            .visible_bounds()
            .map(|rect| UiControl {
                action: None,
                owner: None,
                rect,
            })
            .collect::<Vec<_>>();
        let mut add = |name: &str, bounds: Rect, owner| {
            if !screen_model::widget_name_is_hidden(name, &model.widgets_to_hide)
                && visible_control_rect(model, bounds)
            {
                controls.push(UiControl {
                    action: None,
                    owner,
                    rect: rect(bounds),
                });
            }
        };
        for id in ordered(&model.avatar_widgets) {
            let widget = &model.avatar_widgets[&id];
            add(&widget.name, widget.rect, None);
        }
        for id in ordered(&model.profile_widgets) {
            let widget = &model.profile_widgets[&id];
            add(&widget.name, widget.rect, Some(DropOwner::Profile(id)));
        }
        for widget in &model.messages_widgets {
            add(&widget.name, widget.rect, None);
        }
        for id in ordered(&model.text_input_widgets) {
            let widget = &model.text_input_widgets[&id];
            add(&widget.name, widget.rect, None);
        }
        for id in screen_model::ordered_button_ids(&model.button_widgets) {
            let widget = &model.button_widgets[&id];
            add(&widget.name, widget.rect, Some(DropOwner::Button(id)));
        }
        for id in ordered(&model.choice_widgets) {
            if self.choice_controller.open_id() != Some(id) {
                let widget = &model.choice_widgets[&id];
                add(&widget.name, widget.rect, None);
            }
        }
        for hud in self.hud.frames(world, model) {
            controls.extend(hud.controls().into_iter().map(|(id, bounds)| UiControl {
                action: hud.action(id.part),
                owner: Some(DropOwner::Hud(id)),
                rect: rect(bounds),
            }));
        }
        if let Some(id) = self.choice_controller.open_id() {
            let widget = &model.choice_widgets[&id];
            if !screen_model::widget_name_is_hidden(&widget.name, &model.widgets_to_hide)
                && visible_control_rect(model, widget.rect)
            {
                controls.push(UiControl {
                    action: None,
                    owner: None,
                    rect: rect(widget.rect),
                });
            }
        }
        controls
    }
    fn pointer_drops(&self, world: &RuntimeView<'_>) -> Vec<DropTarget> {
        let Some(map) = world.map else {
            return Vec::new();
        };
        let mut targets = Vec::new();
        for id in ordered(&self.elements.profile_widgets) {
            let w = &self.elements.profile_widgets[&id];
            if screen_model::widget_name_is_hidden(&w.name, &self.elements.widgets_to_hide)
                || !visible_control_rect(&self.elements, w.rect)
            {
                continue;
            }
            if let Some(entity) = screen_model::resolve_party_entity(map, w.party.as_deref()) {
                targets.push(DropTarget {
                    source: DropOwner::Profile(id),
                    rect: w.rect,
                    destination: rusterix::item_interaction::ItemDestination::Actor {
                        entity: Some(entity.id),
                        inventory: None,
                        equipped: None,
                    },
                });
            }
        }
        for id in screen_model::ordered_button_ids(&self.elements.button_widgets) {
            let w = &self.elements.button_widgets[&id];
            if !w.drag_drop
                || (w.inventory_index.is_none() && w.equipped_slot.is_none())
                || screen_model::widget_name_is_hidden(&w.name, &self.elements.widgets_to_hide)
                || !visible_control_rect(&self.elements, w.rect)
            {
                continue;
            }
            if let Some(entity) = screen_model::resolve_party_entity(map, w.party.as_deref()) {
                targets.push(DropTarget {
                    source: DropOwner::Button(id),
                    rect: w.rect,
                    destination: rusterix::item_interaction::ItemDestination::Actor {
                        entity: Some(entity.id),
                        inventory: w.inventory_index,
                        equipped: w.equipped_slot.clone(),
                    },
                });
            }
        }
        for hud in self.hud.frames(world, &self.elements) {
            targets.extend(hud.controls().into_iter().filter_map(|(id, bounds)| {
                Some(DropTarget {
                    source: DropOwner::Hud(id),
                    rect: bounds,
                    destination: hud.destination(id.part)?,
                })
            }));
        }

        targets
    }
    pub(super) fn begin_world_frame(&mut self, world: &RuntimeView<'_>) -> Result<(), String> {
        self.world.staged = None;
        let immediate = self.immediate_intents(world)?;
        self.world.staged = if self.world.focused {
            Binding::new(world, self.revision.unwrap(), self.elements.layout.viewport).map(
                |binding| Frame {
                    binding,
                    views: Vec::new(),
                    cells: None,
                    occluders: self.pointer_occluders(world),
                    drops: self.pointer_drops(world),
                    modal: self.pointer_modal(),
                    author_modal: self.author_modal(),
                    immediate,
                    items: self.item_slots(world),
                    controls: self.pointer_controls(world),
                    commands: self.command_targets(world),
                },
            )
        } else {
            None
        };
        Ok(())
    }
    /// Projected overlays are prepared after each game view's camera. Freeze
    /// their final hit geometry only once the matching paint list is complete.
    pub(super) fn finish_world_frame(&mut self, world: &RuntimeView<'_>) {
        let binding = Binding::new(
            world,
            self.revision.unwrap_or(ContentRevision(0)),
            self.elements.layout.viewport,
        );
        if !self
            .world
            .staged
            .as_ref()
            .is_some_and(|frame| Some(frame.binding) == binding)
        {
            self.world.staged = None;
            return;
        }
        let occluders = self.pointer_occluders(world);
        let drops = world
            .map
            .map(|_| self.pointer_drops(world))
            .unwrap_or_default();
        let modal = self.pointer_modal();
        let author_modal = self.author_modal();
        let items = world
            .map
            .map(|_| self.item_slots(world))
            .unwrap_or_default();
        let controls = self.pointer_controls(world);
        let commands = self.command_targets(world);
        let frame = self.world.staged.as_mut().unwrap();
        frame.occluders = occluders;
        frame.drops = drops;
        frame.modal = modal;
        frame.author_modal = author_modal;
        frame.items = items;
        frame.controls = controls;
        frame.commands = commands;
    }
    fn frame_matches(&self, world: &RuntimeView<'_>) -> bool {
        world.state == ServerState::Running && self.ui_frame_matches(world)
    }
    fn ui_frame_matches(&self, world: &RuntimeView<'_>) -> bool {
        world.state != ServerState::Off && self.presentation_frame_matches(world)
    }
    fn presentation_frame_matches(&self, world: &RuntimeView<'_>) -> bool {
        self.world.focused
            && self.world.displayed.as_ref().is_some_and(|frame| {
                Some(frame.binding)
                    == Binding::new(
                        world,
                        self.revision.unwrap_or(ContentRevision(0)),
                        self.elements.layout.viewport,
                    )
            })
    }
    fn pointer_view(&self, world: &RuntimeView<'_>, point: [f32; 2]) -> Result<&ViewFrame, Query> {
        if !self.frame_matches(world) {
            return Err(Query::Unavailable);
        }
        let frame = self.world.displayed.as_ref().unwrap();
        let root = Uv::new(
            0.,
            0.,
            frame.binding.size.0 as f32,
            frame.binding.size.1 as f32,
        );
        if !inside_frame(frame.binding, point) || !root.contains(point) {
            return Err(Query::Outside);
        }
        if frame.modal
            || self.pointer_modal()
            || frame
                .occluders
                .iter()
                .chain(self.pointer_occluders(world).iter())
                .any(|r| r.contains(point))
        {
            return Err(Query::Outside);
        }
        let Some(view) = frame
            .views
            .iter()
            .rev()
            .find(|view| rect(view.rect).contains(point))
        else {
            return Err(Query::Outside);
        };
        let Some(current) = self.elements.game_widgets.get(&view.key) else {
            return Err(Query::Unavailable);
        };
        if current.rect != view.rect
            || current.camera != view.camera
            || screen_model::widget_name_is_hidden(&current.name, &self.elements.widgets_to_hide)
        {
            return Err(Query::Unavailable);
        }
        Ok(view)
    }
    fn query_world(
        &self,
        world: &RuntimeView<'_>,
        views: Option<&SceneViews>,
        point: [f32; 2],
        prefer_item: bool,
    ) -> Query {
        let view = match self.pointer_view(world, point) {
            Ok(view) => view,
            Err(query) => return query,
        };
        let frame = self.world.displayed.as_ref().unwrap();
        let raw = [
            (point[0] - view.rect.x) / view.rect.width,
            (point[1] - view.rect.y) / view.rect.height,
        ];
        if raw.iter().any(|v| !v.is_finite() || *v < 0. || *v >= 1.) {
            return Query::Outside;
        }
        // Game images use nearest sampling. Query the source pixel actually
        // shown under the pointer, including fractional layout and upscaling.
        let uv = [
            (raw[0] * view.source_size[0]).floor() + 0.5,
            (raw[1] * view.source_size[1]).floor() + 0.5,
        ];
        let uv = [uv[0] / view.source_size[0], uv[1] / view.source_size[1]];
        let hit = match &view.projection {
            Projection::Flat {
                origin,
                grid,
                logical,
            } => {
                if !grid.is_finite() || *grid <= 0. {
                    return Query::Unavailable;
                }
                frame.cells.as_ref().and_then(|cells| {
                    cells.hit(
                        *origin + Vec2::new(uv[0] * logical[0], uv[1] * logical[1]) / *grid,
                        prefer_item,
                    )
                })
            }
            Projection::Ray(output) => match views.map(|views| views.pick_game(*output, uv)) {
                Some(ViewPick::Hit(hit)) => world_interaction::geometry_hit(
                    world.map.unwrap(),
                    world.assets,
                    hit.geo,
                    Vec3::new(hit.world[0], hit.world[1], hit.world[2]),
                    hit.distance,
                    hit.paint_geo,
                ),
                Some(ViewPick::Miss(_)) => None,
                _ => return Query::Unavailable,
            },
        };
        hit.map_or(Query::Outside, |hit| Query::Hit {
            view: view.key,
            hit,
        })
    }
    pub(super) fn consume_targeting_intent(&mut self) {
        self.intent.clear();
        for selected in [
            &mut self.elements.activated_widgets,
            &mut self.elements.permanently_activated_widgets,
        ] {
            selected.retain(|id| {
                self.elements
                    .button_widgets
                    .get(id)
                    .is_none_or(|w| !screen_model::is_targeting_button(w))
            });
        }
        self.geometry = None;
    }
    fn apply_world_click_policy(&mut self, click: WorldClick) -> Vec<ScreenEffect> {
        match click {
            WorldClick::Action {
                action,
                uses_intent,
            } => {
                if uses_intent
                    && self
                        .world
                        .displayed
                        .as_ref()
                        .is_some_and(|frame| frame.immediate)
                {
                    self.consume_targeting_intent();
                }
                vec![ScreenEffect::Action(action)]
            }
            WorldClick::OpenContainer(item) => {
                vec![ScreenEffect::Action(EntityAction::OpenContainer {
                    item_id: item,
                    owner_entity_id: None,
                })]
            }
            WorldClick::BeginItemDrag(_) => Vec::new(),
        }
    }
    fn world_source_current(&self, capture: &Capture) -> bool {
        let Some(frame) = &self.world.displayed else {
            return false;
        };
        frame.binding.same_source(capture.binding)
            && frame.views.iter().any(|v| {
                v.key == capture.view && v.rect == capture.rect && v.camera == capture.camera
            })
            && self
                .elements
                .game_widgets
                .get(&capture.view)
                .is_some_and(|v| {
                    v.rect == capture.rect
                        && v.camera == capture.camera
                        && !screen_model::widget_name_is_hidden(
                            &v.name,
                            &self.elements.widgets_to_hide,
                        )
                })
    }
    fn click_release_visible(
        &self,
        world: &RuntimeView<'_>,
        capture: &Capture,
        point: [f32; 2],
    ) -> bool {
        let Some(frame) = &self.world.displayed else {
            return false;
        };
        inside_frame(frame.binding, point)
            && !frame.modal
            && !self.pointer_modal()
            && rect(capture.rect).contains(point)
            && frame
                .views
                .iter()
                .rev()
                .find(|v| rect(v.rect).contains(point))
                .is_some_and(|v| v.key == capture.view)
            && !frame
                .occluders
                .iter()
                .chain(self.pointer_occluders(world).iter())
                .any(|r| r.contains(point))
    }
    fn release_world_drag(
        &self,
        world: &RuntimeView<'_>,
        views: Option<&SceneViews>,
        capture: &Capture,
        point: [f32; 2],
    ) -> Option<EntityAction> {
        if capture.cancelled || !self.frame_matches(world) || !self.world_source_current(capture) {
            return None;
        }
        let frame = self.world.displayed.as_ref()?;
        if !inside_frame(frame.binding, point) {
            return None;
        }
        if !frame.binding.same_source(capture.binding) || self.pointer_modal() || frame.modal {
            return None;
        }
        self.release_item_drag(
            world,
            views,
            rusterix::item_interaction::ItemSource::Direct(rusterix::item_interaction::ItemRef {
                item: capture.item?,
                owner: None,
            }),
            capture.binding,
            point,
        )
    }
    /// Called after UI routing, except that an already captured world gesture
    /// receives its own pointer continuation/release before unrelated controls.
    pub fn world_input(
        &mut self,
        world: &RuntimeView<'_>,
        views: Option<&SceneViews>,
        event: &InputEvent,
        blocked: bool,
    ) -> ScreenInputResponse {
        if let Some(response) = self.captured_item_input(world, views, event) {
            return response;
        }
        if let InputEvent::Focus(focused) = event {
            self.world.focused = *focused;
            self.world.invalidate();
            self.world.capture = None;
            self.world.pointer = None;
            self.hovered_item = None;
            return ScreenInputResponse {
                input: theframework::platform::InputResponse {
                    consumed: false,
                    redraw: *focused,
                },
                ..Default::default()
            };
        }
        if matches!(event, InputEvent::PointerLeft) {
            self.world.pointer = None;
            self.world.cancel();
            self.hovered_item = None;
            return ScreenInputResponse::handled(true);
        }
        if matches!(
            event,
            InputEvent::Key {
                physical: PhysicalKey::Escape,
                pressed: true,
                ..
            }
        ) && (self.world.capture.is_some() || self.world.look.is_some())
        {
            self.world.capture = None;
            self.world.look = None;
            self.world.pointer = None;
            self.world.hover = None;
            self.hovered_item = None;
            return ScreenInputResponse::handled(true);
        }
        if blocked {
            self.world.cancel();
        }
        if self.world.bound
            != Binding::new(
                world,
                self.revision.unwrap_or(ContentRevision(0)),
                self.elements.layout.viewport,
            )
            || !self.world.focused
        {
            self.world.invalidate();
        }
        if world.state != ServerState::Running {
            self.world.cancel_world();
        }
        if let InputEvent::RelativeMotion { delta } = event {
            return self.mouse_look_motion(world, *delta, blocked);
        }
        let point = match event {
            InputEvent::PointerMoved { position } | InputEvent::PointerButton { position, .. } => {
                *position
            }
            _ => return ScreenInputResponse::default(),
        };
        self.cursor = point;
        self.world.pointer = Some(point);
        self.refresh_mouse_look(world, blocked);
        if let Some(capture) = &mut self.world.capture {
            if let InputEvent::PointerMoved { .. } = event {
                capture.dragging |= (point[0] - capture.point[0]).powi(2)
                    + (point[1] - capture.point[1]).powi(2)
                    >= 16.;
                return ScreenInputResponse::handled(true);
            }
            if matches!(
                event,
                InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed: false,
                    ..
                }
            ) {
                let mut capture = self.world.capture.take().unwrap();
                capture.dragging |= (point[0] - capture.point[0]).powi(2)
                    + (point[1] - capture.point[1]).powi(2)
                    >= 16.;
                let effects = if capture.item.is_some()
                    && !capture.cancelled
                    && !blocked
                    && self.frame_matches(world)
                {
                    if capture.dragging {
                        self.release_world_drag(world, views, &capture, point)
                            .map(ScreenEffect::Action)
                            .into_iter()
                            .collect()
                    } else if self.world_source_current(&capture)
                        && self.click_release_visible(world, &capture, point)
                    {
                        world_interaction::click(
                            world.map.unwrap(),
                            world.assets,
                            capture.hit,
                            None,
                            false,
                        )
                        .map(|click| self.apply_world_click_policy(click))
                        .unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };
                self.world.hover = None;
                self.hovered_item = None;
                return ScreenInputResponse {
                    effects,
                    ..ScreenInputResponse::handled(true)
                };
            }
            if matches!(event, InputEvent::PointerButton { .. }) {
                return ScreenInputResponse::handled(false);
            }
        }
        if blocked {
            self.world.hover = None;
            self.hovered_item = None;
            return ScreenInputResponse::default();
        }
        let can_drag = self
            .world
            .displayed
            .as_ref()
            .is_some_and(|f| !f.drops.is_empty());
        let intent = (!self.intent.trim().is_empty()).then(|| self.intent.clone());
        let query = self.query_world(world, views, point, can_drag && intent.is_none());
        self.world.hover = match query {
            Query::Hit { hit, .. } => Some(hit),
            _ => None,
        };
        self.hovered_item = self.world.hover.and_then(|hit| hit.item);
        if matches!(
            event,
            InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                ..
            }
        ) {
            match query {
                Query::Hit { view, hit } => {
                    let click = world_interaction::click(
                        world.map.unwrap(),
                        world.assets,
                        hit,
                        intent.as_deref(),
                        can_drag,
                    );
                    let item = match &click {
                        Some(WorldClick::BeginItemDrag(item)) => Some(*item),
                        _ => None,
                    };
                    let source = self
                        .world
                        .displayed
                        .as_ref()
                        .unwrap()
                        .views
                        .iter()
                        .find(|source| source.key == view)
                        .unwrap();
                    self.world.capture = Some(Capture {
                        rect: source.rect,
                        camera: source.camera.clone(),
                        view,
                        binding: self.world.displayed.as_ref().unwrap().binding,
                        point,
                        hit,
                        item,
                        dragging: false,
                        cancelled: false,
                    });
                    let effects = click
                        .map(|click| self.apply_world_click_policy(click))
                        .unwrap_or_default();
                    return ScreenInputResponse {
                        effects,
                        cancel_controls: true,
                        ..ScreenInputResponse::handled(true)
                    };
                }
                Query::Unavailable => return ScreenInputResponse::handled(true),
                Query::Outside => {}
            }
        }
        ScreenInputResponse {
            input: theframework::platform::InputResponse {
                consumed: false,
                redraw: matches!(event, InputEvent::PointerMoved { .. }),
            },
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests;

impl GameScreenController {
    pub(super) fn hud_frame_valid(
        &self,
        world: &RuntimeView<'_>,
        id: super::hud::HudControlId,
        bounds: Rect,
    ) -> bool {
        self.hud_action_valid(world, id, bounds, None)
    }
    pub(super) fn hud_action_valid(
        &self,
        world: &RuntimeView<'_>,
        id: super::hud::HudControlId,
        bounds: Rect,
        action: Option<&super::hud::model::HudAction>,
    ) -> bool {
        let matches = |control: &UiControl| {
            control.owner == Some(DropOwner::Hud(id))
                && control.rect == rect(bounds)
                && control.action.as_ref() == action
        };
        self.presentation_frame_matches(world)
            && !self.author_modal()
            && self
                .world
                .displayed
                .as_ref()
                .is_some_and(|frame| !frame.author_modal && frame.controls.iter().any(matches))
            && self.pointer_controls(world).iter().any(matches)
    }
    pub(super) fn hud_control_visible(
        &self,
        world: &RuntimeView<'_>,
        id: super::hud::HudControlId,
        point: [f32; 2],
    ) -> bool {
        let matches = |controls: &[UiControl]| {
            controls
                .iter()
                .rev()
                .find(|control| control.rect.contains(point))
                .is_some_and(|control| control.owner == Some(DropOwner::Hud(id)))
        };
        self.world
            .displayed
            .as_ref()
            .is_some_and(|frame| matches(&frame.controls))
            && matches(&self.pointer_controls(world))
    }
    pub(super) fn hud_item_begin(
        &mut self,
        world: &RuntimeView<'_>,
        id: super::hud::HudControlId,
        point: [f32; 2],
    ) -> Option<ScreenInputResponse> {
        item_pointer::begin(self, world, ItemControl::Hud(id), point)
    }
    pub(super) fn hud_item_receipt(
        &self,
        world: &RuntimeView<'_>,
        id: super::hud::HudControlId,
    ) -> Result<Option<Arc<ItemReceipt>>, ()> {
        item_pointer::receipt_for_control(self, world, ItemControl::Hud(id), None)
    }
    pub(super) fn hud_item_activate(
        &mut self,
        world: &RuntimeView<'_>,
        receipt: &ItemReceipt,
    ) -> Vec<ScreenEffect> {
        item_pointer::activate_hud_receipt(self, world, receipt)
    }
}

impl GameScreenController {
    pub(super) fn hud_binding(&self, world: &RuntimeView<'_>) -> Option<Binding> {
        self.presentation_frame_matches(world)
            .then(|| self.world.displayed.as_ref().unwrap().binding)
    }
    pub(super) fn hud_capture_valid(&self, world: &RuntimeView<'_>, binding: Binding) -> bool {
        self.hud_binding(world) == Some(binding) && !self.author_modal()
    }
}

impl GameScreenController {
    fn command_targets(&self, _world: &RuntimeView<'_>) -> Vec<CommandTarget> {
        screen_model::ordered_button_ids(&self.elements.button_widgets)
            .into_iter()
            .filter_map(|id| {
                let widget = &self.elements.button_widgets[&id];
                if screen_model::widget_name_is_hidden(&widget.name, &self.elements.widgets_to_hide)
                    || !visible_control_rect(&self.elements, widget.rect)
                {
                    return None;
                }
                let slot = widget.command_slot.as_deref()?.trim();
                (!slot.is_empty()).then(|| CommandTarget {
                    button: id,
                    slot: slot.to_string(),
                    rect: widget.rect,
                })
            })
            .collect()
    }
    pub(super) fn hud_displayed_immediate(&self, world: &RuntimeView<'_>) -> Option<bool> {
        self.ui_frame_matches(world)
            .then(|| self.world.displayed.as_ref().unwrap().immediate)
    }
    pub(super) fn command_target_at(
        &self,
        world: &RuntimeView<'_>,
        point: [f32; 2],
    ) -> Option<CommandTarget> {
        if !self.ui_frame_matches(world) || self.pointer_modal() {
            return None;
        }
        let frame = self.world.displayed.as_ref()?;
        let owner = frame
            .controls
            .iter()
            .rev()
            .find(|control| control.rect.contains(point))?
            .owner?;
        let DropOwner::Button(button) = owner else {
            return None;
        };
        let old = frame
            .commands
            .iter()
            .find(|target| target.button == button && rect(target.rect).contains(point))?;
        let live = self
            .command_targets(world)
            .into_iter()
            .find(|target| target.button == button)?;
        if live != *old
            || self
                .pointer_controls(world)
                .iter()
                .rev()
                .find(|control| control.rect.contains(point))
                .and_then(|control| control.owner)
                != Some(owner)
        {
            return None;
        }
        Some(live)
    }
    pub(super) fn command_target_valid(
        &self,
        world: &RuntimeView<'_>,
        target: &CommandTarget,
        point: [f32; 2],
    ) -> bool {
        self.command_target_at(world, point).as_ref() == Some(target)
    }
}
