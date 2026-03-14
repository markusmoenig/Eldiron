use crate::docks::data_undo::*;
use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::PixelSource;
use rusterix::avatar_builder::AvatarRuntimeBuilder;
use rusterix::server::data::{apply_entity_data, apply_item_data};
use theframework::prelude::*;
use theframework::theui::thewidget::thetextedit::TheTextEditState;
use toml::Table;

/// Unique identifier for entities being edited
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKey {
    RegionSettings(Uuid),
    Character(Uuid),
    CharacterPreviewRigging(Uuid),
    Item(Uuid),
    ProjectSettings,
    GameRules,
    GameLocales,
    GameAudioFx,
    ScreenWidget(Uuid, Uuid), // (screen_id, widget_id)
}

#[derive(Clone, Debug)]
struct CharacterPreviewRigging {
    animation: Option<String>,
    perspective: AvatarDirection,
    fixed_frame: usize,
    play: bool,
    speed: f32,
    debug: bool,
    slots: FxHashMap<String, String>,
    slot_overrides: FxHashMap<String, CharacterPreviewSlotOverride>,
    attrs: FxHashMap<String, Value>,
}

#[derive(Clone, Debug, Default)]
struct CharacterPreviewSlotOverride {
    rig_scale: Option<f32>,
    rig_pivot: Option<[f32; 2]>,
    rig_layer: Option<String>,
}

pub struct DataDock {
    // Per-entity undo stacks
    entity_undos: FxHashMap<EntityKey, DataUndo>,
    current_entity: Option<EntityKey>,
    max_undo: usize,
    prev_state: Option<TheTextEditState>,
    validation_signatures: FxHashMap<EntityKey, String>,
}

impl Dock for DataDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            entity_undos: FxHashMap::default(),
            current_entity: None,
            max_undo: 30,
            prev_state: None,
            validation_signatures: FxHashMap::default(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut center = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let mut play = TheTraybarButton::new(TheId::named("Audio FX Preview Play"));
        play.set_text("Play".to_string());
        play.set_status_text("Preview the audio effect under the cursor");
        toolbar_hlayout.add_widget(Box::new(play));

        toolbar_canvas.set_layout(toolbar_hlayout);
        center.set_top(toolbar_canvas);

        let mut textedit = TheTextAreaEdit::new(TheId::named("DockDataEditor"));
        if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_syntax_from_string(source);
                textedit.set_code_type("TOML");
            }
        }

        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                textedit.add_theme_from_string(source);
                textedit.set_code_theme("Gruvbox Dark");
            }
        }

        textedit.set_continuous(true);
        textedit.display_line_number(true);
        // textedit.set_code_theme("base16-eighties.dark");
        textedit.use_global_statusbar(true);
        textedit.set_font_size(14.0);
        // Tell the widget we handle undo/redo manually here
        textedit.set_supports_undo(false);
        center.set_widget(textedit);

        center
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(id) = server_ctx.pc.id() {
            if server_ctx.pc.is_region() {
                if let Some(region) = project.get_region(&id) {
                    ui.set_widget_value(
                        "DockDataEditor",
                        ctx,
                        TheValue::Text(region.config.clone()),
                    );
                    // Switch to this entity's undo stack
                    self.switch_to_entity(EntityKey::RegionSettings(id), ctx);
                }
            } else if server_ctx.pc.is_character() {
                if let Some(character) = project.characters.get(&id) {
                    match server_ctx.pc {
                        ProjectContext::CharacterPreviewRigging(_) => {
                            ui.set_widget_value(
                                "DockDataEditor",
                                ctx,
                                TheValue::Text(character.preview_rigging.clone()),
                            );
                            self.switch_to_entity(EntityKey::CharacterPreviewRigging(id), ctx);
                        }
                        _ => {
                            ui.set_widget_value(
                                "DockDataEditor",
                                ctx,
                                TheValue::Text(character.data.clone()),
                            );
                            self.switch_to_entity(EntityKey::Character(id), ctx);
                        }
                    }
                }
            } else if server_ctx.pc.is_item() {
                if let Some(item) = project.items.get(&id) {
                    ui.set_widget_value("DockDataEditor", ctx, TheValue::Text(item.data.clone()));
                    // Switch to this entity's undo stack
                    self.switch_to_entity(EntityKey::Item(id), ctx);
                }
            } else if let ProjectContext::ScreenWidget(screen_id, widget_id) = server_ctx.pc {
                if let Some(screen) = project.screens.get(&screen_id) {
                    for sector in &screen.map.sectors {
                        if sector.creator_id == widget_id {
                            let data = sector.properties.get_str_default("data", "".into());
                            ui.set_widget_value("DockDataEditor", ctx, TheValue::Text(data));
                            // Switch to this entity's undo stack
                            self.switch_to_entity(
                                EntityKey::ScreenWidget(screen_id, widget_id),
                                ctx,
                            );
                            break;
                        }
                    }
                }
            }
        } else if server_ctx.pc.is_project_settings() {
            ui.set_widget_value(
                "DockDataEditor",
                ctx,
                TheValue::Text(project.config.clone()),
            );
            // Switch to this entity's undo stack
            self.switch_to_entity(EntityKey::ProjectSettings, ctx);
        } else if server_ctx.pc.is_game_rules() {
            ui.set_widget_value("DockDataEditor", ctx, TheValue::Text(project.rules.clone()));
            self.switch_to_entity(EntityKey::GameRules, ctx);
        } else if server_ctx.pc.is_game_locales() {
            ui.set_widget_value(
                "DockDataEditor",
                ctx,
                TheValue::Text(project.locales.clone()),
            );
            self.switch_to_entity(EntityKey::GameLocales, ctx);
        } else if server_ctx.pc.is_game_audio_fx() {
            ui.set_widget_value(
                "DockDataEditor",
                ctx,
                TheValue::Text(project.audio_fx.clone()),
            );
            self.switch_to_entity(EntityKey::GameAudioFx, ctx);
        }

        self.sync_audio_fx_toolbar(ctx, server_ctx);
        self.validate_project_documents(project);

        // Store initial state for undo
        if let Some(edit) = ui.get_text_area_edit("DockDataEditor") {
            self.prev_state = Some(edit.get_state());
        }
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::ValueChanged(id, value) => {
                if id.name == "DockDataEditor" {
                    if let Some(edit) = ui.get_text_area_edit("DockDataEditor") {
                        // Add undo atom before applying the change
                        if let Some(prev) = &self.prev_state {
                            let current_state = edit.get_state();
                            let atom = DataUndoAtom::TextEdit(prev.clone(), current_state.clone());
                            self.add_undo(atom, ctx);
                            self.prev_state = Some(current_state);
                        }
                    }

                    if let Some(id) = server_ctx.pc.id() {
                        if server_ctx.pc.is_region() {
                            if let Some(code) = value.to_string() {
                                if let Some(region) = project.get_region_mut(&id) {
                                    region.config = code;
                                    redraw = true;
                                }
                            }
                            if let Ok(changed) =
                                crate::utils::update_region_settings(project, server_ctx)
                            {
                                if changed {
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Update Minimap"),
                                        TheValue::Empty,
                                    ));

                                    RUSTERIX.write().unwrap().set_dirty();

                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Render SceneManager Map"),
                                        TheValue::Empty,
                                    ));
                                }
                            }
                        } else if server_ctx.pc.is_character() {
                            if let Some(code) = value.to_string() {
                                if let Some(character) = project.characters.get_mut(&id) {
                                    match server_ctx.pc {
                                        ProjectContext::CharacterPreviewRigging(_) => {
                                            character.preview_rigging = code;
                                            ctx.ui.send(TheEvent::Custom(
                                                TheId::named("Update Minimap"),
                                                TheValue::Empty,
                                            ));
                                        }
                                        _ => character.data = code,
                                    }
                                    redraw = true;
                                }
                            }
                        } else if server_ctx.pc.is_item() {
                            if let Some(code) = value.to_string() {
                                if let Some(item) = project.items.get_mut(&id) {
                                    item.data = code;
                                    redraw = true;
                                }
                            }
                        } else if let ProjectContext::ScreenWidget(screen_id, widget_id) =
                            server_ctx.pc
                        {
                            if let Some(code) = value.to_string() {
                                if let Some(screen) = project.screens.get_mut(&screen_id) {
                                    for sector in &mut screen.map.sectors {
                                        if sector.creator_id == widget_id {
                                            sector.properties.set("data".into(), Value::Str(code));
                                            redraw = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    } else if server_ctx.pc.is_project_settings() {
                        if let Some(code) = value.to_string() {
                            _ = RUSTERIX.write().unwrap().scene_handler.settings.read(&code);
                            project.config = code;
                            redraw = true;
                        }
                    } else if server_ctx.pc.is_game_rules() {
                        if let Some(code) = value.to_string() {
                            project.rules = code;
                            redraw = true;
                        }
                    } else if server_ctx.pc.is_game_locales() {
                        if let Some(code) = value.to_string() {
                            project.locales = code;
                            redraw = true;
                        }
                    } else if server_ctx.pc.is_game_audio_fx() {
                        if let Some(code) = value.to_string() {
                            project.audio_fx = code;
                            redraw = true;
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.assets.audio_fx_src = project.audio_fx.clone();
                            rusterix.load_audio_assets();
                        }
                    }

                    self.validate_project_documents(project);
                }
            }
            TheEvent::StateChanged(id, state) => {
                if *state == TheWidgetState::Clicked {
                    if id.name == "Audio FX Preview Play" {
                        self.preview_audio_fx(ui, project);
                    }
                }
            }
            _ => {}
        }
        redraw
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        let ProjectContext::CharacterPreviewRigging(character_id) = server_ctx.pc else {
            return false;
        };
        let Some(character) = project.characters.get(&character_id) else {
            return false;
        };

        let mut entity = rusterix::Entity::default();
        apply_entity_data(&mut entity, &character.data);

        let preview = Self::parse_preview_rigging(&character.preview_rigging);
        if preview.debug {
            eprintln!(
                "[RIGPREVIEW] active char={} anim='{}' perspective={:?} play={} speed={} slots={} overrides={} attrs={}",
                character_id,
                preview.animation.as_deref().unwrap_or("<first>"),
                preview.perspective,
                preview.play,
                preview.speed,
                preview.slots.len(),
                preview.slot_overrides.len(),
                preview.attrs.len()
            );
        }
        Self::populate_preview_equipment(&preview, project, &mut entity);

        let Some(avatar) = Self::find_preview_avatar(&entity, project) else {
            buffer.fill(BLACK);
            return true;
        };

        let frame_index = Self::preview_frame_index(avatar, &preview, server_ctx.animation_counter);
        let mut assets = rusterix::Assets::new();
        assets.palette = project.palette.clone();
        assets.tiles = project.tiles.clone();

        let out = AvatarRuntimeBuilder::build_preview_for_entity(
            &entity,
            avatar,
            &assets,
            preview.animation.as_deref(),
            preview.perspective,
            frame_index,
            rusterix::AvatarShadingOptions::default(),
        );

        buffer.fill(BLACK);
        let Some(out) = out else {
            if preview.debug {
                eprintln!(
                    "[RIGPREVIEW] build failed anim='{}' perspective={:?} frame={}",
                    preview.animation.as_deref().unwrap_or("<first>"),
                    preview.perspective,
                    frame_index
                );
            }
            return true;
        };

        let src_w = out.size as usize;
        let src_h = out.size as usize;
        if src_w == 0 || src_h == 0 {
            return true;
        }

        let dim = buffer.dim();
        let dst_w = dim.width as f32;
        let dst_h = dim.height as f32;
        let scale = (dst_w / src_w as f32).min(dst_h / src_h as f32);
        let draw_w = (src_w as f32 * scale).round().max(1.0) as usize;
        let draw_h = (src_h as f32 * scale).round().max(1.0) as usize;
        let offset_x = ((dst_w as usize).saturating_sub(draw_w)) / 2;
        let offset_y = ((dst_h as usize).saturating_sub(draw_h)) / 2;
        let dst_rect = (offset_x, offset_y, draw_w, draw_h);

        let stride = buffer.stride();
        ctx.draw.blend_scale_chunk(
            buffer.pixels_mut(),
            &dst_rect,
            stride,
            &out.rgba,
            &(src_w, src_h),
        );

        true
    }

    fn supports_minimap_animation(&self) -> bool {
        true
    }

    fn supports_undo(&self) -> bool {
        true
    }

    fn has_changes(&self) -> bool {
        // Check if any entity has changes (index >= 0, meaning not fully undone)
        self.entity_undos.values().any(|undo| undo.has_changes())
    }

    fn mark_saved(&mut self) {
        for undo in self.entity_undos.values_mut() {
            undo.index = -1;
        }
    }

    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(entity_key) = self.current_entity {
            if let Some(undo) = self.entity_undos.get_mut(&entity_key) {
                if let Some(edit) = ui.get_text_area_edit("DockDataEditor") {
                    undo.undo(edit);
                    self.prev_state = Some(edit.get_state());
                    self.set_undo_state_to_ui(ctx);

                    // Update the project with the undone text
                    self.update_project_data(ui, ctx, project, server_ctx);
                }
            }
        }
    }

    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(entity_key) = self.current_entity {
            if let Some(undo) = self.entity_undos.get_mut(&entity_key) {
                if let Some(edit) = ui.get_text_area_edit("DockDataEditor") {
                    undo.redo(edit);
                    self.prev_state = Some(edit.get_state());
                    self.set_undo_state_to_ui(ctx);

                    // Update the project with the redone text
                    self.update_project_data(ui, ctx, project, server_ctx);
                }
            }
        }
    }

    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if let Some(entity_key) = self.current_entity {
            if let Some(undo) = self.entity_undos.get(&entity_key) {
                if undo.has_undo() {
                    ctx.ui.set_enabled("Undo");
                } else {
                    ctx.ui.set_disabled("Undo");
                }

                if undo.has_redo() {
                    ctx.ui.set_enabled("Redo");
                } else {
                    ctx.ui.set_disabled("Redo");
                }
                return;
            }
        }

        // No entity selected or no undo stack
        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");
    }
}

impl DataDock {
    fn sync_audio_fx_toolbar(&mut self, ctx: &mut TheContext, server_ctx: &ServerContext) {
        let active = server_ctx.pc.is_game_audio_fx();
        for id in ["Audio FX Preview Play"] {
            if active {
                ctx.ui.set_enabled(id);
            } else {
                ctx.ui.set_disabled(id);
            }
        }
    }

    fn preview_audio_fx(&mut self, ui: &mut TheUI, project: &Project) {
        let Some(effect_name) = self.current_audio_fx_name(ui) else {
            return;
        };
        let Ok(bytes) = rusterix::audio::synthesize_audio_fx_wav(&project.audio_fx, &effect_name)
        else {
            return;
        };

        let mut rusterix = RUSTERIX.write().unwrap();
        if rusterix.audio.is_none() {
            rusterix.audio = rusterix::AudioEngine::new().ok();
        }
        let Some(engine) = rusterix.audio.as_ref() else {
            return;
        };
        engine.clear_bus("preview");
        let clip_name = "__audio_fx_preview";
        let _ = engine.load_clip_from_bytes(clip_name, &bytes);
        let _ = engine.play_on_bus(clip_name, "preview", 1.0, false);
    }

    fn current_audio_fx_name(&self, ui: &mut TheUI) -> Option<String> {
        let edit = ui.get_text_area_edit("DockDataEditor")?;
        let state = edit.get_state();
        let row = state.cursor.row.min(state.rows.len().saturating_sub(1));

        for index in (0..=row).rev() {
            let line = state.rows.get(index)?.trim();
            if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                let section = section.trim();
                if let Some(name) = section.strip_prefix("sfx.") {
                    let name = name.trim();
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }
        }
        None
    }

    fn is_preview_slot_key(key: &str) -> bool {
        matches!(
            key.to_ascii_lowercase().as_str(),
            "main_hand"
                | "mainhand"
                | "weapon"
                | "weapon_main"
                | "hand_main"
                | "off_hand"
                | "offhand"
                | "weapon_off"
                | "hand_off"
                | "shield"
        )
    }

    fn parse_preview_rigging(toml_src: &str) -> CharacterPreviewRigging {
        let mut out = CharacterPreviewRigging {
            animation: None,
            perspective: AvatarDirection::Front,
            fixed_frame: 0,
            play: true,
            speed: 1.0,
            debug: false,
            slots: FxHashMap::default(),
            slot_overrides: FxHashMap::default(),
            attrs: FxHashMap::default(),
        };

        let Ok(table) = toml_src.parse::<Table>() else {
            return out;
        };

        out.animation = table
            .get("animation")
            .and_then(toml::Value::as_str)
            .map(ToString::to_string);
        if let Some(dir) = table.get("perspective").and_then(toml::Value::as_str) {
            out.perspective = match dir.to_ascii_lowercase().as_str() {
                "back" => AvatarDirection::Back,
                "left" => AvatarDirection::Left,
                "right" => AvatarDirection::Right,
                _ => AvatarDirection::Front,
            };
        }
        out.fixed_frame = table
            .get("frame")
            .and_then(toml::Value::as_integer)
            .unwrap_or(0)
            .max(0) as usize;
        out.play = table
            .get("play")
            .and_then(toml::Value::as_bool)
            .unwrap_or(true);
        out.speed = table
            .get("speed")
            .and_then(toml::Value::as_float)
            .unwrap_or(1.0)
            .max(0.01) as f32;
        out.debug = table
            .get("debug")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);

        // Top-level preview attributes (e.g. torso_index = 2)
        for (key, value) in &table {
            if matches!(
                key.as_str(),
                "animation"
                    | "perspective"
                    | "frame"
                    | "play"
                    | "speed"
                    | "debug"
                    | "slots"
                    | "slot_overrides"
            ) {
                continue;
            }
            if Self::is_preview_slot_key(key)
                && let Some(item_ref) = value.as_str()
            {
                out.slots.insert(key.to_string(), item_ref.to_string());
                continue;
            }
            if let Some(v) = Self::toml_to_attr_value(value) {
                out.attrs.insert(key.to_string(), v);
            }
        }

        if let Some(slots) = table.get("slots").and_then(toml::Value::as_table) {
            for (slot, value) in slots {
                if let Some(v) = value.as_str() {
                    out.slots.insert(slot.to_string(), v.to_string());
                } else if let Some(v) = Self::toml_to_attr_value(value) {
                    // Allow preview color/index overrides under [slots] for backward compatibility.
                    out.attrs.insert(slot.to_string(), v);
                }
            }
        }

        if let Some(overrides) = table.get("slot_overrides").and_then(toml::Value::as_table) {
            for (slot, value) in overrides {
                let Some(slot_table) = value.as_table() else {
                    continue;
                };
                let mut slot_override = CharacterPreviewSlotOverride::default();
                if let Some(scale) = slot_table.get("rig_scale").and_then(toml::Value::as_float) {
                    slot_override.rig_scale = Some(scale as f32);
                }
                if let Some(pivot) = slot_table.get("rig_pivot").and_then(toml::Value::as_array)
                    && pivot.len() == 2
                    && let (Some(x), Some(y)) = (pivot[0].as_float(), pivot[1].as_float())
                {
                    slot_override.rig_pivot = Some([x as f32, y as f32]);
                }
                if let Some(layer) = slot_table.get("rig_layer").and_then(toml::Value::as_str) {
                    slot_override.rig_layer = Some(layer.to_string());
                }
                if slot_override.rig_scale.is_some()
                    || slot_override.rig_pivot.is_some()
                    || slot_override.rig_layer.is_some()
                {
                    out.slot_overrides.insert(slot.to_string(), slot_override);
                }
            }
        }

        out
    }

    fn find_preview_avatar<'a>(
        entity: &rusterix::Entity,
        project: &'a Project,
    ) -> Option<&'a Avatar> {
        if let Some(avatar_id) = entity.attributes.get_id("avatar_id")
            && let Some(avatar) = project.avatars.get(&avatar_id)
        {
            return Some(avatar);
        }
        if let Some(name) = entity.attributes.get_str("avatar") {
            for avatar in project.avatars.values() {
                if avatar.name.eq_ignore_ascii_case(name) {
                    return Some(avatar);
                }
            }
        }
        project.avatars.values().next()
    }

    fn find_item_template<'a>(project: &'a Project, ident: &str) -> Option<&'a Item> {
        project.items.values().find(|item| {
            if item.name.eq_ignore_ascii_case(ident) {
                return true;
            }

            let mut parsed = rusterix::Item::default();
            apply_item_data(&mut parsed, &item.data);
            if parsed
                .attributes
                .get_str("name")
                .map(|name| name.eq_ignore_ascii_case(ident))
                .unwrap_or(false)
            {
                return true;
            }

            // Also support top-level item TOML names in preview lookup.
            if let Ok(table) = item.data.parse::<Table>() {
                return table
                    .get("name")
                    .and_then(toml::Value::as_str)
                    .map(|name| name.eq_ignore_ascii_case(ident))
                    .unwrap_or(false);
            }
            false
        })
    }

    fn apply_preview_item_top_level(item: &mut rusterix::Item, toml_src: &str) {
        let Ok(table) = toml_src.parse::<Table>() else {
            return;
        };
        for key in [
            "tile_id",
            "tile_id_front",
            "tile_id_back",
            "tile_id_left",
            "tile_id_right",
        ] {
            if let Some(id) = table.get(key).and_then(toml::Value::as_str)
                && let Ok(uuid) = Uuid::parse_str(id)
            {
                item.attributes
                    .set(key, Value::Source(PixelSource::TileId(uuid)));
            }
        }
        if let Some(scale) = table.get("rig_scale").and_then(toml::Value::as_float) {
            item.attributes.set("rig_scale", Value::Float(scale as f32));
        }
        if let Some(pivot) = table.get("rig_pivot").and_then(toml::Value::as_array)
            && pivot.len() == 2
            && let (Some(x), Some(y)) = (pivot[0].as_float(), pivot[1].as_float())
        {
            item.attributes
                .set("rig_pivot", Value::Vec2([x as f32, y as f32]));
        }
        if let Some(slot) = table.get("slot").and_then(toml::Value::as_str) {
            item.attributes.set("slot", Value::Str(slot.to_string()));
        }
        if let Some(layer) = table.get("rig_layer").and_then(toml::Value::as_str) {
            item.attributes
                .set("rig_layer", Value::Str(layer.to_string()));
        }
    }

    fn populate_preview_equipment(
        preview: &CharacterPreviewRigging,
        project: &Project,
        entity: &mut rusterix::Entity,
    ) {
        entity.equipped.clear();
        entity
            .attributes
            .set("avatar_preview_debug", Value::Bool(preview.debug));
        for (key, value) in &preview.attrs {
            entity.attributes.set(key, value.clone());
        }
        for (slot, item_ref) in &preview.slots {
            let Some(template) = Self::find_item_template(project, item_ref) else {
                if preview.debug {
                    eprintln!(
                        "[RIGPREVIEW] slot='{}' item='{}' -> NOT FOUND",
                        slot, item_ref
                    );
                }
                continue;
            };
            let mut runtime_item = rusterix::Item::default();
            apply_item_data(&mut runtime_item, &template.data);
            Self::apply_preview_item_top_level(&mut runtime_item, &template.data);
            runtime_item
                .attributes
                .set("slot", Value::Str(slot.to_string()));
            if let Some(override_cfg) = preview.slot_overrides.get(slot) {
                if let Some(scale) = override_cfg.rig_scale {
                    runtime_item
                        .attributes
                        .set("rig_scale", Value::Float(scale.max(0.01)));
                }
                if let Some(pivot) = override_cfg.rig_pivot {
                    runtime_item.attributes.set("rig_pivot", Value::Vec2(pivot));
                }
                if let Some(layer) = &override_cfg.rig_layer {
                    runtime_item
                        .attributes
                        .set("rig_layer", Value::Str(layer.clone()));
                }
            }
            if preview.debug {
                let has_tile = runtime_item
                    .attributes
                    .get_source("source")
                    .or_else(|| runtime_item.attributes.get_source("tile_id"))
                    .or_else(|| runtime_item.attributes.get_source("tile_id_front"))
                    .or_else(|| runtime_item.attributes.get_source("tile_id_back"))
                    .or_else(|| runtime_item.attributes.get_source("tile_id_left"))
                    .or_else(|| runtime_item.attributes.get_source("tile_id_right"))
                    .is_some();
                eprintln!(
                    "[RIGPREVIEW] slot='{}' item='{}' -> FOUND name='{}' tile={} override_scale={:?} override_pivot={:?} override_layer={:?}",
                    slot,
                    item_ref,
                    template.name,
                    has_tile,
                    preview.slot_overrides.get(slot).and_then(|o| o.rig_scale),
                    preview.slot_overrides.get(slot).and_then(|o| o.rig_pivot),
                    preview
                        .slot_overrides
                        .get(slot)
                        .and_then(|o| o.rig_layer.clone())
                );
            }
            entity.equipped.insert(slot.to_string(), runtime_item);
        }
    }

    fn toml_to_attr_value(value: &toml::Value) -> Option<Value> {
        if let Some(v) = value.as_integer() {
            return Some(Value::Int(v as i32));
        }
        if let Some(v) = value.as_float() {
            return Some(Value::Float(v as f32));
        }
        if let Some(v) = value.as_bool() {
            return Some(Value::Bool(v));
        }
        if let Some(v) = value.as_str() {
            return Some(Value::Str(v.to_string()));
        }
        None
    }

    fn preview_frame_index(
        avatar: &Avatar,
        preview: &CharacterPreviewRigging,
        animation_counter: usize,
    ) -> usize {
        let Some(anim) = preview
            .animation
            .as_deref()
            .and_then(|name| {
                avatar
                    .animations
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case(name))
            })
            .or_else(|| avatar.animations.first())
        else {
            return preview.fixed_frame;
        };
        let frame_count = anim
            .perspectives
            .iter()
            .find(|p| p.direction == preview.perspective)
            .or_else(|| {
                anim.perspectives
                    .iter()
                    .find(|p| p.direction == AvatarDirection::Front)
            })
            .or_else(|| anim.perspectives.first())
            .map(|p| p.frames.len().max(1))
            .unwrap_or(1);

        if preview.play {
            ((animation_counter as f32 / preview.speed).floor() as usize) % frame_count
        } else {
            preview.fixed_frame % frame_count
        }
    }

    /// Switch to a different entity and update undo button states
    fn switch_to_entity(&mut self, entity_key: EntityKey, ctx: &mut TheContext) {
        self.current_entity = Some(entity_key);
        self.set_undo_state_to_ui(ctx);
    }

    fn validate_project_documents(&mut self, project: &Project) {
        let Some(entity_key) = self.current_entity else {
            return;
        };

        if !matches!(
            entity_key,
            EntityKey::GameRules | EntityKey::GameLocales | EntityKey::GameAudioFx
        ) {
            return;
        }

        let issues = Self::collect_project_validation_issues(project);
        let signature = issues.join("\n");
        let previous = self
            .validation_signatures
            .insert(entity_key, signature.clone())
            .unwrap_or_default();

        if signature == previous || issues.is_empty() {
            return;
        }

        let label = match entity_key {
            EntityKey::GameRules => "Game / Rules",
            EntityKey::GameLocales => "Game / Locales",
            EntityKey::GameAudioFx => "Game / Audio FX",
            _ => return,
        };

        let mut chunk = format!("[Warning] {} validation\n", label);
        for issue in issues {
            chunk.push_str("- ");
            chunk.push_str(&issue);
            chunk.push('\n');
        }

        let mut rusterix = RUSTERIX.write().unwrap();
        rusterix.server.log.push_str(&chunk);
        rusterix.server.log_changed = true;
    }

    fn collect_project_validation_issues(project: &Project) -> Vec<String> {
        let mut issues = Vec::new();

        let locale_tables = match Self::parse_locale_tables(&project.locales) {
            Ok(locales) => locales,
            Err(err) => {
                issues.push(format!("Locales TOML parse error: {}", err));
                FxHashMap::default()
            }
        };

        let (audio_fx_names, audio_fx_issues) =
            Self::parse_audio_fx_names_and_issues(&project.audio_fx);
        issues.extend(audio_fx_issues);

        let asset_audio_names = project
            .assets
            .values()
            .filter(|asset| matches!(asset.buffer, AssetBuffer::Audio(_)))
            .map(|asset| asset.name.clone())
            .collect::<FxHashSet<_>>();

        match project.rules.parse::<Table>() {
            Ok(rules) => {
                let referenced_locale_keys = Self::rules_locale_keys(&rules);
                let referenced_audio_fx = Self::rules_audio_fx_refs(&rules);

                if locale_tables.is_empty() {
                    for key in &referenced_locale_keys {
                        issues.push(format!(
                            "Rules reference locale key '{}' but Game / Locales has no locale tables.",
                            key
                        ));
                    }
                } else {
                    for locale in locale_tables.keys() {
                        let keys = locale_tables.get(locale).unwrap();
                        for key in &referenced_locale_keys {
                            if !keys.contains(key) {
                                issues
                                    .push(format!("Locale '{}' is missing key '{}'.", locale, key));
                            }
                        }
                    }
                }

                for (path, name) in referenced_audio_fx {
                    if !audio_fx_names.contains(&name) && !asset_audio_names.contains(&name) {
                        issues.push(format!(
                            "Rules reference unknown audio '{}' at '{}'.",
                            name, path
                        ));
                    }
                }
            }
            Err(err) => issues.push(format!("Rules TOML parse error: {}", err)),
        }

        issues
    }

    fn parse_locale_tables(src: &str) -> Result<FxHashMap<String, FxHashSet<String>>, String> {
        let table = src.parse::<Table>().map_err(|err| err.to_string())?;
        let mut locales = FxHashMap::default();
        for (locale, value) in table {
            let Some(locale_table) = value.as_table() else {
                continue;
            };
            let mut keys = FxHashSet::default();
            Self::flatten_locale_keys("", locale_table, &mut keys);
            locales.insert(locale, keys);
        }
        Ok(locales)
    }

    fn flatten_locale_keys(prefix: &str, table: &Table, out: &mut FxHashSet<String>) {
        for (key, value) in table {
            let full = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };
            if let Some(nested) = value.as_table() {
                Self::flatten_locale_keys(&full, nested, out);
            } else {
                out.insert(full);
            }
        }
    }

    fn parse_audio_fx_names_and_issues(src: &str) -> (FxHashSet<String>, Vec<String>) {
        const ALLOWED_PARAMS: &[&str] = &[
            "wave",
            "duration",
            "attack",
            "decay",
            "sustain_level",
            "release",
            "gain",
            "freq",
            "freq_end",
            "noise",
            "lowpass",
            "repeat",
            "repeat_gap",
            "tremolo_depth",
            "tremolo_freq",
        ];
        const ALLOWED_WAVES: &[&str] = &["sine", "square", "saw", "triangle", "noise"];

        let table = match src.parse::<Table>() {
            Ok(table) => table,
            Err(err) => {
                return (
                    FxHashSet::default(),
                    vec![format!("Audio FX TOML parse error: {}", err)],
                );
            }
        };

        let mut names = FxHashSet::default();
        let mut issues = Vec::new();

        let Some(sfx) = table.get("sfx").and_then(toml::Value::as_table) else {
            return (names, issues);
        };

        for (name, value) in sfx {
            let Some(effect) = value.as_table() else {
                issues.push(format!("Audio FX section 'sfx.{}' must be a table.", name));
                continue;
            };
            names.insert(name.clone());

            for key in effect.keys() {
                if !ALLOWED_PARAMS.contains(&key.as_str()) {
                    issues.push(format!(
                        "Audio FX 'sfx.{}' uses unknown parameter '{}'.",
                        name, key
                    ));
                }
            }

            if let Some(wave) = effect.get("wave").and_then(toml::Value::as_str)
                && !ALLOWED_WAVES.contains(&wave)
            {
                issues.push(format!(
                    "Audio FX 'sfx.{}' uses unsupported wave '{}'.",
                    name, wave
                ));
            }
        }

        (names, issues)
    }

    fn rules_locale_keys(rules: &Table) -> Vec<String> {
        let mut keys = Vec::new();
        if let Some(messages) = rules
            .get("combat")
            .and_then(toml::Value::as_table)
            .and_then(|combat| combat.get("messages"))
            .and_then(toml::Value::as_table)
        {
            for key in ["incoming_key", "outgoing_key"] {
                if let Some(value) = messages
                    .get(key)
                    .and_then(toml::Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                {
                    keys.push(value.to_string());
                }
            }
        }
        keys
    }

    fn rules_audio_fx_refs(rules: &Table) -> Vec<(String, String)> {
        let mut refs = Vec::new();

        if let Some(audio) = rules
            .get("combat")
            .and_then(toml::Value::as_table)
            .and_then(|combat| combat.get("audio"))
            .and_then(toml::Value::as_table)
        {
            for key in ["incoming_fx", "outgoing_fx"] {
                if let Some(name) = audio
                    .get(key)
                    .and_then(toml::Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                {
                    refs.push((format!("combat.audio.{}", key), name.to_string()));
                }
            }
        }

        if let Some(kinds) = rules
            .get("combat")
            .and_then(toml::Value::as_table)
            .and_then(|combat| combat.get("kinds"))
            .and_then(toml::Value::as_table)
        {
            for (kind, value) in kinds {
                let Some(kind_audio) = value
                    .as_table()
                    .and_then(|kind_table| kind_table.get("audio"))
                    .and_then(toml::Value::as_table)
                else {
                    continue;
                };
                for key in ["incoming_fx", "outgoing_fx"] {
                    if let Some(name) = kind_audio
                        .get(key)
                        .and_then(toml::Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                    {
                        refs.push((
                            format!("combat.kinds.{}.audio.{}", kind, key),
                            name.to_string(),
                        ));
                    }
                }
            }
        }

        refs
    }

    /// Add an undo atom to the current entity's undo stack
    fn add_undo(&mut self, atom: DataUndoAtom, ctx: &mut TheContext) {
        if let Some(entity_key) = self.current_entity {
            let undo = self
                .entity_undos
                .entry(entity_key)
                .or_insert_with(DataUndo::new);
            undo.add(atom);
            undo.truncate_to_limit(self.max_undo);
            self.set_undo_state_to_ui(ctx);
        }
    }

    /// Update the project with the current text state
    fn update_project_data(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(id) = server_ctx.pc.id() {
            if let Some(edit) = ui.get_text_area_edit("DockDataEditor") {
                let state = edit.get_state();
                let text = state.rows.join("\n");

                if server_ctx.pc.is_region() {
                    if let Some(region) = project.get_region_mut(&id) {
                        region.config = text;
                        if let Ok(changed) =
                            crate::utils::update_region_settings(project, server_ctx)
                        {
                            if changed {
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Minimap"),
                                    TheValue::Empty,
                                ));

                                RUSTERIX.write().unwrap().set_dirty();

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Render SceneManager Map"),
                                    TheValue::Empty,
                                ));
                            }
                        }
                    }
                } else if server_ctx.pc.is_character() {
                    if let Some(character) = project.characters.get_mut(&id) {
                        match server_ctx.pc {
                            ProjectContext::CharacterPreviewRigging(_) => {
                                character.preview_rigging = text;
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Minimap"),
                                    TheValue::Empty,
                                ));
                            }
                            _ => character.data = text,
                        }
                    }
                } else if server_ctx.pc.is_item() {
                    if let Some(item) = project.items.get_mut(&id) {
                        item.data = text;
                    }
                } else if let ProjectContext::ScreenWidget(screen_id, widget_id) = server_ctx.pc {
                    if let Some(screen) = project.screens.get_mut(&screen_id) {
                        for sector in &mut screen.map.sectors {
                            if sector.creator_id == widget_id {
                                sector.properties.set("data".into(), Value::Str(text));
                                break;
                            }
                        }
                    }
                }
            }
        } else if server_ctx.pc.is_project_settings() {
            if let Some(edit) = ui.get_text_area_edit("DockDataEditor") {
                let state = edit.get_state();
                let text = state.rows.join("\n");
                _ = RUSTERIX.write().unwrap().scene_handler.settings.read(&text);
                project.config = text;
            }
        } else if server_ctx.pc.is_game_rules() {
            if let Some(edit) = ui.get_text_area_edit("DockDataEditor") {
                let state = edit.get_state();
                let text = state.rows.join("\n");
                project.rules = text;
            }
        } else if server_ctx.pc.is_game_locales() {
            if let Some(edit) = ui.get_text_area_edit("DockDataEditor") {
                let state = edit.get_state();
                let text = state.rows.join("\n");
                project.locales = text;
            }
        } else if server_ctx.pc.is_game_audio_fx() {
            if let Some(edit) = ui.get_text_area_edit("DockDataEditor") {
                let state = edit.get_state();
                let text = state.rows.join("\n");
                project.audio_fx = text;
            }
        }

        self.validate_project_documents(project);
    }
}
