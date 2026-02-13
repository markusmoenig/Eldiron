use crate::docks::data_undo::*;
use crate::editor::RUSTERIX;
use crate::prelude::*;
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
    ScreenWidget(Uuid, Uuid), // (screen_id, widget_id)
}

#[derive(Clone, Debug)]
struct CharacterPreviewRigging {
    animation: Option<String>,
    perspective: AvatarDirection,
    fixed_frame: usize,
    play: bool,
    speed: f32,
    slots: FxHashMap<String, String>,
}

pub struct DataDock {
    // Per-entity undo stacks
    entity_undos: FxHashMap<EntityKey, DataUndo>,
    current_entity: Option<EntityKey>,
    max_undo: usize,
    prev_state: Option<TheTextEditState>,
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
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut center = TheCanvas::new();

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
        }

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
        );

        buffer.fill(BLACK);
        let Some(out) = out else {
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
    fn parse_preview_rigging(toml_src: &str) -> CharacterPreviewRigging {
        let mut out = CharacterPreviewRigging {
            animation: None,
            perspective: AvatarDirection::Front,
            fixed_frame: 0,
            play: true,
            speed: 1.0,
            slots: FxHashMap::default(),
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

        if let Some(slots) = table.get("slots").and_then(toml::Value::as_table) {
            for (slot, value) in slots {
                if let Some(v) = value.as_str() {
                    out.slots.insert(slot.to_string(), v.to_string());
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
        project
            .items
            .values()
            .find(|item| item.name.eq_ignore_ascii_case(ident))
    }

    fn populate_preview_equipment(
        preview: &CharacterPreviewRigging,
        project: &Project,
        entity: &mut rusterix::Entity,
    ) {
        entity.equipped.clear();
        for (slot, item_ref) in &preview.slots {
            let Some(template) = Self::find_item_template(project, item_ref) else {
                continue;
            };
            let mut runtime_item = rusterix::Item::default();
            apply_item_data(&mut runtime_item, &template.data);
            runtime_item
                .attributes
                .set("slot", Value::Str(slot.to_string()));
            entity.equipped.insert(slot.to_string(), runtime_item);
        }
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
        }
    }
}
