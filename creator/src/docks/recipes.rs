use crate::docks::recipe_preview_3d::Recipe3DPreview;
use crate::editor::UNDOMANAGER;
use crate::prelude::*;
use crate::recipe_utils::{
    cache_recipe_preview_result, cached_recipe_visual_preview, clear_recipe_preview_cache,
    compact_recipe_diagnostic, recipe_description, recipe_name, render_recipe_preview_fresh,
    sync_recipe_compatibility_catalogs,
};
use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicI32, Ordering},
};
use std::time::{Duration, Instant};

const RECIPE_SOURCE: &str = "Procedural Recipe Source";
const RENDER_RECIPE_PREVIEW: &str = "Render Procedural Recipe Preview";
pub(crate) const RECIPE_MINIMAP_PREVIEW: &str = "Set Recipe Minimap Preview";
pub(crate) const RECIPE_SOURCE_CHANGED: &str = "Recipe Source Revision Changed";
const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(250);

type RecipeRenderResult = Result<(TheRGBABuffer, TheRGBABuffer), String>;

#[cfg(not(target_arch = "wasm32"))]
static RECIPE_RENDER_RESULTS: LazyLock<Mutex<FxHashMap<(Uuid, i32), RecipeRenderResult>>> =
    LazyLock::new(|| Mutex::new(FxHashMap::default()));

pub struct RecipeEditorDock {
    selected: Option<Uuid>,
    preview: Option<TheRGBABuffer>,
    pending_previews: FxHashMap<Uuid, (i32, Instant)>,
    preview_generation: Arc<AtomicI32>,
    preview_3d: Mutex<Option<Recipe3DPreview>>,
}

impl Dock for RecipeEditorDock {
    fn new() -> Self {
        Self {
            selected: None,
            preview: None,
            pending_previews: FxHashMap::default(),
            preview_generation: Arc::new(AtomicI32::new(0)),
            preview_3d: Mutex::new(None),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();
        let mut source = TheTextAreaEdit::new(TheId::named(RECIPE_SOURCE));
        if let Some(bytes) = crate::Embedded::get("parser/recipe.sublime-syntax")
            && let Ok(syntax) = std::str::from_utf8(bytes.data.as_ref())
        {
            source.add_syntax_from_string(syntax);
            source.set_code_type("Eldiron Recipe");
        }
        if let Some(bytes) = crate::Embedded::get("parser/gruvbox-dark.tmTheme")
            && let Ok(theme) = std::str::from_utf8(bytes.data.as_ref())
        {
            source.add_theme_from_string(theme);
            source.set_code_theme("Gruvbox Dark");
        }
        source.set_continuous(true);
        source.display_line_number(true);
        source.set_font_size(14.0);
        canvas.set_widget(source);

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
        self.preview_generation.fetch_add(1, Ordering::Relaxed);
        self.pending_previews.clear();
        self.selected = Some(id);
        *self.preview_3d.lock().unwrap() = None;
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
        match event {
            TheEvent::Custom(id, _) if id.name == RECIPE_SOURCE_CHANGED => {
                if self.selected != Some(id.uuid) {
                    return false;
                }
                if let Some(asset) = project.procedural_recipes.get(&id.uuid) {
                    ui.set_widget_value(RECIPE_SOURCE, ctx, TheValue::Text(asset.source.clone()));
                }
                self.handle_source_revision(ui, ctx, project, id.uuid);
                return true;
            }
            TheEvent::Custom(id, TheValue::Int(generation)) if id.name == RENDER_RECIPE_PREVIEW => {
                #[cfg(not(target_arch = "wasm32"))]
                let completed = RECIPE_RENDER_RESULTS
                    .lock()
                    .unwrap()
                    .remove(&(id.uuid, *generation));
                let is_current = self
                    .pending_previews
                    .get(&id.uuid)
                    .is_some_and(|(pending_generation, _)| pending_generation == generation);
                if !is_current {
                    return false;
                }
                self.pending_previews.remove(&id.uuid);

                #[cfg(not(target_arch = "wasm32"))]
                if let Some(completed) = completed {
                    self.apply_preview_result(ui, ctx, project, id.uuid, completed);
                }

                #[cfg(target_arch = "wasm32")]
                self.render_pending_preview(ui, ctx, project, id.uuid);
                return true;
            }
            #[cfg(target_arch = "wasm32")]
            TheEvent::Custom(id, _) if id.name == "Soft Update Minimap" => {
                if let Some(asset_id) =
                    self.pending_previews
                        .iter()
                        .find_map(|(asset_id, (_, deadline))| {
                            (Instant::now() >= *deadline).then_some(*asset_id)
                        })
                {
                    self.pending_previews.remove(&asset_id);
                    self.render_pending_preview(ui, ctx, project, asset_id);
                    return true;
                }
            }
            _ => {}
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
        self.handle_source_revision(ui, ctx, project, asset_id);
        true
    }

    fn poll_background(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pending = self
                .pending_previews
                .iter()
                .map(|(asset_id, (generation, _))| (*asset_id, *generation))
                .collect::<Vec<_>>();
            let completed = {
                let mut results = RECIPE_RENDER_RESULTS.lock().unwrap();
                pending
                    .into_iter()
                    .filter_map(|key| results.remove(&key).map(|result| (key, result)))
                    .collect::<Vec<_>>()
            };
            let mut applied = false;
            for ((asset_id, generation), result) in completed {
                let is_current = self
                    .pending_previews
                    .get(&asset_id)
                    .is_some_and(|(pending_generation, _)| *pending_generation == generation);
                if !is_current {
                    continue;
                }
                self.pending_previews.remove(&asset_id);
                self.apply_preview_result(ui, ctx, project, asset_id, result);
                applied = true;
            }
            applied
        }

        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        _project: &Project,
        _ctx: &mut TheContext,
        _server_ctx: &ServerContext,
    ) -> bool {
        if self.selected.is_none() {
            return false;
        }
        if let Some(preview) = &self.preview {
            if let Some(preview_3d) = self.preview_3d.lock().unwrap().as_mut() {
                preview_3d.draw(buffer);
            } else {
                draw_preview_buffer(buffer, preview);
            }
        } else {
            buffer.fill(BLACK);
        }
        true
    }

    fn supports_actions(&self) -> bool {
        false
    }

    fn supports_minimap_animation(&self) -> bool {
        (cfg!(target_arch = "wasm32") && !self.pending_previews.is_empty())
            || self
                .preview_3d
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(Recipe3DPreview::is_animated)
    }

    fn reset_for_project_switch(&mut self) {
        self.selected = None;
        self.preview = None;
        *self.preview_3d.lock().unwrap() = None;
        self.pending_previews.clear();
        self.preview_generation.store(0, Ordering::Relaxed);
        clear_recipe_preview_cache();
    }
}

impl RecipeEditorDock {
    fn handle_source_revision(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        asset_id: Uuid,
    ) {
        let Some(source) = project
            .procedural_recipes
            .get(&asset_id)
            .map(|asset| asset.source.clone())
        else {
            return;
        };
        match recipe_description(&source) {
            Ok((name, _)) => {
                sync_recipe_compatibility_catalogs(project);
                self.schedule_preview(ctx, project, asset_id);
                ctx.ui
                    .send(TheEvent::SetStatusText(TheId::empty(), String::new()));
                ui.set_widget_value(
                    "Procedural Recipe Editor Title",
                    ctx,
                    TheValue::Text(format!("{} — {name}", fl!("recipe_editor"))),
                );
            }
            Err(error) => {
                self.preview_generation.fetch_add(1, Ordering::Relaxed);
                self.pending_previews.remove(&asset_id);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    compact_recipe_diagnostic(&error),
                ));
            }
        }
    }

    fn apply_preview_result(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        asset_id: Uuid,
        result: RecipeRenderResult,
    ) {
        match result {
            Ok((baked_preview, visual_preview)) => {
                let _ = cache_recipe_preview_result(
                    project,
                    asset_id,
                    baked_preview.clone(),
                    visual_preview.clone(),
                );
                let rebake_result = crate::recipe_utils::rebake_tile_recipe_with_preview(
                    project,
                    asset_id,
                    &baked_preview,
                );
                if self.selected == Some(asset_id) {
                    let preview_result = (|| -> Result<(), String> {
                        rebake_result?;
                        let mut preview_slot = self.preview_3d.lock().unwrap();
                        if let Some(preview) = preview_slot.as_mut() {
                            if !preview.rebuild(project, asset_id)? {
                                *preview_slot = None;
                            }
                        } else {
                            *preview_slot = Recipe3DPreview::from_project(project, asset_id)?;
                        }
                        Ok(())
                    })();
                    let preview_ready = preview_result.is_ok();
                    if let Err(error) = preview_result {
                        *self.preview_3d.lock().unwrap() = None;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            compact_recipe_diagnostic(&error),
                        ));
                    }
                    self.set_preview(ui, ctx, visual_preview);
                    if preview_ready {
                        ctx.ui
                            .send(TheEvent::SetStatusText(TheId::empty(), fl!("recipe_valid")));
                    }
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Refresh Recipe Tree"),
                    TheValue::Id(asset_id),
                ));
            }
            Err(error) => {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    compact_recipe_diagnostic(&error),
                ));
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn render_pending_preview(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        asset_id: Uuid,
    ) {
        let result = render_recipe_preview_fresh(project, asset_id);
        self.apply_preview_result(ui, ctx, project, asset_id, result);
    }

    fn schedule_preview(&mut self, ctx: &mut TheContext, project: &Project, asset_id: Uuid) {
        let generation = self
            .preview_generation
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_add(1);
        self.pending_previews
            .insert(asset_id, (generation, Instant::now() + PREVIEW_DEBOUNCE));
        let event = TheEvent::Custom(
            TheId::named_with_id(RENDER_RECIPE_PREVIEW, asset_id),
            TheValue::Int(generation),
        );

        #[cfg(not(target_arch = "wasm32"))]
        match ctx.ui.state_events_sender.clone() {
            Some(sender) => {
                // Clone only the inputs used by RecipeRenderer. Cloning the
                // full Stonefall Project here would copy its very large map.
                let mut preview_project = Project::new();
                preview_project.art_palette = project.art_palette.clone();
                preview_project.procedural_recipes = project.procedural_recipes.clone();
                preview_project.procedural_materials = project.procedural_materials.clone();
                preview_project.procedural_sdfs = project.procedural_sdfs.clone();
                let latest_generation = Arc::clone(&self.preview_generation);
                std::thread::spawn(move || {
                    std::thread::sleep(PREVIEW_DEBOUNCE);
                    if latest_generation.load(Ordering::Relaxed) != generation {
                        return;
                    }
                    let result = render_recipe_preview_fresh(&preview_project, asset_id);
                    if latest_generation.load(Ordering::Relaxed) != generation {
                        return;
                    }
                    RECIPE_RENDER_RESULTS
                        .lock()
                        .unwrap()
                        .insert((asset_id, generation), result);
                    let _ = sender.send(event);
                });
            }
            None => {
                ctx.ui.send(event);
            }
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
        if let Some(buffer) = cached_recipe_visual_preview(project, id) {
            self.set_preview(ui, ctx, buffer);
        } else {
            self.preview = None;
        }
        if project.procedural_recipes.contains_key(&id) {
            self.schedule_preview(ctx, project, id);
        }
    }

    fn set_preview(&mut self, ui: &mut TheUI, ctx: &mut TheContext, buffer: TheRGBABuffer) {
        self.preview = Some(buffer.clone());

        // There is exactly one MiniMap render view. Update it immediately so
        // the completed render cannot be lost behind another queued minimap
        // event. The custom event below remains as a fallback for the short
        // interval in which a relayout has detached the render view.
        if let Some(render_view) = ui.get_render_view("MiniMap") {
            let dim = *render_view.dim();
            let minimap = render_view.render_buffer_mut();
            if dim.is_valid() {
                minimap.resize(dim.width, dim.height);
                if let Some(preview_3d) = self.preview_3d.lock().unwrap().as_mut() {
                    preview_3d.draw(minimap);
                } else {
                    draw_preview_buffer(minimap, &buffer);
                }
            } else {
                *minimap = buffer.clone();
            }
            render_view.set_needs_redraw(true);
            ctx.ui.redraw_all = true;
        }

        if let Some(asset_id) = self.selected {
            if self.preview_3d.lock().unwrap().is_some() {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Minimap"),
                    TheValue::Empty,
                ));
            } else {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named_with_id(RECIPE_MINIMAP_PREVIEW, asset_id),
                    TheValue::Image(buffer),
                ));
            }
        }
    }
}

pub(crate) fn draw_preview_buffer(buffer: &mut TheRGBABuffer, preview: &TheRGBABuffer) {
    buffer.fill(BLACK);
    let size = buffer.dim().width.min(buffer.dim().height).max(1);
    let preview = preview.scaled(size, size);
    let x = (buffer.dim().width - size) / 2;
    let y = (buffer.dim().height - size) / 2;
    buffer.copy_into(x, y, &preview);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_recipe_syntax_loads_in_creator_highlighter() {
        let mut highlighter = TheCodeHighlighter::default();
        highlighter
            .add_syntax_from_string(include_str!("../../embedded/parser/recipe.sublime-syntax"))
            .unwrap();
        highlighter.set_syntax_by_name("Eldiron Recipe");
        assert_eq!(highlighter.syntax(), "Eldiron Recipe");
    }

    #[test]
    fn completed_preview_is_sent_to_the_sidebar_minimap_owner() {
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(100, 100, 1.0);
        ui.init(&mut ctx);
        let receiver = ui.add_state_listener("Recipe preview test".into());
        let asset_id = Uuid::new_v4();
        let preview = TheRGBABuffer::from(vec![12, 34, 56, 255], 1, 1);
        let mut dock = RecipeEditorDock::new();
        dock.selected = Some(asset_id);

        dock.set_preview(&mut ui, &mut ctx, preview.clone());
        ui.process_events(&mut ctx);

        assert!(receiver.try_iter().any(|event| matches!(
            event,
            TheEvent::Custom(id, TheValue::Image(image))
                if id.name == RECIPE_MINIMAP_PREVIEW
                    && id.uuid == asset_id
                    && image == preview
        )));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn edited_recipe_reaches_the_actual_minimap_buffer_end_to_end() {
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(100, 100, 1.0);
        let mut root = TheCanvas::new();
        root.set_widget(TheRenderView::new(TheId::named("MiniMap")));
        ui.canvas = root;
        ui.init(&mut ctx);
        let receiver = ui.add_state_listener("Recipe worker test".into());
        let mut project = Project::new();
        let asset = ProceduralRecipeAsset::new(
            "worker-test",
            "Tile\n    name = \"Worker Test\"\n    size = I2(8, 8)\n    seed = 2\n\n    Noise Field\n        seed = 3\n\n    Height Surface\n        source = Field\n\n    Output\n        height = Surface\n",
        );
        let asset_id = asset.id;
        project.procedural_recipes.insert(asset_id, asset);

        let mut dock = RecipeEditorDock::new();
        dock.selected = Some(asset_id);
        dock.schedule_preview(&mut ctx, &project, asset_id);

        let deadline = Instant::now() + Duration::from_secs(3);
        let render_event = loop {
            ui.process_events(&mut ctx);
            if let Some(event) = receiver.try_iter().find(|event| {
                matches!(
                    event,
                    TheEvent::Custom(id, TheValue::Int(_))
                        if id.name == RENDER_RECIPE_PREVIEW && id.uuid == asset_id
                )
            }) {
                break event;
            }
            assert!(
                Instant::now() < deadline,
                "recipe worker did not finish in time"
            );
            std::thread::sleep(Duration::from_millis(10));
        };

        assert!(dock.handle_event(
            &render_event,
            &mut ui,
            &mut ctx,
            &mut project,
            &mut ServerContext::default(),
        ));
        let first_minimap = ui
            .get_render_view("MiniMap")
            .unwrap()
            .render_buffer_mut()
            .clone();

        let edited_source = project.procedural_recipes[&asset_id]
            .source
            .replace("seed = 3", "seed = 900");
        project
            .procedural_recipes
            .get_mut(&asset_id)
            .unwrap()
            .source = edited_source;
        assert!(dock.handle_event(
            &TheEvent::Custom(
                TheId::named_with_id(RECIPE_SOURCE_CHANGED, asset_id),
                TheValue::Empty,
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut ServerContext::default(),
        ));
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut server_ctx = ServerContext::default();
        loop {
            if dock.poll_background(&mut ui, &mut ctx, &mut project, &mut server_ctx) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "edited recipe worker was not applied by background polling"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let second_minimap = ui
            .get_render_view("MiniMap")
            .unwrap()
            .render_buffer_mut()
            .clone();

        assert_ne!(first_minimap.pixels(), second_minimap.pixels());
        assert!(project.procedural_recipes[&asset_id].tile_id.is_some());
    }
}
