//! Experimental geometry workspace using the existing MZ row widget.
use crate::{
    editor::{RUSTERIX, UNDOMANAGER},
    material_editor::*,
    prelude::*,
};
use denrim_materials::geometry::{Document, Evaluation, Operation, Row, Tile, Value};
pub(crate) mod animation;
mod assets;
mod authoring_params;
mod control_publisher;
mod groups;
mod interaction;
mod layout;
mod recipe_browser;
pub use layout::{is_visible, main_canvas, sync_workspace};
mod effects;
mod presentation;
pub(crate) mod preview;
mod rigging;
mod scene;
pub(crate) mod textures;
pub const STACK: &str = "Appearance Workspace";
pub const MODE: &str = "Appearance Mode";
const EDITOR: &str = "MZ Geometry Rows";
const PREVIEW: &str = "MZ Geometry Preview";
#[cfg(test)]
const ASSETS: &str = "MZ Geometry Assets";

pub fn workspace(material: TheCanvas) -> TheCanvas {
    let mut geometry = TheCanvas::new();
    geometry.set_widget(MaterialEditorWidget::new(TheId::named(EDITOR)));
    let mut bar = TheHLayout::new(TheId::empty());
    bar.limiter_mut().set_max_height(28);
    bar.set_margin(Vec4::new(4, 2, 4, 2));
    bar.set_padding(4);
    let mut name = TheTextLineEdit::new(TheId::named("MZ Geometry Name"));
    name.limiter_mut().set_max_width(148);
    bar.add_widget(Box::new(name));
    for (id, icon, status) in [
        ("MZ Duplicate Geometry", "mz-copy", "Duplicate geometry"),
        ("MZ Delete Geometry", "mz-trash", "Delete geometry"),
    ] {
        let mut b = TheTraybarButton::new(TheId::named(id));
        b.set_icon_name(icon.into());
        b.set_status_text(status);
        b.limiter_mut().set_max_width(24);
        bar.add_widget(Box::new(b));
    }
    let mut asset_header = TheCanvas::new();
    asset_header.set_layout(bar);
    let mut bar = TheHLayout::new(TheId::empty());
    // This canvas reserves two tool rows; its nested asset header takes 28px.
    bar.limiter_mut().set_max_height(56);
    bar.set_margin(Vec4::new(4, 2, 4, 2));
    bar.set_padding(4);
    let mut create = TheDropdownMenu::new(TheId::named("MZ New Geometry"));
    for name in ["New…", "Browse recipes…", "Empty rig", "Human", "Sword"] {
        create.add_option(name.into());
    }
    create.limiter_mut().set_max_width(145);
    bar.add_widget(Box::new(create));
    let mut actions = TheDropdownMenu::new(TheId::named("MZ Geometry Actions"));
    for name in [
        "Asset…",
        "Publish controls…",
        "Animation events…",
        "Import model…",
        "Export model…",
        "Advanced controls (TOML)…",
    ] {
        actions.add_option(name.into());
    }
    actions.limiter_mut().set_max_width(145);
    bar.add_widget(Box::new(actions));
    for (id, icon, status) in [
        (
            "MZ Edit Group",
            "mz-group",
            "Edit the selected group's construction",
        ),
        ("MZ Parent Group", "mz-back", "Return to the parent group"),
    ] {
        let mut b = TheTraybarButton::new(TheId::named(id));
        b.set_icon_name(icon.into());
        b.set_status_text(status);
        b.set_disabled(true);
        b.limiter_mut().set_max_width(24);
        bar.add_widget(Box::new(b));
    }
    bar.limiter_mut().set_max_height(76);
    let mut top = TheCanvas::new();
    top.set_layout(bar);
    top.set_top(asset_header);
    let mut breadcrumb = TheText::new(TheId::named("MZ Group Path"));
    breadcrumb.set_text("Appearance".into());
    breadcrumb.set_text_size(11.);
    breadcrumb.limiter_mut().set_max_height(20);
    let mut path = TheCanvas::new();
    path.set_widget(breadcrumb);
    top.set_bottom(path);
    geometry.set_top(top);
    let mut stack = TheStackLayout::new(TheId::named(STACK));
    stack.add_canvas(material);
    stack.add_canvas(geometry);
    let mut compact = TheCanvas::new();
    let mut info = TheText::new(TheId::named("MZ Mesh Selection Name"));
    info.set_text("Select an MZ mesh in the tilepicker".into());
    info.set_text_size(14.);
    let mut row = TheVLayout::new(TheId::named("MZ Mesh Selection Panel"));
    row.set_margin(Vec4::new(12, 12, 12, 12));
    row.add_widget(Box::new(info));
    let mut edit = TheTraybarButton::new(TheId::named("MZ Edit Selected Mesh"));
    edit.set_text("Edit mesh".into());
    edit.set_status_text("Open the selected mesh recipe in the full editor");
    row.add_widget(Box::new(edit));
    let mut create = TheTraybarButton::new(TheId::named("MZ New Box"));
    create.set_text("New mesh".into());
    create.set_status_text("Create a new editable MZ mesh");
    row.add_widget(Box::new(create));
    row.limiter_mut().set_max_height(110);
    let mut heading = TheCanvas::new();
    heading.set_layout(row);
    compact.set_top(heading);
    compact.set_center(crate::iso_editor::placement_canvas());
    stack.add_canvas(compact);
    let mut canvas = TheCanvas::new();
    canvas.set_layout(stack);
    let mut mode = TheGroupButton::new(TheId::named(MODE));
    mode.add_text("Materials".into());
    mode.add_text("Meshes".into());
    mode.set_item_width(110);
    let mut bar = TheHLayout::new(TheId::empty());
    bar.limiter_mut().set_max_height(28);
    bar.set_margin(Vec4::new(4, 2, 4, 2));
    bar.add_widget(Box::new(mode));
    bar.add_widget(crate::appearance_workspace::return_button());
    let mut top = TheCanvas::new();
    top.set_layout(bar);
    canvas.set_top(top);
    canvas
}
#[derive(Default)]
pub struct GeometryEditor {
    active: bool,
    selected: Option<Uuid>,
    iso_output: Option<Uuid>,
    panel_revision: Option<(Option<Uuid>, u64)>,
    key: Option<String>,
    viewport_visible: bool,
    open_in_rig: bool,
    source_revision: u64,
    effect_clock: Option<std::time::Instant>,
    preview_epoch: u64,
    preview_context: String,
    displayed_revision: u64,
    document: Option<Document>,
    root_document: Option<Document>,
    group_path: Vec<String>,
    control_target: Option<(Uuid, Vec<String>, String, String)>,
    recipe_browser: Option<recipe_browser::Browser>,
    control_publisher: Option<control_publisher::Publisher>,
    evaluation: Option<Evaluation>,
    selected_operation: Option<String>,
    selected_source: Option<String>,
    preview_selected: bool,
    camera: usize,
    iso_mode: bool,
    contour_camera_available: Option<bool>,
    yaw: f32,
    pitch: f32,
    drag: Option<(Vec2<i32>, Option<usize>, String)>,
    contour_frame: Option<presentation::ContourFrame>,
    revision: u64,
    preview_key: String,
    worker: Option<preview::Worker>,
    inspector: rigging::Selection,
    inspector_dirty: bool,
    local_edit: bool,
    playback: animation::Playback,
    playback_asset: Option<Uuid>,
    scene: scene::Scene,
    texture_sources: Vec<String>,
}
impl GeometryEditor {
    fn texture_material_for_edit(
        &mut self,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> Result<Uuid, String> {
        let Some(Operation::Texture {
            source,
            bundle,
            size,
            ..
        }) = self.selected_texture().cloned()
        else {
            return Err("Select a Texture operation".into());
        };
        if bundle.is_none() {
            if let Ok(id) = Uuid::parse_str(&source) {
                if project.procedural_materials.contains_key(&id) {
                    project.ensure_material_tiles();
                    return Ok(id);
                }
            }
        }
        let spec = denrim_materials::geometry::Texture {
            source: source.clone(),
            bundle: bundle.clone(),
            size,
        };
        let asset = textures::asset(&spec, &project.palette, Some(project))?;
        let before = project.clone();
        let linked_ruleset = if bundle.is_none() {
            source
                .strip_prefix("ruleset:material/")
                .map(shared::rulesets::material_asset_id)
        } else {
            None
        };
        let id = linked_ruleset.unwrap_or_else(Uuid::new_v4);
        let saved = crate::procedural_material::duplicate_bundle(
            &asset.save().map_err(|e| e.to_string())?,
            id,
        )?;
        let mut candidate = project.clone();
        candidate.procedural_materials.insert(id, saved);
        if linked_ruleset.is_none() {
            let mut next = self.document.clone().ok_or("Select a mesh recipe")?;
            let tile = next
                .rows
                .iter_mut()
                .flat_map(|r| &mut r.tiles)
                .find(|t| Some(&t.id) == self.selected_operation.as_ref())
                .ok_or("Select a Texture operation")?;
            if let Operation::Texture { source, bundle, .. } = &mut tile.op {
                *source = id.to_string();
                *bundle = None;
            }
            let root = self.encode_active(next, &candidate)?;
            candidate
                .procedural_appearances
                .insert(self.selected.ok_or("Select a mesh")?, root);
        }
        candidate.ensure_material_tiles();
        *project = candidate;
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::TilePickerEdit(Box::new(before), Box::new(project.clone())),
            ctx,
        );
        self.key = Some(String::new());
        Ok(id)
    }
    fn selected_texture(&self) -> Option<&Operation> {
        let id = self.selected_operation.as_ref()?;
        self.document
            .as_ref()?
            .rows
            .iter()
            .flat_map(|r| &r.tiles)
            .find(|t| &t.id == id)
            .map(|t| &t.op)
            .filter(|op| matches!(op, Operation::Texture { .. }))
    }
    pub fn wants_realtime(&self) -> bool {
        self.active && self.viewport_visible
    }
    fn accepts_frame(&self, epoch: u64, revision: u64) -> bool {
        epoch == self.preview_epoch && revision > self.displayed_revision
    }
    pub fn finish_drag(&mut self, project: &mut Project, ctx: &mut TheContext, cancel: bool) {
        self.finish_scene_drag(project, ctx, cancel);
        if let Some((_, Some(_), before)) = self.drag.take() {
            if let Some(id) = self.selected {
                let after = project.procedural_appearances.get(&id).cloned();
                if cancel {
                    project.procedural_appearances.insert(id, before);
                } else if after.as_ref() != Some(&before) {
                    UNDOMANAGER.write().unwrap().add_undo(
                        ProjectUndoAtom::ProceduralAppearanceEdit(id, Some(before), after),
                        ctx,
                    );
                }
                self.document = project
                    .procedural_appearances
                    .get(&id)
                    .and_then(|text| Document::load(text).ok())
                    .and_then(|root| root.group_document(&self.group_path).ok().cloned());
            }
        }
    }
    pub fn reset(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.active = false;
        self.viewport_visible = false;
        self.open_in_rig = false;
        self.selected = None;
        self.iso_output = None;
        self.key = None;
        self.document = None;
        self.root_document = None;
        self.group_path.clear();
        self.evaluation = None;
        self.selected_operation = None;
        self.selected_source = None;
        self.drag = None;
        self.preview_key.clear();
        self.playback = Default::default();
        self.playback_asset = None;
        self.scene = Default::default();
        self.revision += 1;
        Self::mode(ui, ctx, 0);
    }
    fn mode(ui: &mut TheUI, ctx: &mut TheContext, index: usize) {
        if let Some(s) = ui.get_stack_layout(STACK) {
            if s.index() != index {
                // Hidden row editors must not retain keyboard/text-input focus.
                ctx.ui.clear_focus();
                ctx.ui.keyboard_focus = None;
            }
            s.set_index(index);
        }
        if let Some(b) = ui.get_widget(MODE).and_then(|w| w.as_group_button()) {
            b.set_index(index.min(1) as i32);
        }
        sync_workspace(ui, ctx);
        ctx.ui.relayout = true;
        ctx.ui.redraw_all = true;
    }
    fn status(ctx: &mut TheContext, text: impl Into<String>) {
        ctx.ui
            .send(TheEvent::SetStatusText(TheId::empty(), text.into()));
    }
    fn commit(
        &mut self,
        doc: Document,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> Result<(), String> {
        let id = self.selected.ok_or("Select a geometry asset")?;
        let next = self.encode_active(doc, project)?;
        let next_doc = Document::parse(&next).map_err(|e| e.to_string())?;
        let mut candidate = project.clone();
        candidate.procedural_appearances.insert(id, next.clone());
        shared::appearance_assets::resolve(&candidate, &next_doc)?;
        let before = project.procedural_appearances.get(&id).cloned();
        if before.as_ref() != Some(&next) {
            self.local_edit = true;
            project.procedural_appearances.insert(id, next.clone());
            UNDOMANAGER.write().unwrap().add_undo(
                ProjectUndoAtom::ProceduralAppearanceEdit(id, before, Some(next)),
                ctx,
            );
        }
        Ok(())
    }
    fn encode_active(&self, doc: Document, project: &Project) -> Result<String, String> {
        if self.group_path.is_empty() {
            return doc.save().map_err(|e| e.to_string());
        }
        let source = self
            .selected
            .and_then(|id| project.procedural_appearances.get(&id))
            .ok_or("Select a geometry asset")?;
        let mut root = Document::parse(source).map_err(|e| e.to_string())?;
        let previous = root
            .group_document(&self.group_path)
            .map_err(|e| e.to_string())?
            .label
            .clone();
        let label = doc.label.clone();
        *root
            .group_document_mut(&self.group_path)
            .map_err(|e| e.to_string())? = doc;
        if previous != label {
            let (id, parent) = self.group_path.split_last().unwrap();
            if let Some(tile) = root
                .group_document_mut(parent)
                .map_err(|e| e.to_string())?
                .rows
                .iter_mut()
                .flat_map(|r| &mut r.tiles)
                .find(|t| &t.id == id)
            {
                if let Operation::Group { label: name, .. } = &mut tile.op {
                    *name = label;
                }
            }
        }
        root.save().map_err(|e| e.to_string())
    }
    fn group_preview(&mut self, ui: &mut TheUI) {
        self.preview_selected = false;
        self.scene.view_bounds = None;
        if let Some(w) = ui
            .get_widget("MZ Geometry Target")
            .and_then(|w| w.as_group_button())
        {
            w.set_index(0);
        }
        if self.camera == 3 {
            self.camera = 0;
            layout::set_choice(ui, "MZ Geometry Camera", 0);
        }
    }
    fn apply_command(
        &mut self,
        command: MaterialEditorCommand,
        project: &mut Project,
        ctx: &mut TheContext,
    ) -> Result<(), String> {
        // Several commands can arrive before the next UI refresh. Always edit the
        // latest saved source, rather than the cached presentation document.
        let text = self
            .selected
            .and_then(|id| project.procedural_appearances.get(&id))
            .ok_or("Select a geometry asset")?;
        let root = Document::parse(text).map_err(|e| e.to_string())?;
        let mut next = root
            .group_document(&self.group_path)
            .map_err(|e| e.to_string())?
            .clone();
        if let MaterialEditorCommand::AddSource { source, row, after } = &command {
            if let Some(source) = source.strip_prefix("geometry_asset:") {
                let id = Uuid::parse_str(source).map_err(|_| "Invalid geometry source")?;
                let document = Document::load(
                    project
                        .procedural_appearances
                        .get(&id)
                        .ok_or("Geometry source was removed")?,
                )
                .map_err(|e| e.to_string())?;
                let op = Operation::Asset {
                    source: source.into(),
                    document: Box::new(document.clone()),
                };
                let owner = row
                    .as_ref()
                    .and_then(|id| next.rows.iter().position(|r| &r.id == id))
                    .unwrap_or(next.rows.len());
                if owner == next.rows.len() {
                    next.rows.push(Row {
                        id: Uuid::new_v4().to_string(),
                        label: document.label,
                        tiles: vec![],
                    });
                }
                let tiles = &mut next.rows[owner].tiles;
                let position = after
                    .as_ref()
                    .and_then(|id| tiles.iter().position(|t| &t.id == id))
                    .map_or(0, |i| i + 1);
                tiles.insert(
                    position,
                    Tile {
                        id: Uuid::new_v4().to_string(),
                        op,
                    },
                );
                return self.commit(next, project, ctx);
            }
        }
        if let MaterialEditorCommand::SetNumber {
            parameter, value, ..
        } = &command
        {
            let index = value.round().max(0.) as usize;
            match parameter.as_str() {
                "@joint" => self.inspector.joint = index,
                "@part" => {
                    self.inspector.part = index;
                    self.inspector.section = 0;
                }
                "@section" => self.inspector.section = index,
                "@group_category" => self.inspector.category = index,
                "@track" => {
                    self.inspector.track = index;
                    self.inspector.key = 0;
                    if let Some(operation) = &self.selected_operation {
                        self.playback.inspect(&next, operation, &self.inspector);
                    }
                }
                "@key" => {
                    self.inspector.key = index;
                    if let Some(operation) = &self.selected_operation {
                        self.playback.inspect(&next, operation, &self.inspector);
                    }
                }
                "texture_source" => {
                    let operation =
                        if let MaterialEditorCommand::SetNumber { operation, .. } = &command {
                            operation
                        } else {
                            unreachable!()
                        };
                    if let Some(Tile {
                        op: Operation::Texture { source, bundle, .. },
                        ..
                    }) = next
                        .rows
                        .iter_mut()
                        .flat_map(|r| &mut r.tiles)
                        .find(|t| &t.id == operation)
                    {
                        *source = self
                            .texture_sources
                            .get(index)
                            .cloned()
                            .ok_or("Choose a material")?;
                        *bundle = None;
                    }
                    return self.commit(next, project, ctx);
                }
                _ => {
                    presentation::edit_selected(&mut next, command, &self.inspector)?;
                    return self.commit(next, project, ctx);
                }
            }
            self.inspector_dirty = true;
            self.preview_key.clear();
            return Ok(());
        }
        presentation::edit_selected(&mut next, command, &self.inspector)?;
        self.commit(next, project, ctx)
    }
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        if self.recipe_browser_event(event, project, ui, ctx)
            || self.control_publisher_event(event, project, ui, ctx)
        {
            return true;
        }
        if self.asset_actions(event, project, ui, ctx) {
            return true;
        }
        if let TheEvent::Custom(id, TheValue::Id(tile)) = event {
            if matches!(id.name.as_str(), "Tile Picked" | "Open Procedural Tile") {
                if let Some(shared::isometric::TileRecipe::Geometry(recipe)) =
                    shared::isometric::tile_recipe(project, *tile)
                {
                    self.finish_drag(project, ctx, false);
                    self.selected = Some(recipe);
                    self.iso_output = Some(*tile);
                    // Tile/pixel-editor delegation selects a mesh for placement. Only
                    // explicit mesh editing enters the central modelling workspace.
                    self.active = false;
                    self.group_path.clear();
                    self.key = None;
                    self.document = None;
                    self.evaluation = None;
                    self.scene = Default::default();
                    self.inspector = Default::default();
                    self.selected_operation = None;
                    self.preview_selected = false;
                    self.open_in_rig = false;
                    self.camera = 4;
                    self.preview_key.clear();
                    crate::sidebar::Sidebar::set_navigation_page(
                        crate::procedural_material::SIDEBAR_PAGE,
                        ui,
                        ctx,
                    );
                    if let Some(w) = ui.get_text("MZ Mesh Selection Name") {
                        let label = project
                            .procedural_appearances
                            .get(&recipe)
                            .and_then(|s| Document::load(s).ok())
                            .map(|d| d.label)
                            .unwrap_or_else(|| "MZ mesh".into());
                        w.set_text(label);
                    }
                    Self::mode(ui, ctx, if self.active { 1 } else { 2 });
                    // Continue dispatch so the material editor clears its old source.
                    return false;
                }
                self.iso_output = None;
            }
        }
        if let TheEvent::Custom(id, TheValue::Id(asset)) = event {
            if id.name == "Open Procedural Geometry"
                || id.name == "Open Worn Geometry"
                || id.name == "Open Geometry Recipe"
            {
                self.finish_drag(project, ctx, false);
                self.selected = Some(*asset);
                self.iso_output = None;
                self.active = true;
                self.group_path.clear();
                self.key = None;
                self.scene = Default::default();
                self.open_in_rig = id.name == "Open Procedural Geometry";
                crate::sidebar::Sidebar::set_navigation_page(
                    crate::procedural_material::SIDEBAR_PAGE,
                    ui,
                    ctx,
                );
                Self::mode(ui, ctx, 1);
                return true;
            }
        }
        if matches!(event, TheEvent::IndexChanged(id, _) if id.name == "Ruleset Material")
            || matches!(event,TheEvent::Custom(id,_) if id.name=="Open Procedural Tile")
        {
            self.finish_drag(project, ctx, false);
            self.active = false;
            Self::mode(ui, ctx, 0);
            return false;
        }
        match self.scene_event(event, project, ui, ctx) {
            Ok(true) => return true,
            Err(e) => {
                Self::status(ctx, e);
                return true;
            }
            _ => {}
        }
        if self.active {
            if let TheEvent::ValueChanged(id, value) = event {
                let command = ui
                    .get_widget(EDITOR)
                    .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                    .and_then(|w| w.number_command(id, value));
                if let Some(command) = command {
                    if let Err(e) = self.apply_command(command, project, ctx) {
                        Self::status(ctx, e);
                        self.key = Some(String::new());
                        self.local_edit = false;
                    }
                    return true;
                }
            }
        }
        if self.active && self.playback.event(event) {
            if matches!(event,TheEvent::IndexChanged(id,_) if id.name=="MZ Motion") {
                self.scene.view_bounds = None;
            }
            self.preview_key.clear();
            return true;
        }
        if matches!(event,TheEvent::GainedFocus(id) if id.name=="MZ Motion Phase") {
            self.playback.playing = false;
        }
        if matches!(event,TheEvent::GainedFocus(id) if id.name!=PREVIEW) {
            self.finish_drag(project, ctx, false);
        }
        if let TheEvent::IndexChanged(id, index) = event {
            if id.name == "MZ New Geometry" {
                let preset = match index {
                    1 => Some("MZ New Box"),
                    2 => Some("MZ New Model"),
                    3 => Some("MZ New Human"),
                    4 => Some("MZ New Sword"),
                    _ => None,
                };
                layout::set_choice(ui, "MZ New Geometry", 0);
                return preset.is_some_and(|name| {
                    self.handle_event(
                        &TheEvent::StateChanged(TheId::named(name), TheWidgetState::Clicked),
                        project,
                        ui,
                        ctx,
                    )
                });
            }
        }
        match event {
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "MZ Edit Selected Mesh" =>
            {
                if self.selected.is_some() {
                    self.active = true;
                    if self.document.is_none() && self.iso_output.is_none() {
                        self.open_in_rig = true;
                    }
                    self.key = None;
                    Self::mode(ui, ctx, 1);
                }
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "MZ Edit Texture Material" =>
            {
                match self.texture_material_for_edit(project, ctx) {
                    Ok(material) => {
                        self.active = false;
                        Self::mode(ui, ctx, 0);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Open Mesh Material"),
                            TheValue::Id(material),
                        ));
                    }
                    Err(error) => Self::status(ctx, error),
                }
                return true;
            }
            TheEvent::Custom(id, _) if id.name == "Return to Mesh Recipe" => {
                self.active = true;
                self.preview_key.clear();
                Self::mode(ui, ctx, 1);
                return false;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "MZ Use Ruleset Humanoid" =>
            {
                self.finish_drag(project, ctx, false);
                if let Some(source) = self
                    .selected
                    .and_then(|id| project.procedural_appearances.get(&id))
                {
                    let mut doc = match Document::load(source) {
                        Ok(d) => d,
                        Err(e) => {
                            Self::status(ctx, e.to_string());
                            return true;
                        }
                    };
                    let id =
                        Uuid::parse_str(shared::rulesets::HUMANOID_APPEARANCE_ASSET_ID).unwrap();
                    doc.id = id.to_string();
                    doc.label = "Ruleset humanoid".into();
                    if let Ok(source) = doc.save() {
                        let before = project.procedural_appearances.insert(id, source.clone());
                        if before.as_ref() != Some(&source) {
                            UNDOMANAGER.write().unwrap().add_undo(
                                ProjectUndoAtom::ProceduralAppearanceEdit(id, before, Some(source)),
                                ctx,
                            );
                        }
                        self.scene.select_after_refresh = self.selected_operation.clone();
                        self.selected = Some(id);
                        self.key = None;
                        Self::status(
                            ctx,
                            "Ruleset humanoid selected. Model edits now update 2D and 3D scenes.",
                        );
                    }
                }
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "MZ Close Geometry" =>
            {
                self.finish_drag(project, ctx, false);
                self.active = false;
                self.playback.playing = false;
                Self::mode(ui, ctx, if self.selected.is_some() { 2 } else { 0 });
                let output = self
                    .iso_output
                    .filter(|id| {
                        project
                            .isometric_bakes
                            .get(id)
                            .is_some_and(|b| Some(b.recipe) == self.selected)
                    })
                    .or_else(|| {
                        project
                            .isometric_bakes
                            .iter()
                            .find(|(_, b)| Some(b.recipe) == self.selected)
                            .map(|(id, _)| *id)
                    });
                if let Some(output) = output {
                    self.iso_output = Some(output);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Pick Tile Source"),
                        TheValue::List(vec![TheValue::Text("single".into()), TheValue::Id(output)]),
                    ));
                }
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Play" && self.active =>
            {
                self.finish_drag(project, ctx, false);
                self.active = false;
                Self::mode(ui, ctx, 0);
                return false;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == "MZ Edit Group" => {
                self.finish_drag(project, ctx, false);
                let source = self
                    .document
                    .as_ref()
                    .and_then(|d| {
                        d.rows
                            .iter()
                            .flat_map(|r| &r.tiles)
                            .find(|t| Some(&t.id) == self.selected_operation.as_ref())
                    })
                    .and_then(|t| {
                        if let Operation::Asset { source, .. } = &t.op {
                            Some(source.clone())
                        } else {
                            None
                        }
                    });
                if let Some(source) = source {
                    if source == denrim_materials::geometry::WEARER_SOURCE {
                        Self::status(
                            ctx,
                            "This input is supplied by the character wearing the garment",
                        );
                        return true;
                    }
                    match shared::appearance_assets::asset_id(&source) {
                        Ok(id) => {
                            self.selected = Some(id);
                            self.scene = Default::default();
                            self.open_in_rig = true;
                            self.group_path.clear();
                            self.key = None;
                            self.inspector = Default::default();
                            self.selected_operation = None;
                            self.group_preview(ui);
                        }
                        Err(e) => Self::status(ctx, e),
                    }
                    return true;
                }
                if let Some(operation) = self.selected_operation.as_ref() {
                    if self.document.as_ref().is_some_and(|d| {
                        d.rows
                            .iter()
                            .flat_map(|r| &r.tiles)
                            .any(|t| &t.id == operation && matches!(t.op, Operation::Group { .. }))
                    }) {
                        self.group_path.push(operation.clone());
                        self.key = None;
                        self.inspector = Default::default();
                        self.selected_operation = None;
                        self.group_preview(ui);
                    }
                }
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == "MZ Parent Group" => {
                self.finish_drag(project, ctx, false);
                if self.group_path.pop().is_some() {
                    self.key = None;
                    self.inspector = Default::default();
                    self.selected_operation = None;
                    self.group_preview(ui);
                }
                return true;
            }
            TheEvent::IndexChanged(id, index) if id.name == MODE => {
                self.finish_drag(project, ctx, false);
                self.active = false;
                Self::mode(ui, ctx, if *index == 1 { 2 } else { 0 });
                self.preview_key.clear();
                return true;
            }
            TheEvent::Custom(id, _) if id.name == "Tile Picked" => {
                self.finish_drag(project, ctx, false);
                if self.active {
                    self.active = false;
                    Self::mode(ui, ctx, 0);
                }
                return false;
            }
            TheEvent::ValueChanged(id, TheValue::Text(label)) if id.name == "MZ Geometry Name" => {
                if let Some(mut next) = self
                    .selected
                    .and_then(|id| project.procedural_appearances.get(&id))
                    .and_then(|text| Document::load(text).ok())
                    .and_then(|root| root.group_document(&self.group_path).ok().cloned())
                {
                    next.label = label.trim().chars().take(80).collect();
                    if !next.label.is_empty() {
                        if let Err(e) = self.commit(next, project, ctx) {
                            Self::status(ctx, e);
                        }
                    }
                }
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "MZ New Sword"
                    || id.name == "MZ New Human"
                    || id.name == "MZ New Model" =>
            {
                self.finish_drag(project, ctx, false);
                self.active = true;
                Self::mode(ui, ctx, 1);
                let mut doc = if id.name == "MZ New Model" {
                    scene::new_document()
                } else {
                    Document::load(if id.name == "MZ New Human" {
                        shared::rulesets::bundled_appearance_assets_for_ruleset(
                            shared::rulesets::OFFICIAL_RULESET_ID,
                            shared::rulesets::OFFICIAL_RULESET_VERSION,
                        )
                        .first()
                        .map_or(denrim_materials::geometry::HUMAN_EXAMPLE, |a| a.source)
                    } else {
                        denrim_materials::geometry::SWORD_GROUP_EXAMPLE
                    })
                    .unwrap()
                };
                self.scene = Default::default();
                self.open_in_rig = id.name == "MZ New Human";
                if id.name == "MZ New Model" {
                    self.scene.tool = 2;
                    layout::set_choice(ui, "MZ Geometry Tool", 2);
                }
                let id_new = Uuid::new_v4();
                doc.id = id_new.to_string();
                let text = doc.save().unwrap();
                project.procedural_appearances.insert(id_new, text.clone());
                UNDOMANAGER.write().unwrap().add_undo(
                    ProjectUndoAtom::ProceduralAppearanceEdit(id_new, None, Some(text)),
                    ctx,
                );
                self.selected = Some(id_new);
                self.key = None;
                self.group_path.clear();
                self.inspector = Default::default();
                self.preview_selected = false;
                if let Some(b) = ui
                    .get_widget("MZ Geometry Target")
                    .and_then(|w| w.as_group_button())
                {
                    b.set_index(0);
                }
                self.camera = if self.iso_mode { 4 } else { 0 };
                layout::set_choice(ui, "MZ Geometry Camera", self.camera);
                self.yaw = -25.;
                self.pitch = 12.;
                self.selected_operation = None;
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "MZ Duplicate Geometry" =>
            {
                self.finish_drag(project, ctx, false);
                if let Some(mut doc) = self
                    .selected
                    .and_then(|id| project.procedural_appearances.get(&id))
                    .and_then(|text| Document::load(text).ok())
                {
                    let id = Uuid::new_v4();
                    doc.id = id.to_string();
                    doc.label.push_str(" Copy");
                    if let Ok(text) = doc.save() {
                        project.procedural_appearances.insert(id, text.clone());
                        UNDOMANAGER.write().unwrap().add_undo(
                            ProjectUndoAtom::ProceduralAppearanceEdit(id, None, Some(text)),
                            ctx,
                        );
                        self.selected = Some(id);
                        self.key = None;
                        self.group_path.clear();
                    }
                }
                return true;
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "MZ Delete Geometry" =>
            {
                self.finish_drag(project, ctx, false);
                if let Some(id) = self.selected {
                    let used = project.procedural_appearances.iter().any(|(other, text)| {
                        *other != id
                            && (text.contains(&id.to_string())
                                || shared::rulesets::bundled_appearance_assets_for_ruleset(
                                    shared::rulesets::OFFICIAL_RULESET_ID,
                                    shared::rulesets::OFFICIAL_RULESET_VERSION,
                                )
                                .iter()
                                .any(|a| {
                                    Uuid::parse_str(a.asset_id).ok() == Some(id)
                                        && text.contains(&format!("ruleset:geometry/{}", a.id))
                                }))
                    }) || project
                        .procedural_materials
                        .values()
                        .any(|source| source.contains(&id.to_string()))
                        || project
                            .characters
                            .values()
                            .any(|character| character.data.contains(&id.to_string()))
                        || project.items.values().any(|item| {
                            item.data.contains(&id.to_string())
                                || shared::rulesets::bundled_appearance_assets_for_ruleset(
                                    shared::rulesets::OFFICIAL_RULESET_ID,
                                    shared::rulesets::OFFICIAL_RULESET_VERSION,
                                )
                                .iter()
                                .any(|a| {
                                    Uuid::parse_str(a.asset_id).ok() == Some(id)
                                        && item.data.contains(&format!(
                                            "appearance_geometry = \"{}\"",
                                            a.id
                                        ))
                                })
                        });
                    if used {
                        Self::status(
                            ctx,
                            "This geometry is referenced by an item or another model",
                        );
                        return true;
                    }
                    if let Some(before) = project.procedural_appearances.shift_remove(&id) {
                        UNDOMANAGER.write().unwrap().add_undo(
                            ProjectUndoAtom::ProceduralAppearanceEdit(id, Some(before), None),
                            ctx,
                        );
                        self.document = None;
                        self.evaluation = None;
                        self.selected = None;
                        self.key = None;
                        self.group_path.clear();
                        self.preview_key.clear();
                    }
                }
                return true;
            }
            TheEvent::IndexChanged(id, index) if id.name == "MZ Geometry Camera" => {
                if *index == 3
                    && !self.document.as_ref().is_some_and(|d| {
                        d.rows
                            .iter()
                            .flat_map(|r| &r.tiles)
                            .find(|t| Some(&t.id) == self.selected_operation.as_ref())
                            .is_some_and(|t| {
                                matches!(t.op, Operation::Contour { .. } | Operation::Loft { .. })
                            })
                    })
                {
                    layout::set_choice(ui, "MZ Geometry Camera", self.camera);
                    Self::status(ctx, "Select a Contour or Loft tile for contour editing");
                    return true;
                }
                self.camera = *index as usize;
                if self.camera == 1 {
                    self.yaw = 0.;
                    self.pitch = 0.;
                }
                if self.camera == 2 {
                    self.yaw = 90.;
                    self.pitch = 0.;
                }
                self.preview_key.clear();
                return true;
            }
            TheEvent::IndexChanged(id, index) if id.name == "MZ Geometry Target" => {
                self.preview_selected = *index == 1;
                self.preview_key.clear();
                return true;
            }
            TheEvent::Custom(id, TheValue::Id(widget)) if id.name == MATERIAL_EDITOR_COMMANDS => {
                let commands = ui
                    .get_widget(EDITOR)
                    .and_then(|w| {
                        if w.id().uuid != *widget {
                            return None;
                        }
                        w.as_any().downcast_mut::<MaterialEditorWidget>()
                    })
                    .map(MaterialEditorWidget::take_commands);
                let Some(commands) = commands else {
                    return false;
                };
                for command in commands {
                    match command {
                        MaterialEditorCommand::InspectOperation(operation) => {
                            self.finish_drag(project, ctx, false);
                            if self.selected_operation.as_ref() != Some(&operation) {
                                self.inspector = Default::default();
                                self.contour_frame = None;
                            }
                            if let Some(doc) = &self.document {
                                self.playback.inspect(doc, &operation, &self.inspector);
                            }
                            self.selected_operation = Some(operation);
                            self.selected_source = None;
                            self.inspector_dirty = true;
                            self.preview_key.clear();
                        }
                        MaterialEditorCommand::InspectRow(row) => {
                            if let Some(id) = self
                                .document
                                .as_ref()
                                .and_then(|d| d.rows.iter().find(|r| r.id == row))
                                .and_then(|r| r.tiles.first())
                                .map(|t| t.id.clone())
                            {
                                self.scene_select(id);
                            }
                        }
                        MaterialEditorCommand::InspectSource(source) => {
                            self.selected_source = Some(source)
                        }
                        MaterialEditorCommand::InspectInput { .. } => {}
                        MaterialEditorCommand::InspectSurface => {
                            self.preview_selected = false;
                            layout::set_choice(ui, "MZ Geometry Target", 0);
                            self.preview_key.clear();
                        }
                        command => {
                            if let Err(e) = self.apply_command(command, project, ctx) {
                                Self::status(ctx, e);
                            }
                        }
                    }
                }
                return true;
            }
            TheEvent::RenderViewClicked(id, p) if id.name == PREVIEW => {
                if self.document.is_some() {
                    let point = if self.camera == 3 {
                        self.contour_frame.as_ref().and_then(|f| f.hit(*p))
                    } else {
                        None
                    };
                    if let Some(source) = self
                        .selected
                        .and_then(|id| project.procedural_appearances.get(&id))
                    {
                        self.drag = Some((*p, point, source.clone()));
                    }
                }
                return true;
            }
            TheEvent::RenderViewContext(id, p) if id.name == PREVIEW && self.camera == 3 => {
                self.finish_drag(project, ctx, false);
                if let (Some(doc), Some(frame), Some(operation)) = (
                    &self.document,
                    &self.contour_frame,
                    &self.selected_operation,
                ) {
                    let mut next = doc.clone();
                    if let Some(tile) = next
                        .rows
                        .iter_mut()
                        .flat_map(|r| &mut r.tiles)
                        .find(|t| &t.id == operation)
                    {
                        if matches!(tile.op, Operation::Loft { .. }) {
                            if let Err(e) = rigging::ring_context(
                                &mut tile.op,
                                frame.hit(*p),
                                frame.unproject(*p),
                            ) {
                                Self::status(ctx, e);
                                return true;
                            }
                            if let Err(e) = self.commit(next, project, ctx) {
                                Self::status(ctx, e);
                            }
                            return true;
                        }
                    }
                    if let Some(Tile {
                        op: Operation::Contour { points },
                        ..
                    }) = next
                        .rows
                        .iter_mut()
                        .flat_map(|r| &mut r.tiles)
                        .find(|t| &t.id == operation)
                    {
                        if let Some(index) = frame.hit(*p) {
                            if points.len() > 3 {
                                points.remove(index);
                            }
                        } else if points.len() < 64 {
                            let mut q = frame.unproject(*p);
                            let edge = (0..points.len())
                                .min_by(|&i, &j| {
                                    let distance = |i: usize| {
                                        let a = points[i];
                                        let b = points[(i + 1) % points.len()];
                                        let d = [b[0] - a[0], b[1] - a[1]];
                                        let t = (((q[0] - a[0]) * d[0] + (q[1] - a[1]) * d[1])
                                            / (d[0] * d[0] + d[1] * d[1]))
                                            .clamp(0., 1.);
                                        (q[0] - a[0] - t * d[0]).hypot(q[1] - a[1] - t * d[1])
                                    };
                                    distance(i).total_cmp(&distance(j))
                                })
                                .unwrap();
                            let a = points[edge];
                            let b = points[(edge + 1) % points.len()];
                            if ((b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0])).abs()
                                < 1e-6
                            {
                                q[0] += (b[1] - a[1]) * 0.025;
                                q[1] -= (b[0] - a[0]) * 0.025;
                            }
                            points.insert(edge + 1, q);
                        }
                        if let Err(e) = self.commit(next, project, ctx) {
                            Self::status(ctx, e);
                        }
                    }
                }
                return true;
            }
            TheEvent::RenderViewDragged(id, p) if id.name == PREVIEW => {
                if let Some((previous, point, before)) = self.drag.clone() {
                    if let (Some(index), Some(frame), Some(doc), Some(operation)) = (
                        point,
                        self.contour_frame.as_ref(),
                        self.document.as_ref(),
                        self.selected_operation.as_ref(),
                    ) {
                        let mut next = doc.clone();
                        if let Some(tile) = next
                            .rows
                            .iter_mut()
                            .flat_map(|r| &mut r.tiles)
                            .find(|t| &t.id == operation)
                        {
                            if let Err(e) =
                                rigging::move_point(&mut tile.op, index, frame.unproject(*p))
                            {
                                Self::status(ctx, e);
                                return true;
                            }
                            // Invalid contours never replace the working recipe. The mouse can
                            // continue moving and recover without creating a broken document.
                            match self.encode_active(next, project) {
                                Ok(text) => {
                                    if let Some(id) = self.selected {
                                        project.procedural_appearances.insert(id, text);
                                    }
                                }
                                Err(e) => Self::status(ctx, e.to_string()),
                            }
                        }
                    } else if self.camera != 3 {
                        self.yaw += (p.x - previous.x) as f32 * 0.5;
                        self.pitch =
                            (self.pitch + (p.y - previous.y) as f32 * 0.5).clamp(-85., 85.);
                        self.preview_key.clear();
                    }
                    self.drag = Some((*p, point, before));
                }
                return true;
            }
            TheEvent::RenderViewUp(id, _) if id.name == PREVIEW => {
                self.finish_drag(project, ctx, false);
                return true;
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(TheKeyCode::Escape))
                if self.active && self.drag.is_some() =>
            {
                self.finish_drag(project, ctx, true);
                self.preview_key.clear();
                return true;
            }
            TheEvent::RenderViewScrollBy(id, d) | TheEvent::RenderViewPreciseScrollBy(id, d)
                if id.name == PREVIEW =>
            {
                if self.camera != 3 {
                    self.yaw += d.x as f32;
                    self.pitch = (self.pitch + d.y as f32).clamp(-85., 85.);
                    self.preview_key.clear();
                }
                return true;
            }
            TheEvent::KeyDown(_) | TheEvent::KeyCodeDown(_)
                if self.active
                    && !ui.ctrl
                    && !ui.logo
                    && !ui.alt
                    && ctx.ui.focus.as_ref().is_some_and(|id| {
                        id.name == EDITOR
                            || id.name == PREVIEW
                            || (id.name == "MZ Joint Name" || id.name == "MZ Profile Name")
                    }) =>
            {
                return true;
            }
            _ => {}
        }
        false
    }
    fn source_key(&self, project: &Project, id: Uuid, text: &str) -> String {
        fingerprint(&(
            id,
            text,
            project.procedural_appearances.iter().collect::<Vec<_>>(),
            serde_json::to_string(&project.palette.colors).unwrap_or_default(),
            &self.group_path,
            textures::input_key(project, text),
            project.procedural_materials.revision(),
            project
                .procedural_materials
                .iter()
                .filter(|(id, _)| text.contains(&id.to_string()))
                .map(|(_, b)| b.as_str())
                .collect::<Vec<_>>(),
            &project.config,
            shared::geometry_recipe::preview_input(project, id).map(|(_, key)| key),
        ))
        .to_string()
    }
    pub fn update(
        &mut self,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server: &ServerContext,
    ) -> bool {
        let panel_revision = (self.selected, project.procedural_appearances.revision());
        if self.panel_revision != Some(panel_revision) {
            self.panel_revision = Some(panel_revision);
            let label = self
                .selected
                .and_then(|id| project.procedural_appearances.get(&id))
                .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                .and_then(|v| v["label"].as_str().map(str::to_owned))
                .unwrap_or_else(|| "Select an MZ mesh in the tilepicker".into());
            if let Some(w) = ui.get_text("MZ Mesh Selection Name") {
                w.set_text(label);
            }
            if let Some(w) = ui.get_widget("MZ Edit Selected Mesh") {
                w.set_disabled(self.selected.is_none());
            }
        }
        let iso = project
            .get_map(server)
            .is_some_and(|m| m.camera == rusterix::MapCamera::TwoDIso);
        if iso != self.iso_mode {
            self.iso_mode = iso;
            self.camera = if iso { 4 } else { 0 };
            self.preview_key.clear();
        }
        let page = ui
            .get_stack_layout("Sidebar Page Stack")
            .is_some_and(|s| s.index() == crate::procedural_material::SIDEBAR_PAGE);
        let layout_changed = sync_workspace(ui, ctx);
        self.viewport_visible = self.active && page && is_visible(ui);
        if !self.active || !page {
            self.playback.suspend();
            return layout_changed;
        }
        layout::sync_controls(self, ui, ctx);
        if !self
            .selected
            .is_some_and(|id| project.procedural_appearances.contains_key(&id))
        {
            self.selected = project.procedural_appearances.keys().next().copied();
            self.key = None;
            self.group_path.clear();
            self.root_document = None;
        }
        let Some(id) = self.selected else {
            if self.key.is_none() {
                self.document = None;
                self.evaluation = None;
                self.selected_operation = None;
                let background = ui.style.theme().color(MaterialCanvas).to_vec();
                if let Some(w) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
                    *w.render_buffer_mut() = TheRGBABuffer::from(background, 1, 1);
                }
                if let Some(w) = ui
                    .get_widget(EDITOR)
                    .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                {
                    w.set_view(MaterialEditorView {
                        geometry: true,
                        title: "Create a geometry asset above".into(),
                        ..Default::default()
                    });
                }
                self.key = Some(String::new());
            }
            return false;
        };
        let text = &project.procedural_appearances[&id];
        let mut changed = layout_changed;
        // Refresh on undo, palette edits, load and selection, without baking gameplay tiles.
        let key = self.source_key(project, id, text);
        if self.key.as_ref() != Some(&key) {
            self.source_revision += 1;
            match Document::load(text).and_then(|root| {
                if self.open_in_rig {
                    self.open_in_rig = false;
                    if let Some(path) =
                        scene::find_path(&root, |op| matches!(op, Operation::Armature { .. }))
                    {
                        self.group_path = path;
                        self.scene.tool = 1;
                        self.scene.view_bounds = None;
                        self.playback.playing = false;
                        self.playback.selected = 0;
                        if let Some((id, _)) =
                            scene::armature(root.group_document(&self.group_path)?)
                        {
                            self.scene.select_after_refresh = Some(id.into());
                        }
                    }
                }
                if root.group_document(&self.group_path).is_err() {
                    self.group_path.clear();
                }
                let doc = root.group_document(&self.group_path)?.clone();
                let resolved = shared::appearance_assets::resolve(project, &doc)
                    .map_err(denrim_materials::Error)?;
                let inputs = self
                    .selected
                    .and_then(|id| shared::geometry_recipe::preview_input(project, id))
                    .map(|(mesh, _)| std::collections::BTreeMap::from([("selection".into(), mesh)]))
                    .unwrap_or_default();
                let mut evaluation = resolved.evaluate_with_inputs(&inputs)?;
                if let Some(body) = resolved.wearer_context() {
                    let mut preview = body.evaluate()?.mesh;
                    preview.replace_parts(evaluation.mesh.clone())?;
                    evaluation.mesh = preview;
                }
                textures::resolve_evaluation(&mut evaluation, project)
                    .map_err(denrim_materials::Error)?;
                Ok((root, doc, evaluation))
            }) {
                Ok((root, doc, evaluation)) => {
                    let named_sources = textures::ruleset_choices(project);
                    self.texture_sources = named_sources
                        .iter()
                        .cloned()
                        .map(|(uri, _)| uri)
                        .chain(project.procedural_materials.keys().map(ToString::to_string))
                        .collect();
                    let thumbnails = evaluation
                        .operations
                        .iter()
                        .map(|(id, value)| {
                            (
                                id.clone(),
                                presentation::thumbnail(
                                    value,
                                    &project.palette,
                                    ui.style.theme().as_ref(),
                                ),
                            )
                        })
                        .collect::<Vec<_>>();
                    if let Some(w) = ui
                        .get_widget(EDITOR)
                        .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                    {
                        let mut view = presentation::view_selected(
                            &doc,
                            &evaluation,
                            &project.palette,
                            &self.inspector,
                        );
                        presentation::add_project_sources(&mut view, project, self.selected);
                        for operation in view.rows.iter_mut().flat_map(|r| &mut r.operations) {
                            if let Some(p) = operation
                                .properties
                                .iter_mut()
                                .find(|p| p.id == "texture_source")
                            {
                                p.choices = self
                                    .texture_sources
                                    .iter()
                                    .map(|s| {
                                        if let Some((_, label)) =
                                            named_sources.iter().find(|(uri, _)| uri == s)
                                        {
                                            label.clone()
                                        } else {
                                            Uuid::parse_str(s)
                                                .ok()
                                                .and_then(|id| project.tiles.get(&id))
                                                .map(|t| t.alias.clone())
                                                .filter(|s| !s.is_empty())
                                                .unwrap_or_else(|| s.clone())
                                        }
                                    })
                                    .collect();
                                if let Some(n) = operation
                                    .parameters
                                    .iter_mut()
                                    .find(|n| n.id == "texture_source")
                                {
                                    n.max = self.texture_sources.len().saturating_sub(1) as f32;
                                    if let Some(Operation::Texture { source, .. }) = doc
                                        .rows
                                        .iter()
                                        .flat_map(|r| &r.tiles)
                                        .find(|t| t.id == operation.id)
                                        .map(|t| &t.op)
                                    {
                                        n.value = self
                                            .texture_sources
                                            .iter()
                                            .position(|s| s == source)
                                            .unwrap_or(0)
                                            as f32;
                                    }
                                }
                            }
                        }
                        if self.local_edit && self.key.is_some() {
                            w.refresh_geometry_view(view);
                        } else {
                            w.set_view(view);
                        }
                        if self.key.is_none() {
                            let operation = doc
                                .rows
                                .iter()
                                .flat_map(|r| &r.tiles)
                                .find(|t| {
                                    matches!(
                                        t.op,
                                        Operation::Contour { .. }
                                            | Operation::Box { .. }
                                            | Operation::Loft { .. }
                                            | Operation::Group { .. }
                                            | Operation::Armature { .. }
                                            | Operation::Profiles { .. }
                                    )
                                })
                                .map(|t| t.id.clone());
                            if let Some(id) = operation {
                                w.select_operation(&id);
                                self.selected_operation = Some(id);
                            }
                        }
                        if let Some(id) = self.scene.select_after_refresh.take() {
                            w.select_operation(&id);
                            self.selected_operation = Some(id);
                        }
                        for (id, image) in thumbnails {
                            w.set_thumbnail(id, image);
                        }
                    }
                    layout::set_choice(ui, "MZ Geometry Tool", self.scene.tool);
                    if ctx
                        .ui
                        .focus
                        .as_ref()
                        .is_none_or(|id| id.name != "MZ Geometry Name")
                    {
                        if let Some(w) = ui.get_widget("MZ Geometry Name") {
                            w.set_value(TheValue::Text(doc.label.clone()));
                        }
                    }
                    let reset = self.playback_asset != Some(id);
                    self.playback_asset = Some(id);
                    if let Err(e) =
                        self.playback
                            .refresh(&root, &self.group_path, &evaluation, reset, ui)
                    {
                        Self::status(ctx, e);
                    }
                    if let Some(row) = self.scene.pending_clip.take() {
                        self.playback.select_row(&row);
                    }
                    self.document = Some(doc);
                    if let Some(w) = ui.get_widget("MZ Group Path").and_then(|w| w.as_text()) {
                        let mut names = vec![root.label.clone()];
                        let mut current = &root;
                        for id in &self.group_path {
                            if let Some(Tile {
                                op:
                                    Operation::Group {
                                        label, document, ..
                                    },
                                ..
                            }) = current
                                .rows
                                .iter()
                                .flat_map(|r| &r.tiles)
                                .find(|t| &t.id == id)
                            {
                                names.push(label.clone());
                                current = document;
                            }
                        }
                        w.set_text(names.join(" / "));
                    }
                    self.root_document = Some(root);
                    self.evaluation = Some(evaluation);
                    self.preview_key.clear();
                }
                Err(e) => {
                    self.document = None;
                    self.evaluation = None;
                    self.selected_operation = None;
                    let background = ui.style.theme().color(MaterialCanvas).to_vec();
                    if let Some(w) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
                        *w.render_buffer_mut() = TheRGBABuffer::from(background, 1, 1);
                    }
                    if let Some(w) = ui
                        .get_widget(EDITOR)
                        .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                    {
                        w.set_view(MaterialEditorView {
                            geometry: true,
                            title: "Could not open geometry asset".into(),
                            ..Default::default()
                        });
                    }
                    Self::status(ctx, e.to_string());
                }
            }
            self.key = Some(self.source_key(project, id, text));
            self.local_edit = false;
            changed = true;
        }
        if self.inspector_dirty {
            if let (Some(doc), Some(e)) = (&self.document, &self.evaluation) {
                if let Some(w) = ui
                    .get_widget(EDITOR)
                    .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                {
                    let mut view =
                        presentation::view_selected(doc, e, &project.palette, &self.inspector);
                    presentation::add_project_sources(&mut view, project, self.selected);
                    w.set_view(view);
                }
            }
            if let Some(id) = self.scene.select_after_refresh.take() {
                if let Some(w) = ui
                    .get_widget(EDITOR)
                    .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                {
                    w.select_operation(&id);
                }
                self.selected_operation = Some(id);
            }
            self.inspector_dirty = false;
            changed = true;
        }
        let can_enter = self.document.as_ref().is_some_and(|doc| {
            doc.rows.iter().flat_map(|r| &r.tiles).any(|t| {
                Some(&t.id) == self.selected_operation.as_ref()
                    && (matches!(t.op, Operation::Group { .. }) || matches!(&t.op,Operation::Asset{source,..} if source!=denrim_materials::geometry::WEARER_SOURCE))
            })
        });
        for (id, disabled) in [
            ("MZ Edit Group", !can_enter),
            ("MZ Parent Group", self.group_path.is_empty()),
        ] {
            if let Some(w) = ui.get_widget(id) {
                w.set_disabled(disabled);
            }
        }
        if ctx
            .ui
            .focus
            .as_ref()
            .is_none_or(|id| id.name != "MZ Joint Name" && id.name != "MZ Profile Name")
        {
            let name = if self.scene.tool == 3 {
                self.document
                    .as_ref()
                    .and_then(scene::profiles)
                    .and_then(|(_, _, p)| p.get(self.inspector.part))
                    .map(|p| p.label.clone())
            } else {
                self.document
                    .as_ref()
                    .and_then(scene::armature)
                    .and_then(|(_, j)| j.get(self.inspector.joint))
                    .map(|j| j.label.clone())
            };
            for id in ["MZ Joint Name", "MZ Profile Name"] {
                if let Some(w) = ui.get_widget(id) {
                    if w.disabled() != name.is_none() {
                        w.set_disabled(name.is_none());
                    }
                    let value = TheValue::Text(name.clone().unwrap_or_default());
                    if w.value() != value {
                        w.set_value(value);
                    }
                }
            }
        }
        layout::sync_camera_options(self, ui);
        layout::sync_controls(self, ui, ctx);
        let dim = ui.get_widget(PREVIEW).map(|w| *w.dim()).unwrap_or_default();
        if dim.width < 16 || dim.height < 16 {
            return changed;
        }
        if let Some(e) = &self.evaluation {
            self.playback.tick(e, ui, ctx);
        }
        if self.camera == 4 && self.group_path.is_empty() {
            let texture = self
                .iso_output
                .and_then(|output| {
                    let source = project.isometric_bakes.get(&output)?;
                    (Some(source.recipe) == self.selected)
                        .then(|| project.tiles.get(&output))
                        .flatten()
                })
                .or_else(|| {
                    self.selected.and_then(|id| {
                        project
                            .isometric_bakes
                            .iter()
                            .find(|(_, b)| b.recipe == id)
                            .and_then(|(tile, _)| project.tiles.get(tile))
                    })
                })
                .and_then(|t| {
                    let layout = t.isometric.as_ref()?;
                    let frames = if layout.frames.is_empty() {
                        &t.textures
                    } else {
                        &layout.frames
                    };
                    let index = if self.playback.selected == 0 {
                        layout.walk_frames
                    } else {
                        (self.playback.phase * layout.walk_frames as f32) as usize
                    };
                    frames.get(index.min(layout.frames_per_direction.saturating_sub(1)))
                });
            if let Some(texture) = texture {
                let checker = [
                    *ui.style.theme().color(MaterialCheckerDark),
                    *ui.style.theme().color(MaterialCheckerLight),
                ];
                if let Some(w) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
                    let buffer = w.render_buffer_mut();
                    buffer.resize(dim.width, dim.height);
                    shared::isometric::draw_preview(buffer, texture, checker);
                }
                return true;
            }
        }
        let bg = *ui.style.theme().color(MaterialCanvas);
        let animation_tick = self
            .evaluation
            .as_ref()
            .and_then(|e| self.playback.motion(e))
            .map_or(0, |m| {
                (self.playback.phase * m.duration * 60.).round() as u64
            });
        let surface_seconds = self
            .evaluation
            .as_ref()
            .and_then(|e| self.playback.motion(e))
            .map_or_else(
                || {
                    if self.playback.playing {
                        self.effect_clock
                            .get_or_insert_with(std::time::Instant::now)
                            .elapsed()
                            .as_secs_f32()
                    } else {
                        self.playback.phase
                    }
                },
                |m| self.playback.phase * m.duration,
            );
        let surface_tick = (surface_seconds * 30.).floor() as u64;
        let effect_tick = if self
            .evaluation
            .as_ref()
            .is_some_and(|e| !e.mesh.effects.is_empty())
        {
            (self
                .effect_clock
                .get_or_insert_with(std::time::Instant::now)
                .elapsed()
                .as_secs_f32()
                * 30.)
                .floor() as u64
        } else {
            0
        };
        let key = format!(
            "{}:{:?}:{}:{}:{}:{:?}:{:?}:{}:{}:{}:{}",
            self.source_revision,
            self.selected_operation,
            self.camera,
            self.yaw,
            self.pitch,
            (dim.width, dim.height),
            bg,
            self.preview_selected,
            self.playback.selected,
            animation_tick,
            self.playback.show_rig
        ) + &format!(":effect:{effect_tick}:surface:{surface_tick}")
            + &format!(
                ":{}:{}:{}:{}",
                self.scene.tool, self.inspector.joint, self.inspector.part, self.inspector.section
            );
        let context = format!(
            "{}:{:?}:{:?}:{}:{:?}:{:?}:{}:{}",
            self.source_revision,
            self.selected_operation,
            (dim.width, dim.height),
            self.preview_selected,
            bg,
            self.group_path,
            self.scene.tool,
            self.playback.selected
        );
        if context != self.preview_context {
            self.preview_epoch += 1;
            self.preview_context = context;
        }
        if key != self.preview_key {
            self.preview_key = key;
            self.revision += 1;
            let points = self
                .document
                .as_ref()
                .and_then(|d| {
                    d.rows
                        .iter()
                        .flat_map(|r| &r.tiles)
                        .find(|t| Some(&t.id) == self.selected_operation.as_ref())
                })
                .map(|t| rigging::profile(&t.op));
            if self.camera == 3 {
                let (image, frame) = presentation::contour_image(
                    points.as_deref().unwrap_or(&[]),
                    dim.width,
                    dim.height,
                    ui.style.theme().as_ref(),
                    self.contour_frame.as_ref().filter(|_| self.drag.is_some()),
                );
                self.contour_frame = frame;
                if let Some(w) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
                    *w.render_buffer_mut() = image;
                }
                changed = true;
            } else if let Some(evaluation) = &self.evaluation {
                #[cfg(not(target_arch = "wasm32"))]
                if self.worker.is_none() {
                    if let Some((device, queue)) =
                        RUSTERIX.read().unwrap().scene_handler.vm.gpu_context()
                    {
                        self.worker = Some(preview::Worker::new(device, queue));
                    }
                }
                if let Some(worker) = &self.worker {
                    let mesh = if self.preview_selected {
                        self.selected_operation
                            .as_ref()
                            .and_then(|id| evaluation.operations.get(id))
                            .and_then(|value| match value {
                                Value::Mesh(m) => Some(m.clone()),
                                _ => None,
                            })
                            .unwrap_or_else(|| evaluation.mesh.clone())
                    } else {
                        evaluation.mesh.clone()
                    };
                    let motion = self.playback.motion(evaluation);
                    let posed = mesh
                        .posed(
                            motion,
                            motion.map_or(0., |m| m.duration * self.playback.phase),
                        )
                        .ok();
                    let joints = posed.as_ref().map(|p| p.joints.clone()).unwrap_or_default();
                    let bones = if self.playback.show_rig {
                        mesh.rig
                            .as_ref()
                            .zip(posed.as_ref())
                            .map(|(r, p)| {
                                r.joints
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(i, j)| {
                                        j.parent.map(|parent| {
                                            (p.joints[parent].translation, p.joints[i].translation)
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default()
                    } else {
                        vec![]
                    };
                    let attachment_frames = if self.playback.show_rig {
                        posed
                            .as_ref()
                            .map(|p| p.attachments.values().cloned().collect())
                            .unwrap_or_default()
                    } else {
                        vec![]
                    };
                    let bounds = self
                        .scene
                        .drag
                        .as_ref()
                        .map(|d| d.frame.bounds)
                        .or(if self.preview_selected {
                            None
                        } else {
                            self.playback.framing
                        })
                        .unwrap_or_else(|| mesh.bounds());
                    let bounds =
                        rusterix::procedural_avatar::effects::preview_bounds(&mesh, bounds);
                    let bounds = if self.scene.tool > 0 {
                        *self.scene.view_bounds.get_or_insert(bounds)
                    } else {
                        bounds
                    };
                    let projection =
                        scene::Projection::new(bounds, dim.width, dim.height, self.yaw, self.pitch);
                    worker.submit(preview::Request {
                        epoch: self.preview_epoch,
                        projection,
                        posed: joints,
                        job: preview::Job {
                            surface_seconds,
                            revision: self.revision,
                            mesh: posed.map_or(mesh, |p| p.mesh),
                            framing: Some(bounds),
                            bones,
                            attachment_frames,
                            palette: project.palette.clone(),
                            width: dim.width as u32,
                            height: dim.height as u32,
                            yaw: self.yaw,
                            pitch: self.pitch,
                            background: bg,
                            grip: evaluation.grip.clone(),
                            axes: std::array::from_fn(|i| {
                                ui.style
                                    .theme()
                                    .palette_color(TheThemePalettes::MaterialCategories, i)
                            }),
                        },
                    });
                } else {
                    Self::status(ctx, "Geometry preview requires the desktop GPU renderer");
                }
            }
        }
        if let Some(worker) = &self.worker {
            for frame in worker.results.try_iter() {
                if !self.accepts_frame(frame.epoch, frame.revision) {
                    continue;
                }
                match frame.image {
                    Ok(mut image) => {
                        self.displayed_revision = frame.revision;
                        self.scene.projection = Some(frame.projection);
                        self.scene.posed = frame.posed;
                        self.scene.previewing_animation = self.playback.selected > 0;
                        if let (Some(doc), Some(e)) = (&self.document, &self.evaluation) {
                            self.scene.paint(
                                &mut image,
                                doc,
                                e,
                                &self.inspector,
                                ui.style.theme().as_ref(),
                            );
                        }
                        if let Some(w) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
                            *w.render_buffer_mut() = image;
                        }
                        changed = true;
                    }
                    Err(e) => Self::status(ctx, e),
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod visual_tests;

#[cfg(test)]
mod viewport_tests;

fn fingerprint<T: std::hash::Hash>(value: &T) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}
