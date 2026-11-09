//! A data-driven entry point to ordinary MZ documents, not per-asset actions.
use super::*;

const TITLE: &str = "Mesh recipes";
const LIST: &str = "MZ Recipe List";
const FORM: &str = "MZ Recipe Controls";
const VIEW: &str = "MZ Recipe Preview";
const CREATE: &str = "MZ Recipe Create";
const CATEGORIES: &[&str] = &[
    "All",
    "Building parts",
    "Nature",
    "Props",
    "Primitives",
    "Project",
];

pub(super) struct Entry {
    pub id: TheId,
    pub label: String,
    pub category: String,
    pub description: String,
    pub document: Document,
}
pub(super) struct Browser {
    entries: Vec<Entry>,
    selected: Option<usize>,
    draft: Option<Document>,
    name: String,
    query: String,
    category: usize,
    error: Option<String>,
}

pub(super) fn bundled() -> Vec<Entry> {
    macro_rules! recipe {
        ($file:literal,$category:literal,$description:literal) => {{
            let document = Document::parse(include_str!(concat!(
                "../../embedded/meshes/recipes/",
                $file,
                ".mzgeometry.json"
            )))
            .unwrap();
            Entry {
                id: TheId::named($file),
                label: document.label.clone(),
                category: $category.into(),
                description: $description.into(),
                document,
            }
        }};
    }
    vec![
        recipe!(
            "box",
            "Primitives",
            "A grounded box for starting a custom prop."
        ),
        recipe!(
            "wall-segment",
            "Building parts",
            "A reusable straight wall, with editable dimensions and color."
        ),
        recipe!(
            "wall-corner",
            "Building parts",
            "Two wall arms meeting at a grounded corner."
        ),
        recipe!(
            "roof-slope",
            "Building parts",
            "One sloping roof part. Assemble building components on the map."
        ),
        recipe!(
            "repeated-posts",
            "Building parts",
            "Repeated posts with recipe-driven count, spacing and dimensions."
        ),
        recipe!(
            "rock",
            "Nature",
            "A faceted rock built from editable loft rings."
        ),
        recipe!(
            "tree",
            "Nature",
            "A trunk and three foliage clusters, with separate palette controls."
        ),
        recipe!(
            "doorway",
            "Building parts",
            "A lintel and two jambs, with separate recipe footprints so the doorway stays open."
        ),
        recipe!(
            "ring-walkway",
            "Building parts",
            "A top-anchored ring with an editable opening. Set Top height to raise its walkable surface."
        ),
        recipe!(
            "gable-roof",
            "Building parts",
            "Two roof slopes with a joined, mitered ridge. Panel length follows span and pitch."
        ),
        recipe!(
            "fence",
            "Building parts",
            "Repeated posts and continuous rails. Fence length is calculated from count × spacing."
        ),
        recipe!("barrel", "Props", "A lofted barrel with two metal hoops."),
        recipe!(
            "crate",
            "Props",
            "A wooden crate with separate body and frame colors."
        ),
    ]
}

pub(super) fn controls(doc: &Document) -> Option<&Vec<denrim_materials::geometry::group::Control>> {
    doc.rows
        .iter()
        .flat_map(|r| &r.tiles)
        .find_map(|t| match &t.op {
            Operation::Group { controls, .. } => Some(controls),
            _ => None,
        })
}
fn controls_mut(
    doc: &mut Document,
) -> Option<&mut Vec<denrim_materials::geometry::group::Control>> {
    doc.rows
        .iter_mut()
        .flat_map(|r| &mut r.tiles)
        .find_map(|t| match &mut t.op {
            Operation::Group { controls, .. } => Some(controls),
            _ => None,
        })
}

impl Browser {
    fn new(project: &Project) -> Self {
        let mut entries = bundled();
        let selection = shared::rulesets::selected_ruleset_config(&project.config);
        let library: std::collections::HashMap<_, _> =
            shared::rulesets::bundled_appearance_assets_for_ruleset(
                &selection.id,
                &selection.version,
            )
            .into_iter()
            .filter_map(|a| Uuid::parse_str(a.asset_id).ok().map(|id| (id, a.source)))
            .collect();
        let mut own: Vec<_> = project
            .procedural_appearances
            .iter()
            .filter(|(id, source)| {
                library
                    .get(id)
                    .is_none_or(|original| *original != source.as_str())
            })
            .filter_map(|(id, source)| {
                Document::parse(source).ok().map(|document| Entry {
                    id: TheId::named_with_id("MZ Project Recipe", *id),
                    label: document.label.clone(),
                    category: "Project".into(),
                    description: "Create an independent editable copy of this project recipe."
                        .into(),
                    document,
                })
            })
            .collect();
        own.sort_by(|a, b| a.label.cmp(&b.label));
        entries.extend(own);
        Self {
            entries,
            selected: None,
            draft: None,
            name: String::new(),
            query: String::new(),
            category: 0,
            error: None,
        }
    }
    fn visible(&self, entry: &Entry) -> bool {
        (self.category == 0 || entry.category == CATEGORIES[self.category])
            && format!("{} {} {}", entry.label, entry.category, entry.description)
                .to_lowercase()
                .contains(&self.query.to_lowercase())
    }
    fn select(&mut self, index: usize, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        if let Some(list) = ui.get_list_layout(LIST) {
            list.select_item(self.entries[index].id.uuid, ctx, false);
        }
        self.selected = Some(index);
        self.draft = Some(self.entries[index].document.clone());
        self.name = self.entries[index].label.clone();
        if let Some(w) = ui.get_widget("MZ Recipe Name") {
            w.set_value(TheValue::Text(self.name.clone()));
        }
        if let Some(w) = ui.get_widget("MZ Recipe Description") {
            w.set_value(TheValue::Text(self.entries[index].description.clone()));
        }
        if let Some(form) = ui.get_text_layout(FORM) {
            form.clear();
            if let Some(controls) = self.draft.as_ref().and_then(controls) {
                for (i, c) in controls.iter().enumerate() {
                    let id = TheId::named(&format!("MZ Recipe Control {i}"));
                    if c.is_palette() {
                        let mut picker = ThePaletteIndexPicker::new(id);
                        picker.set_palette(project.palette.clone());
                        picker.set_selected_index(c.value as i32);
                        form.add_pair(c.label.clone(), Box::new(picker));
                    } else {
                        let mut edit = TheTextLineEdit::new(id);
                        if c.is_integer() {
                            edit.set_value(TheValue::Int(c.value as i32));
                            edit.set_range(TheValue::RangeI32(
                                c.min.ceil() as i32..=c.max.floor() as i32,
                            ));
                        } else {
                            edit.set_value(TheValue::Float(c.value));
                            edit.set_range(TheValue::RangeF32(c.min..=c.max));
                        }
                        edit.set_status_text(&c.help);
                        edit.set_disabled(c.expression.is_some());
                        form.add_pair(c.label.clone(), Box::new(edit));
                    }
                }
            }
        }
        self.preview(ui, ctx, project);
        ctx.ui.relayout = true;
    }
    fn filter(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        let visible: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| self.visible(e))
            .map(|(i, _)| i)
            .collect();
        if let Some(list) = ui.get_list_layout(LIST) {
            list.clear();
            for &i in &visible {
                let entry = &self.entries[i];
                let mut item = TheListItem::new(entry.id.clone());
                item.set_text(entry.label.clone());
                item.set_sub_text(entry.category.clone());
                if entry.category != "Project" {
                    if let Ok(evaluation) = entry.document.evaluate() {
                        item.set_icon(
                            presentation::thumbnail(
                                &Value::Mesh(evaluation.mesh),
                                &project.palette,
                                &TheDarkTheme::new(),
                            )
                            .scaled(40, 40),
                        );
                    }
                }
                list.add_item(item, ctx);
            }
        }
        let selected = self
            .selected
            .filter(|i| visible.contains(i))
            .or_else(|| visible.first().copied());
        if let Some(index) = selected {
            if self.selected != Some(index) {
                self.select(index, ui, ctx, project);
            }
            if let Some(list) = ui.get_list_layout(LIST) {
                list.select_item(self.entries[index].id.uuid, ctx, false);
            }
        } else {
            self.selected = None;
            self.draft = None;
            if let Some(form) = ui.get_text_layout(FORM) {
                form.clear();
            }
            if let Some(w) = ui.get_widget("MZ Recipe Description") {
                w.set_value(TheValue::Text("No matching recipes".into()));
            }
            self.preview(ui, ctx, project);
        }
        ctx.ui.relayout = true;
    }
    fn preview(&mut self, ui: &mut TheUI, ctx: &mut TheContext, project: &Project) {
        self.error = None;
        if let Some(controls) = self.draft.as_ref().and_then(controls) {
            if let Ok(resolved) = denrim_materials::geometry::expressions::resolve(controls) {
                for (i, c) in resolved
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.expression.is_some())
                {
                    if let Some(w) = ui.get_widget(&format!("MZ Recipe Control {i}")) {
                        w.set_value(TheValue::Float(c.value));
                    }
                }
            }
        }
        let image = self
            .draft
            .as_ref()
            .map(|doc| {
                let doc = shared::appearance_assets::resolve_dependencies(project, doc)?;
                let evaluation = doc.evaluate().map_err(|e| e.to_string())?;
                Ok::<_, String>(presentation::thumbnail(
                    &Value::Mesh(evaluation.mesh),
                    &project.palette,
                    &TheDarkTheme::new(),
                ))
            })
            .transpose();
        let image = match image {
            Ok(image) => image,
            Err(error) => {
                self.error = Some(error);
                None
            }
        };
        if let Some(view) = ui.get_render_view(VIEW) {
            let dim = *view.dim();
            let width = dim.width.max(1);
            let height = dim.height.max(1);
            let buffer = view.render_buffer_mut();
            buffer.resize(width, height);
            buffer.fill([30, 32, 35, 255]);
            if let Some(image) = image {
                let side = width.min(height).min(256);
                buffer.copy_into(
                    (width - side) / 2,
                    (height - side) / 2,
                    &image.scaled(side, side),
                );
            }
        }
        if let Some(w) = ui.get_widget(CREATE) {
            w.set_disabled(
                self.draft.is_none() || self.error.is_some() || self.name.trim().is_empty(),
            );
        }
        if let Some(w) = ui.get_widget("MZ Recipe Error") {
            w.set_value(TheValue::Text(self.error.clone().unwrap_or_default()));
        }
        ctx.ui.redraw_all = true;
    }
    fn canvas(&self) -> TheCanvas {
        let mut search = TheTextLineEdit::new(TheId::named("MZ Recipe Search"));
        search.limiter_mut().set_max_width(460);
        search.set_status_text("Search mesh recipes");
        let mut category = TheDropdownMenu::new(TheId::named("MZ Recipe Category"));
        for name in CATEGORIES {
            category.add_option((*name).into());
        }
        category.limiter_mut().set_max_width(145);
        let mut bar = TheHLayout::new(TheId::empty());
        bar.set_margin(Vec4::new(8, 6, 8, 6));
        bar.set_padding(8);
        bar.limiter_mut().set_max_height(38);
        let mut label = TheText::new(TheId::empty());
        label.set_text("Search".into());
        bar.add_widget(Box::new(label));
        bar.add_widget(Box::new(search));
        bar.add_widget(Box::new(category));
        let mut top = TheCanvas::new();
        top.set_layout(bar);
        let mut left = TheCanvas::new();
        left.limiter_mut().set_max_width(240);
        let mut list = TheListLayout::new(TheId::named(LIST));
        list.limiter_mut().set_max_width(240);
        list.set_item_size(48);
        left.set_layout(list);
        let mut right = TheCanvas::new();
        let mut heading = TheVLayout::new(TheId::empty());
        heading.set_margin(Vec4::new(10, 6, 10, 6));
        heading.set_padding(6);
        heading.limiter_mut().set_max_height(96);
        let mut name = TheTextLineEdit::new(TheId::named("MZ Recipe Name"));
        name.limiter_mut().set_max_width(440);
        name.set_status_text("Name of the new editable mesh");
        heading.add_widget(Box::new(name));
        let mut text = TheText::new(TheId::named("MZ Recipe Description"));
        text.set_text_size(12.);
        text.limiter_mut().set_max_height(42);
        heading.add_widget(Box::new(text));
        let mut header = TheCanvas::new();
        header.set_layout(heading);
        right.set_top(header);
        let mut preview = TheCanvas::new();
        preview.limiter_mut().set_max_height(176);
        let mut view = TheRenderView::new(TheId::named(VIEW));
        view.limiter_mut().set_max_size(Vec2::new(i32::MAX, 176));
        preview.set_widget(view);
        let mut details = TheCanvas::new();
        details.set_top(preview);
        let mut form = TheTextLayout::new(TheId::named(FORM));
        form.set_fixed_text_width(118);
        form.set_padding(6);
        details.set_layout(form);
        right.set_center(details);
        let mut footer = TheVLayout::new(TheId::empty());
        footer.limiter_mut().set_max_height(85);
        let mut sentinel = TheText::new(TheId::named("Dialog Value"));
        sentinel.set_text("Editable MZ construction".into());
        sentinel.set_text_size(11.);
        sentinel.limiter_mut().set_max_height(18);
        footer.add_widget(Box::new(sentinel));
        let mut error = TheText::new(TheId::named("MZ Recipe Error"));
        error.set_text_size(11.);
        footer.add_widget(Box::new(error));
        let mut create = TheTraybarButton::new(TheId::named(CREATE));
        create.set_text("Create mesh".into());
        footer.add_widget(Box::new(create));
        let mut bottom = TheCanvas::new();
        bottom.set_layout(footer);
        right.set_bottom(bottom);
        let mut canvas = TheCanvas::new();
        canvas.limiter_mut().set_max_size(Vec2::new(720, 610));
        canvas.set_top(top);
        canvas.set_left(left);
        canvas.set_center(right);
        canvas
    }
}

impl GeometryEditor {
    fn install_browser_recipe(
        &mut self,
        mut doc: Document,
        name: &str,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Result<Uuid, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a mesh name".into());
        }
        let id = Uuid::new_v4();
        doc.id = id.to_string();
        doc.label = name.into();
        shared::appearance_assets::resolve_dependencies(project, &doc)?
            .evaluate()
            .map_err(|e| e.to_string())?;
        let text = doc.save().map_err(|e| e.to_string())?;
        project.procedural_appearances.insert(id, text.clone());
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProceduralAppearanceEdit(id, None, Some(text)),
            ctx,
        );
        self.active = true;
        self.selected = Some(id);
        self.iso_output = None;
        self.group_path.clear();
        self.key = None;
        self.scene = Default::default();
        self.open_in_rig = false;
        self.selected_operation = doc
            .rows
            .iter()
            .flat_map(|r| &r.tiles)
            .find(|t| matches!(t.op, Operation::Group { .. }))
            .map(|t| t.id.clone());
        self.inspector = Default::default();
        self.preview_selected = false;
        Self::mode(ui, ctx, 1);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Procedural Tiles Restored"),
            TheValue::Empty,
        ));
        Ok(id)
    }
    pub(super) fn recipe_browser_event(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        if matches!(event,TheEvent::StateChanged(id,TheWidgetState::Clicked) if id.name=="MZ New Box")
        {
            self.finish_drag(project, ctx, false);
            let mut browser = Browser::new(project);
            ui.show_dialog(
                TITLE,
                browser.canvas(),
                vec![TheDialogButtonRole::Reject],
                ctx,
            );
            browser.filter(ui, ctx, project);
            self.recipe_browser = Some(browser);
            return true;
        }
        if ui.dialog_text != TITLE || ui.dialog.is_none() {
            self.recipe_browser = None;
            return false;
        }
        let Some(mut browser) = self.recipe_browser.take() else {
            return false;
        };
        let mut handled = true;
        match event {
            TheEvent::WidgetResized(id, _) if id.name == VIEW => {
                browser.preview(ui, ctx, project);
            }
            TheEvent::ValueChanged(id, value) if id.name == "MZ Recipe Search" => {
                browser.query = value.to_string().unwrap_or_default();
                browser.filter(ui, ctx, project);
            }
            TheEvent::ValueChanged(id, value) if id.name == "MZ Recipe Name" => {
                browser.name = value.to_string().unwrap_or_default();
                browser.preview(ui, ctx, project);
            }
            TheEvent::IndexChanged(id, index) if id.name == "MZ Recipe Category" => {
                browser.category = (*index as usize).min(CATEGORIES.len() - 1);
                browser.filter(ui, ctx, project);
            }
            TheEvent::NewListItemSelected(id, list) if list.name == LIST => {
                if let Some(i) = browser.entries.iter().position(|e| e.id.uuid == id.uuid) {
                    if browser.selected != Some(i) {
                        browser.select(i, ui, ctx, project);
                    }
                }
            }
            TheEvent::ValueChanged(id, value) if id.name.starts_with("MZ Recipe Control ") => {
                if let (Some(i), Some(value)) = (
                    id.name
                        .strip_prefix("MZ Recipe Control ")
                        .and_then(|s| s.parse::<usize>().ok()),
                    value.to_f32().or_else(|| value.to_i32().map(|v| v as f32)),
                ) {
                    if let Some(c) = browser
                        .draft
                        .as_mut()
                        .and_then(controls_mut)
                        .and_then(|c| c.get_mut(i))
                    {
                        if c.expression.is_none() {
                            c.value = if c.is_integer() { value.round() } else { value };
                        }
                    }
                    browser.preview(ui, ctx, project);
                }
            }
            TheEvent::PaletteIndexChanged(id, index)
                if id.name.starts_with("MZ Recipe Control ") =>
            {
                if let Some(c) = id
                    .name
                    .strip_prefix("MZ Recipe Control ")
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|i| {
                        browser
                            .draft
                            .as_mut()
                            .and_then(controls_mut)
                            .and_then(|c| c.get_mut(i))
                    })
                {
                    if c.expression.is_none() {
                        c.value = *index as f32;
                    }
                }
                browser.preview(ui, ctx, project);
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked) if id.name == CREATE => {
                if let Some(doc) = browser.draft.clone() {
                    match self.install_browser_recipe(doc, &browser.name, project, ui, ctx) {
                        Ok(_) => {
                            ui.clear_dialog();
                            ctx.ui.relayout = true;
                            return true;
                        }
                        Err(error) => {
                            browser.error = Some(error.clone());
                            if let Some(w) = ui.get_widget("MZ Recipe Error") {
                                w.set_value(TheValue::Text(error));
                            }
                        }
                    }
                }
            }
            _ => handled = false,
        }
        self.recipe_browser = Some(browser);
        handled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recipe_browser_templates_are_grounded_editable_geometry_and_bake_to_iso() {
        let mut project = Project::default();
        project.regions[0].map.camera = rusterix::MapCamera::TwoDIso;
        for entry in bundled() {
            let evaluation = entry
                .document
                .evaluate()
                .unwrap_or_else(|e| panic!("{}: {e}", entry.label));
            assert!(evaluation.mesh.vertex_count() > 0);
            assert!(evaluation.mesh.rig.is_none());
            let (lo, hi) = evaluation.mesh.bounds();
            if entry.document.id == "ring-walkway" {
                assert!(
                    hi[1].abs() < 0.01 && lo[1] < 0.,
                    "Walkable ring must be anchored at its top: {lo:?} {hi:?}"
                );
            } else {
                assert!(
                    lo[1].abs() < 0.01,
                    "{} must start at ground: {lo:?}",
                    entry.label
                );
            }
            let settings = controls(&entry.document).unwrap();
            for c in settings {
                if let [denrim_materials::geometry::group::Modifier::FitAxis { axis }] =
                    c.modifiers.as_slice()
                {
                    assert!(
                        (hi[*axis] - lo[*axis] - c.value).abs() < 0.001,
                        "{} axis {axis}",
                        entry.label
                    );
                }
            }
            assert_eq!(
                Document::load(&entry.document.save().unwrap()).unwrap(),
                entry.document
            );
            let id = Uuid::new_v4();
            project
                .procedural_appearances
                .insert(id, entry.document.save().unwrap());
        }
        assert!(shared::isometric::synchronize(&mut project).is_empty());
        for id in project.procedural_appearances.keys() {
            assert!(project.tiles.get(id).is_some_and(|t| t.isometric.is_some()));
            assert!(
                matches!(shared::isometric::tile_recipe(&project,*id),Some(shared::isometric::TileRecipe::Geometry(recipe)) if recipe==*id)
            );
        }
    }

    #[test]
    fn recipe_browser_creates_an_independent_named_source_and_keeps_draft_changes_local() {
        let mut project = Project::default();
        let mut editor = GeometryEditor::default();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1000, 760, 1.);
        let source = bundled().remove(1).document;
        let mut draft = source.clone();
        controls_mut(&mut draft).unwrap()[0].value = 2.5;
        let id = editor
            .install_browser_recipe(draft, "Long wall", &mut project, &mut ui, &mut ctx)
            .unwrap();
        let created = Document::load(&project.procedural_appearances[&id]).unwrap();
        assert_eq!(created.label, "Long wall");
        assert_eq!(created.id, id.to_string());
        assert!(
            (created.evaluate().unwrap().mesh.bounds().1[0]
                - created.evaluate().unwrap().mesh.bounds().0[0]
                - 2.5)
                .abs()
                < 0.001
        );
        assert_eq!(controls(&source).unwrap()[0].value, 1.);
        assert!(project.tiles.is_empty());
        let bytes = shared::project_io::encode_project(&project).unwrap();
        assert_eq!(
            shared::project_io::decode_project(&bytes)
                .unwrap()
                .procedural_appearances[&id],
            project.procedural_appearances[&id]
        );
        let browser = Browser::new(&project);
        assert!(
            browser
                .entries
                .iter()
                .any(|e| e.category == "Project" && e.label == "Long wall")
        );
        let atom = ProjectUndoAtom::ProceduralAppearanceEdit(
            id,
            None,
            Some(project.procedural_appearances[&id].clone()),
        );
        let mut server = ServerContext::default();
        atom.undo(&mut project, &mut ui, &mut ctx, &mut server);
        assert!(!project.procedural_appearances.contains_key(&id));
        atom.redo(&mut project, &mut ui, &mut ctx, &mut server);
        assert!(project.procedural_appearances.contains_key(&id));
    }

    #[test]
    fn recipe_browser_search_and_category_do_not_rewrite_recipes() {
        let project = Project::default();
        let mut browser = Browser::new(&project);
        browser.category = 1;
        browser.query = "wall".into();
        let matches = browser
            .entries
            .iter()
            .filter(|e| browser.visible(e))
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 2);
        browser.query = "no such mesh".into();
        assert!(!browser.entries.iter().any(|e| browser.visible(e)));
        assert!(project.procedural_appearances.is_empty());
    }
    #[test]
    fn recipe_browser_displays_and_recomputes_derived_values_without_rewriting_formulas() {
        let mut project = Project::default();
        let mut editor = GeometryEditor::default();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1000, 760, 1.);
        ui.init(&mut ctx);
        editor.handle_event(
            &TheEvent::StateChanged(TheId::named("MZ New Box"), TheWidgetState::Clicked),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        let fence = editor
            .recipe_browser
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .find(|e| e.document.id == "fence")
            .unwrap()
            .id
            .clone();
        editor.handle_event(
            &TheEvent::NewListItemSelected(fence, TheId::named(LIST)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        let draft = editor
            .recipe_browser
            .as_ref()
            .unwrap()
            .draft
            .as_ref()
            .unwrap();
        let length = controls(draft)
            .unwrap()
            .iter()
            .position(|c| c.id == "length")
            .unwrap();
        editor.handle_event(
            &TheEvent::ValueChanged(TheId::named("MZ Recipe Control 0"), TheValue::Int(8)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(
            (ui.get_widget_value(&format!("MZ Recipe Control {length}"))
                .unwrap()
                .to_f32()
                .unwrap()
                - 2.8)
                .abs()
                < 1e-5
        );
        editor.handle_event(
            &TheEvent::ValueChanged(
                TheId::named(&format!("MZ Recipe Control {length}")),
                TheValue::Float(9.),
            ),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        let draft = editor
            .recipe_browser
            .as_ref()
            .unwrap()
            .draft
            .as_ref()
            .unwrap();
        let c = &controls(draft).unwrap()[length];
        assert_eq!(c.expression.as_deref(), Some("count * spacing"));
        assert!((c.value - 2.1).abs() < 1e-5);
        assert!(
            (ui.get_widget_value(&format!("MZ Recipe Control {length}"))
                .unwrap()
                .to_f32()
                .unwrap()
                - 2.8)
                .abs()
                < 1e-5
        );
    }

    #[test]
    #[ignore = "writes native UI review captures under ignored generated/"]
    fn recipe_browser_and_publisher_native_dialog_review() {
        let (mut ui, mut ctx) = super::super::viewport_tests::fixture(1100, 800);
        ui.init(&mut ctx);
        let rx = ui.add_state_listener("recipe browser review".into());
        ui.canvas.set_dim(TheDim::new(0, 0, 1100, 800), &mut ctx);
        let mut editor = GeometryEditor::default();
        let mut project = Project::default();
        project.regions[0].map.camera = rusterix::MapCamera::TwoDIso;
        editor.handle_event(
            &TheEvent::StateChanged(TheId::named("MZ New Box"), TheWidgetState::Clicked),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        let corner = editor
            .recipe_browser
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .find(|e| e.label == "Wall corner")
            .unwrap()
            .id
            .clone();
        editor.handle_event(
            &TheEvent::NewListItemSelected(corner, TheId::named(LIST)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        fn capture(
            editor: &mut GeometryEditor,
            project: &mut Project,
            ui: &mut TheUI,
            ctx: &mut TheContext,
            rx: &std::sync::mpsc::Receiver<TheEvent>,
            path: &str,
        ) {
            let mut pixels = vec![0; 1100 * 800 * 4];
            for _ in 0..3 {
                ui.draw(&mut pixels, ctx);
                ui.process_events(ctx);
                while let Ok(e) = rx.try_recv() {
                    editor.handle_event(&e, project, ui, ctx);
                }
            }
            std::fs::create_dir_all("../generated/mz-recipe-browser").unwrap();
            std::fs::write(
                format!("../generated/mz-recipe-browser/{path}.png"),
                TheRGBABuffer::from(pixels, 1100, 800).to_png().unwrap(),
            )
            .unwrap();
        }
        capture(&mut editor, &mut project, &mut ui, &mut ctx, &rx, "browser");
        let dialog = ui.dialog.as_ref().unwrap().dim;
        for name in [
            "MZ Recipe Category",
            "MZ Recipe Name",
            "MZ Recipe Control 0",
            CREATE,
            VIEW,
        ] {
            let d = *ui.get_widget(name).unwrap().dim();
            assert!(d.width > 0 && d.height > 0, "{name}: {d:?}");
            assert!(
                d.x >= dialog.x && d.x + d.width <= dialog.x + dialog.width,
                "{name}: {d:?}"
            );
        }
        let buffer = ui.get_render_view(VIEW).unwrap().render_buffer_mut();
        assert!(buffer.pixels().chunks(4).any(|p| p != [30, 32, 35, 255]));
        editor.handle_event(
            &TheEvent::StateChanged(TheId::named(CREATE), TheWidgetState::Clicked),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(ui.dialog.is_none());
        let id = editor.selected.unwrap();
        assert!(shared::isometric::synchronize(&mut project).is_empty());
        let original_tile = project.tiles[&id].clone();
        editor.handle_event(
            &TheEvent::IndexChanged(TheId::named("MZ Geometry Actions"), 1),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        capture(
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
            &rx,
            "publisher",
        );
        for name in [
            "MZ Control Label",
            "MZ Control Value",
            "MZ Control Maximum",
            "MZ Control Save",
        ] {
            let d = *ui.get_widget(name).unwrap().dim();
            assert!(d.width > 0 && d.height > 0, "{name}: {d:?}");
        }
        editor.handle_event(
            &TheEvent::StateChanged(
                TheId::named("MZ Control Choose Parameter"),
                TheWidgetState::Clicked,
            ),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        capture(&mut editor, &mut project, &mut ui, &mut ctx, &rx, "inputs");
        let d = *ui.get_widget("MZ Parameter Search").unwrap().dim();
        let point = Vec2::new(d.x + d.width / 2, d.y + d.height / 2);
        assert_eq!(
            ui.get_widget_at_coord(point).unwrap().id().name,
            "MZ Parameter Search"
        );
        let list = *ui.get_layout("MZ Parameter Choices").unwrap().dim();
        assert_eq!(
            ui.get_layout_at_coord(Vec2::new(list.x + 20, list.y + 20))
                .unwrap()
                .name,
            "MZ Parameter Choices"
        );
        ui.hover((list.x + 20) as f32, (list.y + 20) as f32, &mut ctx);
        assert!(ui.mouse_wheel((0, -180), &mut ctx));
        editor.handle_event(
            &TheEvent::ValueChanged(
                TheId::named("MZ Parameter Search"),
                TheValue::Text("Wall X / Box / Width".into()),
            ),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        capture(
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
            &rx,
            "inputs-filtered",
        );
        let d = *ui.get_layout("MZ Parameter Choices").unwrap().dim();
        ui.touch_down((d.x + 30) as f32, (d.y + 22) as f32, &mut ctx);
        ui.touch_up((d.x + 30) as f32, (d.y + 22) as f32, &mut ctx);
        ui.process_events(&mut ctx);
        while let Ok(e) = rx.try_recv() {
            editor.handle_event(&e, &mut project, &mut ui, &mut ctx);
        }
        assert!(ui.popover.is_none());
        assert!(ui.dialog.is_some());
        editor.handle_event(
            &TheEvent::ValueChanged(TheId::named("MZ Control Value"), TheValue::Float(2.5)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.handle_event(
            &TheEvent::StateChanged(TheId::named("MZ Control Save"), TheWidgetState::Clicked),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(ui.dialog.is_none());
        assert!(shared::isometric::synchronize(&mut project).is_empty());
        assert_ne!(original_tile.isometric, project.tiles[&id].isometric);
        assert!(
            matches!(shared::isometric::tile_recipe(&project,id),Some(shared::isometric::TileRecipe::Geometry(recipe)) if recipe == id)
        );

        let fence = bundled()
            .into_iter()
            .find(|e| e.document.id == "fence")
            .unwrap()
            .document;
        let length = controls(&fence)
            .unwrap()
            .iter()
            .position(|c| c.id == "length")
            .unwrap();
        editor
            .install_browser_recipe(fence, "Derived fence", &mut project, &mut ui, &mut ctx)
            .unwrap();
        editor.handle_event(
            &TheEvent::IndexChanged(TheId::named("MZ Geometry Actions"), 1),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.handle_event(
            &TheEvent::NewListItemSelected(
                TheId::named(&format!("MZ Published Control {length}")),
                TheId::named("MZ Published Controls"),
            ),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        capture(
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
            &rx,
            "derived-publisher",
        );
        for name in [
            "MZ Control Expression",
            "MZ Control Insert Reference",
            "MZ Control Value",
        ] {
            let d = *ui.get_widget(name).unwrap().dim();
            assert!(d.width > 0 && d.height > 0, "{name}: {d:?}");
        }
        editor.handle_event(
            &TheEvent::StateChanged(
                TheId::named("MZ Control Insert Reference"),
                TheWidgetState::Clicked,
            ),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        capture(
            &mut editor,
            &mut project,
            &mut ui,
            &mut ctx,
            &rx,
            "references",
        );
        assert!(ui.get_widget("MZ Reference Choice 0").is_some());
        let search = *ui.get_widget("MZ Parameter Search").unwrap().dim();
        assert_eq!(
            ui.get_widget_at_coord(Vec2::new(search.x + 10, search.y + 10))
                .unwrap()
                .id()
                .name,
            "MZ Parameter Search"
        );
    }
    #[test]
    fn parameter_recipes_keep_wall_joins_roof_thickness_and_actual_repeated_parts() {
        fn recipe(id: &str) -> Document {
            bundled()
                .into_iter()
                .find(|e| e.document.id == id)
                .unwrap()
                .document
        }
        fn value(d: &mut Document, id: &str, value: f32) {
            controls_mut(d)
                .unwrap()
                .iter_mut()
                .find(|c| c.id == id)
                .unwrap()
                .value = value;
        }
        fn bounds(p: &denrim_materials::geometry::Part) -> ([f32; 3], [f32; 3]) {
            let lo = std::array::from_fn(|k| {
                p.vertices
                    .iter()
                    .map(|v| v.position[k])
                    .fold(f32::INFINITY, f32::min)
            });
            let hi = std::array::from_fn(|k| {
                p.vertices
                    .iter()
                    .map(|v| v.position[k])
                    .fold(f32::NEG_INFINITY, f32::max)
            });
            (lo, hi)
        }
        let mut wall = recipe("wall-corner");
        for (id, v) in [
            ("width", 2.),
            ("height", 3.2),
            ("depth", 1.5),
            ("thickness", 0.35),
        ] {
            value(&mut wall, id, v);
        }
        let mesh = wall.evaluate().unwrap().mesh;
        let (lo, x) = bounds(mesh.parts.iter().find(|p| p.id == "wall-x").unwrap());
        let (other_lo, z) = bounds(mesh.parts.iter().find(|p| p.id == "wall-z").unwrap());
        for k in 0..3 {
            assert!(lo[k].abs() < 0.00001);
            assert!(other_lo[k].abs() < 0.00001);
        }
        for (actual, expected) in [
            (x[0], 2.),
            (x[1], 3.2),
            (x[2], 0.35),
            (z[0], 0.35),
            (z[1], 3.2),
            (z[2], 1.5),
        ] {
            assert!((actual - expected).abs() < 0.00001);
        }
        let mut roof = recipe("roof-slope");
        value(&mut roof, "slope", 60.);
        value(&mut roof, "length", 2.);
        let mesh = roof.evaluate().unwrap().mesh;
        let (sin, cos) = 60f32.to_radians().sin_cos();
        for v in mesh.parts.iter().flat_map(|p| &p.vertices) {
            let local_y = -sin * v.position[0] + cos * v.position[1];
            let local_x = cos * v.position[0] + sin * v.position[1];
            assert!(
                local_y.abs() < 0.00001 || (local_y - 0.12).abs() < 0.00001,
                "The roof panel keeps its authored thickness"
            );
            assert!(local_x.abs() < 0.00001 || (local_x - 2.).abs() < 0.00001);
        }
        let mut posts = recipe("repeated-posts");
        value(&mut posts, "count", 7.);
        value(&mut posts, "spacing", 0.6);
        value(&mut posts, "height", 2.);
        let mesh = posts.evaluate().unwrap().mesh;
        assert_eq!(mesh.parts.len(), 7);
        for p in &mesh.parts {
            let (lo, hi) = bounds(p);
            assert!((hi[0] - lo[0] - 0.1).abs() < 0.00001);
            assert!(lo[1].abs() < 0.00001);
            assert!((hi[1] - 2.).abs() < 0.00001);
        }
        assert_eq!(Document::load(&posts.save().unwrap()).unwrap(), posts);
    }
    #[test]
    fn integer_repeat_controls_work_from_browser_through_visual_publisher() {
        let mut project = Project::default();
        let mut editor = GeometryEditor::default();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1100, 800, 1.);
        ui.init(&mut ctx);
        editor.handle_event(
            &TheEvent::StateChanged(TheId::named("MZ New Box"), TheWidgetState::Clicked),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        let posts = editor
            .recipe_browser
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .find(|e| e.document.id == "repeated-posts")
            .unwrap()
            .id
            .clone();
        editor.handle_event(
            &TheEvent::NewListItemSelected(posts, TheId::named(LIST)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.handle_event(
            &TheEvent::ValueChanged(
                TheId::named("MZ Recipe Control 0"),
                TheValue::IntRange(8, 1..=64),
            ),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.handle_event(
            &TheEvent::StateChanged(TheId::named(CREATE), TheWidgetState::Clicked),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        let id = editor.selected.unwrap();
        let doc = Document::load(&project.procedural_appearances[&id]).unwrap();
        assert_eq!(doc.evaluate().unwrap().mesh.parts.len(), 8);
        editor.handle_event(
            &TheEvent::IndexChanged(TheId::named("MZ Geometry Actions"), 1),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.handle_event(
            &TheEvent::ValueChanged(TheId::named("MZ Control Value"), TheValue::Int(6)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        editor.handle_event(
            &TheEvent::StateChanged(TheId::named("MZ Control Save"), TheWidgetState::Clicked),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        assert!(ui.dialog.is_none());
        let doc = Document::load(&project.procedural_appearances[&id]).unwrap();
        assert_eq!(doc.evaluate().unwrap().mesh.parts.len(), 6);
    }
    #[test]
    fn repeat_is_created_and_edited_through_the_normal_mz_source_commands() {
        let root = bundled().remove(0).document;
        let Operation::Group { document, .. } = &root.rows[0].tiles[0].op else {
            panic!()
        };
        let mut doc = document.as_ref().clone();
        presentation::edit(
            &mut doc,
            MaterialEditorCommand::AddSource {
                source: "repeat".into(),
                row: Some("solid".into()),
                after: Some("solid-2".into()),
            },
        )
        .unwrap();
        assert_eq!(doc.evaluate().unwrap().mesh.parts.len(), 3);
        let operation = doc.rows[1]
            .tiles
            .iter()
            .find(|t| matches!(t.op, Operation::Repeat { .. }))
            .unwrap()
            .id
            .clone();
        presentation::edit(
            &mut doc,
            MaterialEditorCommand::SetNumber {
                operation,
                parameter: "count".into(),
                value: 5.,
            },
        )
        .unwrap();
        assert_eq!(doc.evaluate().unwrap().mesh.parts.len(), 5);
    }
}
