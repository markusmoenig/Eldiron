use crate::editor::UNDOMANAGER;
use crate::prelude::*;
use crate::recipe_utils::{
    clear_recipe_preview_cache, recipe_catalog_fingerprint, recipe_description, recipe_name,
    render_recipe_preview, sync_recipe_compatibility_catalogs,
};
use std::time::Duration;

const RECIPE_LIST: &str = "Procedural Recipe List";
const RECIPE_SOURCE: &str = "Procedural Recipe Source";
const RECIPE_PREVIEW: &str = "Procedural Recipe Preview";
const RENDER_RECIPE_PREVIEW: &str = "Render Procedural Recipe Preview";
const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(250);

pub struct RecipesDock {
    selected: Option<Uuid>,
    list_fingerprint: Option<u64>,
}

impl Dock for RecipesDock {
    fn new() -> Self {
        Self {
            selected: None,
            list_fingerprint: None,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        canvas.set_layout(TheListLayout::new(TheId::named(RECIPE_LIST)));

        let mut toolbar = TheCanvas::new();
        toolbar.set_widget(TheTraybar::new(TheId::empty()));
        let mut layout = TheHLayout::new(TheId::empty());
        layout.set_margin(Vec4::new(10, 2, 5, 2));
        let mut edit = TheTraybarButton::new(TheId::named("Edit Procedural Recipe"));
        edit.set_text(fl!("edit_recipe"));
        edit.set_status_text(&fl!("status_edit_recipe"));
        layout.add_widget(Box::new(edit));
        toolbar.set_layout(layout);
        canvas.set_top(toolbar);
        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.selected = match server_ctx.pc {
            ProjectContext::ProceduralRecipe(id) => Some(id),
            _ => self.selected,
        };
        self.refresh_list_if_needed(ui, ctx, project);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Minimap"),
            TheValue::Empty,
        ));
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::StateChanged(id, TheWidgetState::Selected)
                if id.name == "Procedural Recipe List Item" =>
            {
                self.selected = Some(id.uuid);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Select Procedural Recipe"),
                    TheValue::Id(id.uuid),
                ));
                true
            }
            TheEvent::StateChanged(id, _) if id.name == "Edit Procedural Recipe" => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Edit Procedural Recipe"),
                    TheValue::Empty,
                ));
                true
            }
            TheEvent::Custom(id, _) if id.name == "Refresh Recipe Tree" => {
                self.refresh_list_if_needed(ui, ctx, project);
                true
            }
            _ => false,
        }
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        _ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        let id = match server_ctx.pc {
            ProjectContext::ProceduralRecipe(id) => id,
            _ => return false,
        };
        draw_preview(buffer, project, id);
        true
    }

    fn supports_actions(&self) -> bool {
        false
    }

    fn reset_for_project_switch(&mut self) {
        self.selected = None;
        self.list_fingerprint = None;
        clear_recipe_preview_cache();
    }
}

impl RecipesDock {
    fn refresh_list_if_needed(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        let fingerprint = recipe_catalog_fingerprint(project);
        if self.list_fingerprint != Some(fingerprint) {
            self.refresh_list(ui, ctx, project);
            self.list_fingerprint = Some(fingerprint);
        }
        if let Some(id) = self.selected
            && let Some(list) = ui.get_list_layout(RECIPE_LIST)
        {
            list.select_item(id, ctx, false);
        }
    }

    fn refresh_list(&self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        let Some(list) = ui.get_list_layout(RECIPE_LIST) else {
            return;
        };
        list.clear();
        for (id, asset) in &project.procedural_recipes {
            let (name, kind) = recipe_description(&asset.source).unwrap_or_else(|_| {
                (
                    fl!("invalid_recipe"),
                    crate::recipe_utils::ProceduralRecipeKind::Tile,
                )
            });
            let mut item =
                TheListItem::new(TheId::named_with_id("Procedural Recipe List Item", *id));
            item.set_text(name);
            item.set_sub_text(format!(
                "{} · {}",
                crate::recipe_utils::localized_recipe_kind(kind),
                asset.alias
            ));
            item.set_size(58);
            if let Ok(preview) = render_recipe_preview(project, *id) {
                item.set_icon(preview.scaled(52, 52));
            }
            if self.selected == Some(*id) {
                item.set_state(TheWidgetState::Selected);
            }
            list.add_item(item, ctx);
        }
    }
}

pub struct RecipeEditorDock {
    selected: Option<Uuid>,
    preview: Option<TheRGBABuffer>,
    pending_previews: FxHashMap<Uuid, i32>,
    preview_generation: i32,
}

impl Dock for RecipeEditorDock {
    fn new() -> Self {
        Self {
            selected: None,
            preview: None,
            pending_previews: FxHashMap::default(),
            preview_generation: 0,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        let mut split = TheSharedHLayout::new(TheId::named("Procedural Recipe Editor Split"));
        split.set_shared_ratio(0.58);

        let mut source_canvas = TheCanvas::new();
        let mut source = TheTextAreaEdit::new(TheId::named(RECIPE_SOURCE));
        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme")
            && let Ok(theme) = std::str::from_utf8(bytes.data.as_ref())
        {
            source.add_theme_from_string(theme);
            source.set_code_theme("Gruvbox Dark");
        }
        source.set_continuous(true);
        source.display_line_number(true);
        source.use_global_statusbar(true);
        source.set_font_size(14.0);
        source_canvas.set_widget(source);

        let mut preview_canvas = TheCanvas::new();
        preview_canvas.set_layout(TheRGBALayout::new(TheId::named(RECIPE_PREVIEW)));
        split.add_canvas(source_canvas);
        split.add_canvas(preview_canvas);
        canvas.set_layout(split);

        let mut toolbar = TheCanvas::new();
        toolbar.set_widget(TheTraybar::new(TheId::empty()));
        let mut layout = TheHLayout::new(TheId::empty());
        layout.set_margin(Vec4::new(10, 2, 5, 2));
        let mut title = TheText::new(TheId::named("Procedural Recipe Editor Title"));
        title.set_text(fl!("recipe_editor"));
        layout.add_widget(Box::new(title));
        toolbar.set_layout(layout);
        canvas.set_top(toolbar);
        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        let ProjectContext::ProceduralRecipe(id) = server_ctx.pc else {
            return;
        };
        self.selected = Some(id);
        ctx.ui
            .send(TheEvent::SetStatusText(TheId::empty(), String::new()));
        if let Some(asset) = project.procedural_recipes.get(&id) {
            ui.set_widget_value(RECIPE_SOURCE, ctx, TheValue::Text(asset.source.clone()));
            ui.set_widget_value(
                "Procedural Recipe Editor Title",
                ctx,
                TheValue::Text(format!(
                    "{} — {}",
                    fl!("recipe_editor"),
                    recipe_name(&asset.source)
                )),
            );
        }
        self.refresh_preview(ui, ctx, project, id);
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let TheEvent::Custom(id, TheValue::Int(generation)) = event
            && id.name == RENDER_RECIPE_PREVIEW
            && self.pending_previews.get(&id.uuid) == Some(generation)
        {
            self.pending_previews.remove(&id.uuid);
            self.render_pending_preview(ui, ctx, project, id.uuid);
            return true;
        }

        let TheEvent::ValueChanged(id, value) = event else {
            return false;
        };
        if id.name != RECIPE_SOURCE {
            return false;
        }
        let Some(asset_id) = self.selected else {
            return false;
        };
        let Some(source) = value.to_string() else {
            return false;
        };
        let previous = project
            .procedural_recipes
            .get(&asset_id)
            .map(|asset| asset.source.clone())
            .unwrap_or_else(|| source.clone());
        if previous == source {
            return false;
        }
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::EditProceduralRecipe(asset_id, previous, source.clone()),
            ctx,
        );
        if let Some(asset) = project.procedural_recipes.get_mut(&asset_id) {
            asset.source = source;
        }
        match project
            .procedural_recipes
            .get(&asset_id)
            .map(|asset| recipe_description(&asset.source))
        {
            Some(Ok((name, _))) => {
                sync_recipe_compatibility_catalogs(project);
                self.schedule_preview(ctx, asset_id);
                ctx.ui
                    .send(TheEvent::SetStatusText(TheId::empty(), fl!("recipe_valid")));
                ui.set_widget_value(
                    "Procedural Recipe Editor Title",
                    ctx,
                    TheValue::Text(format!("{} — {name}", fl!("recipe_editor"))),
                );
            }
            Some(Err(error)) => {
                self.pending_previews.remove(&asset_id);
                ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
            }
            None => {}
        }
        true
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        _ctx: &mut TheContext,
        _server_ctx: &ServerContext,
    ) -> bool {
        let Some(id) = self.selected else {
            return false;
        };
        if let Some(preview) = &self.preview {
            draw_preview_buffer(buffer, preview);
        } else {
            draw_preview(buffer, project, id);
        }
        true
    }

    fn supports_actions(&self) -> bool {
        false
    }

    fn reset_for_project_switch(&mut self) {
        self.selected = None;
        self.preview = None;
        self.pending_previews.clear();
        self.preview_generation = 0;
    }
}

impl RecipeEditorDock {
    fn render_pending_preview(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        asset_id: Uuid,
    ) {
        match render_recipe_preview(project, asset_id) {
            Ok(preview) => {
                let _ = crate::recipe_utils::rebake_tile_recipe_with_preview(
                    project, asset_id, &preview,
                );
                if self.selected == Some(asset_id) {
                    self.set_preview(ui, ctx, preview);
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Refresh Recipe Tree"),
                    TheValue::Id(asset_id),
                ));
            }
            Err(error) => {
                ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
            }
        }
    }

    fn schedule_preview(&mut self, ctx: &mut TheContext, asset_id: Uuid) {
        self.preview_generation = self.preview_generation.wrapping_add(1);
        let generation = self.preview_generation;
        self.pending_previews.insert(asset_id, generation);
        let event = TheEvent::Custom(
            TheId::named_with_id(RENDER_RECIPE_PREVIEW, asset_id),
            TheValue::Int(generation),
        );

        #[cfg(not(target_arch = "wasm32"))]
        match ctx.ui.state_events_sender.clone() {
            Some(sender) => {
                std::thread::spawn(move || {
                    std::thread::sleep(PREVIEW_DEBOUNCE);
                    let _ = sender.send(event);
                });
            }
            None => ctx.ui.send(event),
        }

        #[cfg(target_arch = "wasm32")]
        ctx.ui.send(event);
    }

    fn refresh_preview(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        id: Uuid,
    ) {
        let Ok(buffer) = render_recipe_preview(project, id) else {
            self.preview = None;
            return;
        };
        self.set_preview(ui, ctx, buffer);
    }

    fn set_preview(&mut self, ui: &mut TheUI, ctx: &mut TheContext, buffer: TheRGBABuffer) {
        if let Some(layout) = ui.get_rgba_layout(RECIPE_PREVIEW)
            && let Some(view) = layout.rgba_view_mut().as_rgba_view()
        {
            view.set_mode(TheRGBAViewMode::Display);
            view.set_buffer(buffer.clone());
            layout.relayout(ctx);
        }
        self.preview = Some(buffer);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Minimap"),
            TheValue::Empty,
        ));
    }
}

fn draw_preview(buffer: &mut TheRGBABuffer, project: &Project, id: Uuid) {
    buffer.fill(BLACK);
    let Ok(preview) = render_recipe_preview(project, id) else {
        return;
    };
    draw_preview_buffer(buffer, &preview);
}

fn draw_preview_buffer(buffer: &mut TheRGBABuffer, preview: &TheRGBABuffer) {
    buffer.fill(BLACK);
    let size = buffer.dim().width.min(buffer.dim().height).max(1);
    let preview = preview.scaled(size, size);
    let x = (buffer.dim().width - size) / 2;
    let y = (buffer.dim().height - size) / 2;
    buffer.copy_into(x, y, &preview);
}
