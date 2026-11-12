//! Application-owned native screen/controller state. It borrows CPU runtime
//! views; a host supplies scene/image services and owns its surface and clock.
mod choice;
mod commands;
mod cursor;
mod focus;
mod fonts;
pub use commands::{ScreenActivation, ScreenEffect};
pub use focus::{ScreenCapture, ScreenFocus};
mod hud;
pub use hud::{HudControlId, HudPanelId, HudPart};
mod input;
mod keyboard;
mod router;
mod speech;
pub use speech::SpeechOverlay;
mod transcript;
mod world_pointer;
mod world_text;
pub use router::ScreenInputResponse;
pub use world_text::WorldTextOverlay;
mod paint;
use crate::{GameSceneInput, SceneViews, ViewId, ViewImage, screen::*};
use eldiron_runtime::{RegionEvents, RuntimeView};
pub use fonts::{GameFontVersion, GameFonts};
pub use input::{ScreenTextResponse, ScreenTextTarget};
use renderkit::ImageRegistry;
use rustc_hash::FxHashMap;
use rusterix::{
    Currencies, D2Builder, Map, Rect, Scene,
    client::{
        content::{ContentRecipe, SourceFit},
        screen_model::{self, ScreenElements, ScreenLayout},
    },
};
use std::sync::Arc;
use theframework::paint::{PaintList, PaintOp, Rect as Uv, Sampling, Transform};
use theframework_core::Uuid;

struct ProfileContent {
    layout: ProfileLayout,
    source: Option<Arc<PreparedContent>>,
}
struct AvatarContent {
    source: Option<Arc<PreparedContent>>,
    outline: ([u8; 4], usize),
}
struct ButtonSource {
    source: Arc<PreparedContent>,
    quantity: u32,
}
struct ScreenGeometry {
    base: Scene,
    foreground: Scene,
}

pub struct GameScreenController {
    session: Uuid,
    hud: hud::Hud,
    pointer_visuals: cursor::PointerVisuals,
    world: world_pointer::WorldPointer,
    pub speech: SpeechOverlay,
    pub world_texts: WorldTextOverlay,
    pub elements: ScreenElements,
    pub name: String,
    pub cursor: [f32; 2],
    focus: Option<ScreenFocus>,
    keyboard_capture: Option<focus::KeyboardCapture>,
    pub pressed_button: Option<u32>,
    pub choice_controller: screen_model::ChoiceController,
    pub intent: String,
    pub hovered_item: Option<u32>,
    sources: ContentSources,
    revision: Option<ContentRevision>,
    font_version: Option<GameFontVersion>,
    currencies: Currencies,
    empty_map: Map,
    messages: Vec<MessageLayouts>,
    profiles: FxHashMap<Uuid, ProfileContent>,
    avatars: FxHashMap<Uuid, AvatarContent>,
    buttons: FxHashMap<u32, ButtonSource>,
    geometry: Option<ScreenGeometry>,
    view_ids: FxHashMap<Uuid, ViewId>,
    configured_views: FxHashMap<ViewId, ContentRevision>,
    view_images: FxHashMap<Uuid, ViewImage>,
    inputs: FxHashMap<ScreenTextTarget, theframework::text_edit::EditorView>,
    input_modifiers: theframework::platform::Modifiers,
    pub animation_frame: usize,
    pointer_capture: Option<router::PointerCapture>,
    suppress_activation_slash: bool,
    message_scroll: FxHashMap<u32, f32>,
}
impl GameScreenController {
    /// Move project/session presentation owners into a replacement screen.
    /// Authored controls, editor/clipboard targets, scene outputs and input
    /// captures are not transferred. Hosts call this before preparing the new
    /// screen, and return the owners to the old screen if preparation fails.
    pub fn retain_session_presentation_from(&mut self, previous: &mut Self, fonts: &mut GameFonts) {
        if self.session == previous.session {
            self.sources = std::mem::take(&mut previous.sources);
            self.hud = std::mem::take(&mut previous.hud);
            self.speech = std::mem::take(&mut previous.speech);
            self.world_texts = std::mem::take(&mut previous.world_texts);
            self.speech.invalidate_geometry();
            self.world_texts.invalidate_geometry();
        }
        self.hud.after_navigation();
        self.focus_control(fonts, None);
        self.cancel_keyboard_capture();
        self.pointer_capture = self
            .pointer_capture
            .take()
            .map(|_| router::PointerCapture::Cancelled);
        self.pressed_button = None;
        self.elements.release_button_state();
        self.invalidate_world_input();
    }
    /// Runtime incarnation last observed by this retained screen. Hosts can
    /// revoke asynchronous platform requests before delivering their results.
    pub fn runtime_session(&self) -> Uuid {
        self.session
    }
    pub fn from_screen(
        name: &str,
        world: &RuntimeView<'_>,
        layout: ScreenLayout,
    ) -> Result<Self, String> {
        Self::from_screen_with_state(name, world, layout, FxHashMap::default())
    }
    /// Bindings live across navigation; a new screen compiles its controls from
    /// the existing values while retaining the project's separate font owner.
    pub fn from_screen_with_state(
        name: &str,
        world: &RuntimeView<'_>,
        mut layout: ScreenLayout,
        state: FxHashMap<String, String>,
    ) -> Result<Self, String> {
        let screen = world
            .screen(name)
            .ok_or_else(|| format!("Screen '{name}' not found"))?;
        layout.screen_responsive = screen.responsive;
        if !layout.screen_responsive {
            layout.viewport = layout.reference_viewport;
        }
        let config = world
            .config()
            .parse::<toml::Table>()
            .map_err(|e| e.to_string())?;
        let elements = ScreenElements::compile(
            screen.map,
            world.assets,
            layout,
            screen_model::screen_background(&config),
            state,
            0,
        );
        Ok(Self::with_elements(name, world, elements))
    }
    pub fn region_fallback(
        world: &RuntimeView<'_>,
        mut layout: ScreenLayout,
        state: FxHashMap<String, String>,
    ) -> Result<Self, String> {
        let map = world
            .map
            .ok_or("Select a runtime region before preparing a fallback screen")?;
        if !layout.screen_responsive {
            layout.viewport = layout.reference_viewport;
        }
        let config = world
            .config()
            .parse::<toml::Table>()
            .map_err(|e| e.to_string())?;
        let elements = ScreenElements::region_fallback(
            &map.name,
            layout,
            screen_model::screen_background(&config),
            state,
        );
        Ok(Self::with_elements(&map.name, world, elements))
    }
    fn with_elements(name: &str, world: &RuntimeView<'_>, elements: ScreenElements) -> Self {
        Self {
            session: world.session_id,
            hud: Default::default(),
            pointer_visuals: Default::default(),
            world: world_pointer::WorldPointer::default(),
            speech: SpeechOverlay::default(),
            world_texts: WorldTextOverlay::default(),
            elements,
            name: name.into(),
            cursor: [0.; 2],
            focus: None,
            keyboard_capture: None,
            pressed_button: None,
            choice_controller: screen_model::ChoiceController::default(),
            intent: String::new(),
            hovered_item: None,
            sources: ContentSources::default(),
            revision: None,
            font_version: None,
            currencies: Currencies::from_rules_source(&world.assets.rules),
            empty_map: Map::default(),
            messages: Vec::new(),
            profiles: FxHashMap::default(),
            avatars: FxHashMap::default(),
            buttons: FxHashMap::default(),
            geometry: None,
            view_ids: FxHashMap::default(),
            configured_views: FxHashMap::default(),
            view_images: FxHashMap::default(),
            inputs: FxHashMap::default(),
            input_modifiers: Default::default(),
            animation_frame: 0,
            pointer_capture: None,
            suppress_activation_slash: false,
            message_scroll: FxHashMap::default(),
        }
    }
    /// Bind before receiving or routing events, not only at frame preparation:
    /// the first fresh batch must survive, and stale choices/IME must not act
    /// during the interval before the next frame. Authored bindings survive.
    fn bind_runtime(&mut self, world: &RuntimeView<'_>) -> bool {
        if self.session == world.session_id {
            return false;
        }
        self.session = world.session_id;
        self.hud = Default::default();
        self.pointer_visuals = Default::default();
        self.speech.bind_runtime(world);
        self.world_texts.bind_runtime(world);
        self.invalidate_world_input();
        self.focus = None;
        self.inputs.clear();
        self.input_modifiers = Default::default();
        self.keyboard_capture = None;
        self.pointer_capture = self
            .pointer_capture
            .take()
            .map(|_| router::PointerCapture::Cancelled);
        self.pressed_button = None;
        self.suppress_activation_slash = false;
        self.intent.clear();
        self.choice_controller
            .cancel(&mut self.elements.choice_widgets);
        self.choice_controller = screen_model::ChoiceController::default();
        self.elements.release_button_state();
        for widget in &mut self.elements.messages_widgets {
            widget.reset_runtime();
        }
        self.message_scroll.clear();
        self.messages.clear();
        self.profiles.clear();
        self.avatars.clear();
        self.buttons.clear();
        self.sources.clear();
        self.revision = None;
        self.geometry = None;
        self.configured_views.clear();
        self.view_images.clear();
        true
    }
    /// Distribute each addressed batch once. Reflow/painting do not consume it
    /// again; the application can also deliver it to audio/world-text consumers.
    pub fn receive(&mut self, world: &RuntimeView<'_>, events: &RegionEvents) {
        self.bind_runtime(world);
        self.hud
            .receive(world, events, self.elements.layout.viewport);
        self.speech.receive(world, events);
        self.world_texts.receive(world, events);
        let map = world.map.unwrap_or(&self.empty_map);
        for widget in &mut self.elements.messages_widgets {
            widget.queue_messages(
                world.assets,
                map,
                &world.time,
                &events.messages,
                &events.choices,
            );
        }
    }
    pub fn resize(&mut self, logical: (u32, u32)) -> bool {
        let changed = self.elements.relayout(theframework::prelude::Vec2::new(
            logical.0.min(i32::MAX as u32) as i32,
            logical.1.min(i32::MAX as u32) as i32,
        ));
        if changed {
            self.invalidate_world_input();
            self.geometry = None;
            self.speech.invalidate_geometry();
            self.world_texts.invalidate_geometry();
        }
        changed
    }
    /// Prepare source artwork, message hit geometry and retained identity text
    /// before routing input. Only a content/publication edit advances revision.
    pub fn prepare(
        &mut self,
        world: &RuntimeView<'_>,
        fonts: &mut GameFonts,
        revision: ContentRevision,
        animation_frame: usize,
    ) -> Result<(), String> {
        self.bind_runtime(world);
        let font_version = fonts.refresh(world, revision);
        if self.font_version != Some(font_version) {
            self.font_version = Some(font_version);
            self.messages.clear();
            self.profiles.clear();
        }
        if self.revision != Some(revision) {
            self.currencies = Currencies::from_rules_source(&world.assets.rules);
            self.geometry = None;
            self.revision = Some(revision);
        }
        if self
            .world
            .sync(world, revision, self.elements.layout.viewport)
        {
            self.hud.cancel_commands();
            self.configured_views.clear();
            for widget in self.elements.game_widgets.values_mut() {
                widget.force_dynamics_rebuild = true;
            }
        }
        self.animation_frame = animation_frame;
        self.choice_controller.prepare(
            &mut self.elements.choice_widgets,
            &self.elements.widgets_to_hide,
            &self.elements.ui_state,
        );
        let map = world.map.unwrap_or(&self.empty_map);
        self.messages.resize_with(
            self.elements.messages_widgets.len(),
            MessageLayouts::default,
        );
        for (widget, layout) in self
            .elements
            .messages_widgets
            .iter_mut()
            .zip(&mut self.messages)
        {
            if screen_model::widget_name_is_hidden(&widget.name, &self.elements.widgets_to_hide) {
                continue;
            }
            widget.set_text_input_available(true);
            layout.use_command_editor(true);
            layout.prepare(
                widget,
                &mut fonts.text,
                world.assets,
                map,
                &world.time,
                &[],
                &[],
            )?;
        }
        self.avatars.clear();
        for (id, widget) in &mut self.elements.avatar_widgets {
            if screen_model::widget_name_is_hidden(&widget.name, &self.elements.widgets_to_hide) {
                continue;
            }
            let Some(entity) = screen_model::resolve_party_entity(map, widget.party.as_deref())
            else {
                continue;
            };
            let outline = widget.prepare_outline(entity, world.assets);
            let source =
                self.sources
                    .prepare(widget.content_recipe(entity), world.assets, revision)?;
            self.avatars.insert(*id, AvatarContent { source, outline });
        }
        for (id, widget) in &mut self.elements.profile_widgets {
            if screen_model::widget_name_is_hidden(&widget.name, &self.elements.widgets_to_hide) {
                if let Some(content) = self.profiles.get_mut(id) {
                    content.source = None;
                }
                continue;
            }
            widget.animation_frame = animation_frame;
            let Some(entity) = screen_model::resolve_party_entity(map, widget.party.as_deref())
            else {
                self.profiles.remove(id);
                continue;
            };
            let source =
                self.sources
                    .prepare(widget.content_recipe(entity), world.assets, revision)?;
            let content = self.profiles.entry(*id).or_insert_with(|| ProfileContent {
                layout: ProfileLayout::default(),
                source: None,
            });
            content
                .layout
                .prepare(widget, entity, &mut fonts.text, "Roboto", source.as_deref());
            content.source = source;
        }
        self.buttons.clear();
        for (id, widget) in &self.elements.button_widgets {
            if screen_model::widget_name_is_hidden(&widget.name, &self.elements.widgets_to_hide) {
                continue;
            }
            let Some(entity) = screen_model::resolve_party_entity(map, widget.party.as_deref())
            else {
                continue;
            };
            if widget.portrait {
                if let Some(source) = self.sources.prepare(
                    ContentRecipe::Portrait {
                        entity,
                        frame: animation_frame,
                    },
                    world.assets,
                    revision,
                )? {
                    self.buttons.insert(
                        *id,
                        ButtonSource {
                            source,
                            quantity: 1,
                        },
                    );
                    continue;
                }
            }
            let item = if let Some(index) = widget.inventory_index {
                entity.inventory.get(index).and_then(Option::as_ref)
            } else {
                widget
                    .equipped_slot
                    .as_deref()
                    .and_then(|slot| entity.get_equipped_item(slot))
            };
            if let Some(item) = item {
                if let Some(source) = self.sources.prepare(
                    ContentRecipe::Item {
                        item,
                        frame: animation_frame,
                    },
                    world.assets,
                    revision,
                )? {
                    self.buttons.insert(
                        *id,
                        ButtonSource {
                            source,
                            quantity: item.stack_quantity().max(0) as u32,
                        },
                    );
                }
            }
        }
        self.hud.prepare(
            world,
            &self.elements,
            fonts,
            &mut self.sources,
            revision,
            animation_frame,
        )?;
        self.prepare_inputs(fonts);
        self.prepare_focus(fonts);
        for widget in self.elements.deco_widgets.values_mut() {
            widget.animation_frame = animation_frame;
        }
        if self.geometry.is_none() {
            let size = self.elements.layout.viewport.map(|v| v as f32);
            let mut builder = D2Builder::new();
            builder.activated_widgets = self.elements.activated_widgets.clone();
            self.geometry = Some(ScreenGeometry {
                base: builder.build(&self.elements.render_layer(false), world.assets, size),
                foreground: builder.build(&self.elements.render_layer(true), world.assets, size),
            });
        }
        self.message_scroll
            .retain(|id, _| self.elements.messages_widgets.iter().any(|w| w.id == *id));
        self.speech.prepare(world, &self.elements, fonts);
        self.world_texts.prepare(world, &self.elements, fonts);
        self.prepare_pointer_visuals(world, revision)?;
        Ok(())
    }
    pub fn prepare_game_views(
        &mut self,
        world: &RuntimeView<'_>,
        views: &mut SceneViews,
        images: &mut ImageRegistry,
    ) -> Result<(), String> {
        if self.session != world.session_id {
            return Err("Prepare screen layouts after changing their runtime session".into());
        }
        self.view_images.clear();
        self.begin_world_frame(world)?;
        let Some(map) = world.map else {
            return Ok(());
        };
        let revision = self
            .revision
            .ok_or("Prepare the screen before its scene views")?;
        let input = GameSceneInput {
            map,
            assets: world.assets,
            time: world.time,
            animation_frame: self.animation_frame,
            render_state: &world.render_state,
        };
        for (key, widget) in &mut self.elements.game_widgets {
            if screen_model::widget_name_is_hidden(&widget.name, &self.elements.widgets_to_hide) {
                continue;
            }
            let id = *self.view_ids.entry(*key).or_default();
            if views.scene(id).is_err() {
                views.create(id, (0, 0)).map_err(|e| e.to_string())?;
                self.configured_views.remove(&id);
            }
            if self.configured_views.get(&id) != Some(&revision) {
                views
                    .configure_game(id, world.assets)
                    .map_err(|e| e.to_string())?;
                widget.rebuild_graphical_scene(
                    map,
                    world.assets,
                    views.scene_mut(id).map_err(|e| e.to_string())?,
                );
                self.configured_views.insert(id, revision);
            }
            let size = (
                widget.rect.width.round().max(0.) as u32,
                widget.rect.height.round().max(0.) as u32,
            );
            views
                .prepare_game(id, widget, size, &input, images)
                .map_err(|e| e.to_string())?;
            if let Some(output) = views
                .render(id, self.animation_frame as u64, images)
                .map_err(|e| e.to_string())?
            {
                self.world.push_view(*key, widget, output, map)?;
                self.view_images.insert(*key, output);
            }
        }
        Ok(())
    }
    /// The host calls this before closing/replacing the screen while retaining
    /// its registry. IDs remain stable if a device owner is rebuilt afterward.
    pub fn release_game_views(
        &mut self,
        views: &mut SceneViews,
        images: &mut ImageRegistry,
    ) -> Result<(), String> {
        self.invalidate_world_input();
        for id in self.view_ids.values() {
            views.remove(*id, images).map_err(|e| e.to_string())?;
        }
        self.configured_views.clear();
        self.view_images.clear();
        Ok(())
    }
    /// Detach only GPU view identities when replacing a CPU screen outside a
    /// frame callback. The application retires these on its next registry borrow.
    /// No old screen, source pixels or text layouts need to wait for that frame.
    pub fn detach_game_views(&mut self) -> Vec<ViewId> {
        self.invalidate_world_input();
        self.configured_views.clear();
        self.view_images.clear();
        self.view_ids.drain().map(|(_, id)| id).collect()
    }
    pub fn view_images(&self) -> &FxHashMap<Uuid, ViewImage> {
        &self.view_images
    }
    pub fn source_builds(&self) -> u64 {
        self.sources.builds()
    }
    pub fn source_bytes(&self) -> usize {
        self.sources.resident_bytes()
    }
}
fn ordered<K: Ord + Copy, V>(map: &FxHashMap<K, V>) -> Vec<K> {
    let mut keys = map.keys().copied().collect::<Vec<_>>();
    keys.sort_unstable();
    keys
}

/// Interactive controls must intersect the same root clip as their GPU paint.
fn visible_control_rect(elements: &ScreenElements, r: Rect) -> bool {
    [r.x, r.y, r.width, r.height].iter().all(|v| v.is_finite())
        && r.width > 0.
        && r.height > 0.
        && rect(r)
            .intersection(Uv::new(
                0.,
                0.,
                elements.layout.viewport.x as f32,
                elements.layout.viewport.y as f32,
            ))
            .width
            > 0.
        && rect(r)
            .intersection(Uv::new(
                0.,
                0.,
                elements.layout.viewport.x as f32,
                elements.layout.viewport.y as f32,
            ))
            .height
            > 0.
}
