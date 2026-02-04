use crate::prelude::*;

pub struct TheExternalCode {
    name: String,
    description: String,
    arg_names: Vec<String>,
    arg_values: Vec<TheValue>,
    returns: Option<TheValue>,
}

impl TheExternalCode {
    pub fn new(
        name: String,
        description: String,
        arg_names: Vec<String>,
        arg_values: Vec<TheValue>,
        returns: Option<TheValue>,
    ) -> Self {
        Self {
            name,
            description,
            arg_names,
            arg_values,
            returns,
        }
    }
}

pub struct TheCodeEditor {
    code_list_selection: Option<TheId>,
    grid_selection: Option<(u16, u16)>,

    codegrid_selection: Option<TheId>,
    bundle: TheCodeBundle,

    externals: Vec<TheExternalCode>,

    // The modules available for the code editor. The String is the name of the bundle and it's id it is contained in.
    modules: FxHashMap<Uuid, (String, Uuid, TheCodeModule)>,

    function_list_needs_update: bool,
    allow_modules: bool,

    curr_list_index: u32,

    undo: Option<TheUndo>,

    // Id set by the app to identify the type of the current code
    pub code_id: String,
}

impl Default for TheCodeEditor {
    fn default() -> Self {
        TheCodeEditor::new()
    }
}

impl TheCodeEditor {
    pub fn new() -> Self {
        Self {
            code_list_selection: None,
            grid_selection: None,
            codegrid_selection: None,

            bundle: TheCodeBundle::new(),

            externals: vec![],
            modules: FxHashMap::default(),

            curr_list_index: 0,

            function_list_needs_update: false,
            allow_modules: false,

            undo: None,
            code_id: str!(""),
        }
    }

    /// Add an external function to the code.
    pub fn clear_externals(&mut self) {
        self.externals.clear();
        self.function_list_needs_update = true;
    }

    /// Add an external function to the code.
    pub fn add_external(&mut self, external: TheExternalCode) {
        self.externals.push(external);
        self.function_list_needs_update = true;
    }

    /// Clears the module based code.
    pub fn clear_modules(&mut self) {
        self.modules.clear();
    }

    /// Add a moodule function to the code.
    pub fn insert_module(&mut self, bundle_name: String, bundle_id: Uuid, module: TheCodeModule) {
        self.modules
            .insert(module.codegrid_id, (bundle_name, bundle_id, module));
        self.function_list_needs_update = true;
    }

    /// Adds all modules of the given packages to the code.
    pub fn set_packages(&mut self, packages: FxHashMap<Uuid, TheCodePackage>) {
        self.clear_modules();
        for p in packages.values() {
            for m in p.modules.values() {
                self.insert_module(p.name.clone(), p.id, m.clone());
            }
        }
    }

    /// Sets if modules are allowed.
    pub fn set_allow_modules(&mut self, allow: bool) {
        self.allow_modules = allow;
        self.function_list_needs_update = true;
    }

    pub fn handle_event(&mut self, event: &TheEvent, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw = false;

        if self.function_list_needs_update {
            if let Some(code_list) = ui.get_list_layout("Code Editor Code List") {
                self.get_code_list_items(self.curr_list_index, code_list, ctx);
                redraw = true;
                self.function_list_needs_update = false;
            }
        }

        match event {
            /*
            TheEvent::CodeEditorApply(_id) => {
                let mut atom: Option<TheCodeAtom> = None;

                if let Some(code_list_selection) = &self.code_list_selection {
                    if let Some(widget) = ui.get_widget_id(code_list_selection.uuid) {
                        if let Some(name) = widget.value().to_string() {
                            atom = Some(self.create_atom(name.as_str()));
                        }
                    }
                }

                if let Some(atom) = atom {
                    self.set_selected_atom(ui, atom);
                    self.set_grid_selection_ui(ui, ctx);
                    redraw = true;
                }
            }*/
            TheEvent::SDFIndexChanged(_id, index) => {
                if let Some(code_list) = ui.get_list_layout("Code Editor Code List") {
                    self.curr_list_index = *index;
                    self.get_code_list_items(*index, code_list, ctx);
                }
            }
            TheEvent::DragStarted(id, text, offset) => {
                if id.name == "Code Editor Code List Item" {
                    if let Some(atom) = Some(self.create_atom(text.as_str(), id.uuid)) {
                        let mut drop = TheDrop::new(TheId::named("Code Editor Atom"));
                        drop.set_data(atom.to_json());
                        drop.set_title(text.clone());
                        drop.set_offset(*offset);
                        ui.style.create_drop_image(&mut drop, ctx);
                        ctx.ui.set_drop(drop);
                    }
                }
            }
            // TheEvent::CodeEditorDelete(_id) => {
            //     if let Some(selection) = self.grid_selection {
            //         if let Some(layout) = ui.get_code_layout("Code Editor") {
            //             if let Some(code_view) = layout.code_view_mut().as_code_view() {
            //                 code_view.codegrid_mut().code.remove_entry(&selection);
            //             }
            //         }
            //     }

            //     self.set_grid_selection_ui(ui, ctx);
            //     self.set_grid_status_message(ui, ctx);
            //     redraw = true;
            // }
            // TheEvent::CodeBundleChanged(_, _edit_state) => {
            // }
            TheEvent::CodeEditorChanged(_id, codegrid) => {
                self.bundle.insert_grid(codegrid.clone());
                ctx.ui
                    .send(TheEvent::CodeBundleChanged(self.bundle.clone(), true));
                ctx.ui.relayout = true;
            }
            TheEvent::CodeEditorSelectionChanged(_id, selection) => {
                self.grid_selection = *selection;
                self.set_grid_selection_ui(ui, ctx);

                if let Some(selection) = selection {
                    if selection.0 % 2 == 0 {
                        ctx.ui.set_enabled("Code Keywords Menu");
                        ctx.ui.set_disabled("Code Operators Menu");
                        ctx.ui.set_enabled("Code Values Menu");
                        ctx.ui.set_enabled("Code Functions Menu");
                        ctx.ui.set_enabled("Code Modules Menu");
                    } else {
                        ctx.ui.set_disabled("Code Keywords Menu");
                        ctx.ui.set_enabled("Code Operators Menu");
                        ctx.ui.set_disabled("Code Values Menu");
                        ctx.ui.set_disabled("Code Functions Menu");
                        ctx.ui.set_disabled("Code Modules Menu");
                    }
                }
                redraw = true;
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "Code Editor Code List Item" {
                    self.code_list_selection = Some(id.clone());
                } else if id.name == "CodeGrid List Add" {
                    if *state == TheWidgetState::Clicked {
                        let codegrid = TheCodeGrid::new();
                        self.bundle.insert_grid(codegrid.clone());

                        if let Some(code_list) = ui.get_list_layout("CodeGrid List") {
                            let item_id = TheId::named_with_id("CodeGrid List Item", codegrid.id);
                            let mut item = TheListItem::new(item_id.clone());
                            item.set_text(codegrid.name.clone());
                            item.set_associated_layout(code_list.id().clone());
                            item.set_state(TheWidgetState::Selected);
                            code_list.deselect_all();
                            code_list.add_item(item, ctx);

                            ctx.ui
                                .send_widget_state_changed(&item_id, TheWidgetState::Selected);
                        }

                        ui.set_widget_disabled_state("CodeGrid List Name", ctx, false);
                        ui.set_widget_disabled_state("CodeGrid List Remove", ctx, false);

                        ctx.ui
                            .send(TheEvent::CodeBundleChanged(self.bundle.clone(), true));

                        self.set_codegrid(codegrid.clone(), ui);
                        self.set_grid_selection_ui(ui, ctx);
                    }
                } else if id.name == "CodeGrid List Remove" {
                    if *state == TheWidgetState::Clicked {
                        if let Some(codegrid_selection) = &self.codegrid_selection {
                            self.bundle.grids.remove(&codegrid_selection.uuid);
                            let mut disable = false;
                            if let Some(code_list) = ui.get_list_layout("CodeGrid List") {
                                code_list.remove(codegrid_selection.clone());
                                code_list.select_first_item(ctx);

                                disable = code_list.widgets().is_empty();
                            }

                            ui.set_widget_disabled_state("CodeGrid List Name", ctx, disable);
                            ui.set_widget_disabled_state("CodeGrid List Remove", ctx, disable);

                            self.codegrid_selection = None;

                            ctx.ui
                                .send(TheEvent::CodeBundleChanged(self.bundle.clone(), true));

                            self.set_grid_selection_ui(ui, ctx);
                        }
                    }
                } else if id.name == "CodeGrid List Item" && *state == TheWidgetState::Selected {
                    if let Some(codegrid) = self.bundle.get_grid(&id.uuid) {
                        self.codegrid_selection = Some(id.clone());
                        if let Some(text_edit) = ui.get_text_line_edit("CodeGrid List Name") {
                            text_edit.set_text(codegrid.name.clone());
                        }

                        ui.set_widget_disabled_state("CodeGrid List Name", ctx, false);
                        ui.set_widget_disabled_state("CodeGrid List Remove", ctx, false);

                        self.set_codegrid(codegrid.clone(), ui);
                        self.set_grid_selection_ui(ui, ctx);

                        // We send the update to remember the current selection

                        self.bundle.selected_grid_id = Some(id.uuid);
                        ctx.ui
                            .send(TheEvent::CodeBundleChanged(self.bundle.clone(), false));
                    }
                }

                redraw = true;
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Atom Comparison" {
                    if let Some(op) = TheValueComparison::from_index(*index as u8) {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::Comparison(op));
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom Assignment" {
                    if let Some(op) = TheValueAssignment::from_index(*index as u8) {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::Assignment(op));
                        self.finish_undo(ui, ctx);
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "CodeGrid List Name" {
                    if let Some(text) = value.to_string() {
                        if let Some(codegrid_selection) = &self.codegrid_selection {
                            let mut cg_for_rename_clone: Option<TheCodeGrid> = None;
                            if let Some(codegrid) =
                                self.bundle.get_grid_mut(&codegrid_selection.uuid)
                            {
                                if codegrid.name != text {
                                    if let Some(widget) = ui.get_widget_id(codegrid_selection.uuid)
                                    {
                                        widget.set_value(TheValue::Text(text.clone()));
                                        codegrid.name.clone_from(&text);

                                        cg_for_rename_clone = Some(codegrid.clone());

                                        ctx.ui.send(TheEvent::CodeBundleChanged(
                                            self.bundle.clone(),
                                            true,
                                        ));
                                        ctx.ui.relayout = true;
                                    }
                                }
                            }
                            // We have to change the bundle in the view after rename
                            // Otherwise the old name will be used after a change in the view.
                            if let Some(cg_for_rename_clone) = cg_for_rename_clone {
                                self.set_codegrid(cg_for_rename_clone, ui);
                            }
                        }
                    }
                } else if id.name == "Code Zoom" {
                    if let Some(v) = value.to_f32() {
                        if let Some(layout) = ui.get_code_layout("Code Editor") {
                            if let Some(code_view) = layout.code_view_mut().as_code_view() {
                                code_view.set_zoom(v);
                                ctx.ui.relayout = true;
                            }
                        }
                    }
                } else if id.name == "Atom Comparison" {
                    if let TheValue::Int(v) = value {
                        self.set_selected_atom(
                            ui,
                            TheCodeAtom::Comparison(
                                TheValueComparison::from_index(*v as u8).unwrap(),
                            ),
                        );
                    }
                } else if id.name == "Atom Assignment" {
                    if let TheValue::Int(v) = value {
                        self.set_selected_atom(
                            ui,
                            TheCodeAtom::Assignment(
                                TheValueAssignment::from_index(*v as u8).unwrap(),
                            ),
                        );
                    }
                } else if id.name == "Atom TextList" {
                    if let Some(index) = value.to_i32() {
                        self.start_undo(ui);
                        if let Some(TheCodeAtom::Value(TheValue::TextList(_, list))) =
                            self.get_selected_atom(ui)
                        {
                            self.set_selected_atom(
                                ui,
                                TheCodeAtom::Value(TheValue::TextList(index, list.clone())),
                            );
                            self.finish_undo(ui, ctx);
                        }
                    }
                } else if id.name == "Atom Argument" {
                    if let Some(name) = value.to_string() {
                        if !name.is_empty() {
                            self.start_undo(ui);
                            self.set_selected_atom(ui, TheCodeAtom::Argument(name));
                            self.finish_undo(ui, ctx);
                        }
                    }
                } else if id.name == "Atom Local Get" {
                    if let Some(name) = value.to_string() {
                        if !name.is_empty() {
                            self.start_undo(ui);
                            self.set_selected_atom(ui, TheCodeAtom::LocalGet(name));
                            self.finish_undo(ui, ctx);
                        }
                    }
                } else if id.name == "Atom Local Set" {
                    if let Some(name) = value.to_string() {
                        if !name.is_empty() {
                            self.start_undo(ui);
                            self.set_selected_atom(
                                ui,
                                TheCodeAtom::LocalSet(name, TheValueAssignment::Assign),
                            );
                            self.finish_undo(ui, ctx);
                        }
                    }
                } else if id.name == "Atom Get" {
                    if let Some(name) = value.to_string() {
                        if !name.is_empty() {
                            self.start_undo(ui);
                            self.set_selected_atom(ui, TheCodeAtom::Get(name));
                            self.finish_undo(ui, ctx);
                        }
                    }
                } else if id.name == "Atom Set" {
                    if let Some(name) = value.to_string() {
                        if !name.is_empty() {
                            self.start_undo(ui);
                            self.set_selected_atom(
                                ui,
                                TheCodeAtom::Set(name, TheValueAssignment::Assign),
                            );
                            self.finish_undo(ui, ctx);
                        }
                    }
                } else if id.name == "Atom Object Get Object" {
                    if let Some(name) = value.to_string() {
                        if !name.is_empty() {
                            if let Some(TheCodeAtom::ObjectGet(_object, variable)) =
                                self.get_selected_atom(ui)
                            {
                                self.start_undo(ui);
                                self.set_selected_atom(ui, TheCodeAtom::ObjectGet(name, variable));
                                self.finish_undo(ui, ctx);
                            }
                        }
                    }
                } else if id.name == "Atom Object Get Variable" {
                    if let Some(name) = value.to_string() {
                        if !name.is_empty() {
                            if let Some(TheCodeAtom::ObjectGet(object, _variable)) =
                                self.get_selected_atom(ui)
                            {
                                self.start_undo(ui);
                                self.set_selected_atom(ui, TheCodeAtom::ObjectGet(object, name));
                                self.finish_undo(ui, ctx);
                            }
                        }
                    }
                } else if id.name == "Atom Object Set Object" {
                    if let Some(name) = value.to_string() {
                        if !name.is_empty() {
                            if let Some(TheCodeAtom::ObjectSet(_, variable, _)) =
                                self.get_selected_atom(ui)
                            {
                                self.start_undo(ui);
                                self.set_selected_atom(
                                    ui,
                                    TheCodeAtom::ObjectSet(
                                        name,
                                        variable,
                                        TheValueAssignment::Assign,
                                    ),
                                );
                                self.finish_undo(ui, ctx);
                            }
                        }
                    }
                } else if id.name == "Atom Object Set Variable" {
                    if let Some(name) = value.to_string() {
                        if !name.is_empty() {
                            if let Some(TheCodeAtom::ObjectSet(object, _, _)) =
                                self.get_selected_atom(ui)
                            {
                                self.start_undo(ui);
                                self.set_selected_atom(
                                    ui,
                                    TheCodeAtom::ObjectSet(
                                        object,
                                        name,
                                        TheValueAssignment::Assign,
                                    ),
                                );
                                self.finish_undo(ui, ctx);
                            }
                        }
                    }
                } else if id.name == "Atom Color Hex" {
                    if let Some(hex_color) = value.to_string() {
                        if !hex_color.is_empty() {
                            if let Some(TheCodeAtom::Value(TheValue::ColorObject(_))) =
                                self.get_selected_atom(ui)
                            {
                                self.start_undo(ui);
                                self.set_selected_atom(
                                    ui,
                                    TheCodeAtom::Value(TheValue::ColorObject(TheColor::from_hex(
                                        hex_color.as_str(),
                                    ))),
                                );
                                self.finish_undo(ui, ctx);
                            }
                        }
                    }
                } else if id.name == "Atom Direction Float2" {
                    if let Some(value) = value.to_vec2f() {
                        if let Some(TheCodeAtom::Value(TheValue::Direction(_))) =
                            self.get_selected_atom(ui)
                        {
                            self.start_undo(ui);
                            self.set_selected_atom(
                                ui,
                                TheCodeAtom::Value(TheValue::Direction(Vec3::new(
                                    value.x, 0.0, value.y,
                                ))),
                            );
                            self.finish_undo(ui, ctx);
                        }
                    }
                } else if id.name == "Atom Integer" {
                    if let Some(v) = value.to_i32() {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::Value(TheValue::Int(v)));
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom Float" {
                    if let Some(v) = value.to_f32() {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::Value(TheValue::Float(v)));
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom Tile" {
                    if let Some(name) = value.to_string() {
                        self.start_undo(ui);
                        self.set_selected_atom(
                            ui,
                            TheCodeAtom::Value(TheValue::Tile(name, Uuid::nil())),
                        );
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom Text" {
                    if let Some(name) = value.to_string() {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::Value(TheValue::Text(name)));
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom Position" {
                    if let Some(v) = value.to_vec2f() {
                        self.start_undo(ui);
                        self.set_selected_atom(
                            ui,
                            TheCodeAtom::Value(TheValue::Position(Vec3::new(v.x, 0.0, v.y))),
                        );
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom Bool" {
                    if let Some(v) = value.as_f32() {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::Value(TheValue::Bool(v > 0.0)));
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom Int2" {
                    if let Some(v) = value.to_vec2i() {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::Value(TheValue::Int2(v)));
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom Float2" {
                    if let Some(v) = value.to_vec2f() {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::Value(TheValue::Float2(v)));
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom RandInt" {
                    if let Some(v) = value.to_vec2i() {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::RandInt(v));
                        self.finish_undo(ui, ctx);
                    }
                } else if id.name == "Atom RandFloat" {
                    if let Some(v) = value.to_vec2f() {
                        self.start_undo(ui);
                        self.set_selected_atom(ui, TheCodeAtom::RandFloat(v));
                        self.finish_undo(ui, ctx);
                    }
                }

                redraw = true;
            }
            _ => {}
        }

        redraw
    }

    /// Gets the codegrid from the editor
    pub fn get_codegrid(&mut self, ui: &mut TheUI) -> TheCodeGrid {
        if let Some(layout) = ui.get_code_layout("Code Editor") {
            if let Some(code_view) = layout.code_view_mut().as_code_view() {
                return code_view.codegrid().clone();
            }
        }
        TheCodeGrid::new()
    }

    /// Gets the codegrid from the editor
    pub fn get_codegrid_id(&mut self, ui: &mut TheUI) -> Uuid {
        if let Some(layout) = ui.get_code_layout("Code Editor") {
            if let Some(code_view) = layout.code_view_mut().as_code_view() {
                return code_view.codegrid().id;
            }
        }
        Uuid::nil()
    }

    /// Sets the codegrid to the editor
    pub fn set_codegrid(&mut self, codegrid: TheCodeGrid, ui: &mut TheUI) {
        if let Some(layout) = ui.get_code_layout("Code Editor") {
            if let Some(code_view) = layout.code_view_mut().as_code_view() {
                code_view.set_codegrid(codegrid);
            }
        }
    }

    /// Sets the debug module to the editor.
    pub fn set_debug_module(&mut self, debug_module: TheDebugModule, ui: &mut TheUI) {
        if let Some(layout) = ui.get_code_layout("Code Editor") {
            if let Some(code_view) = layout.code_view_mut().as_code_view() {
                code_view.set_debug_module(debug_module);
            }
        }
    }

    /// Clears the debug module of the editor.
    pub fn clear_debug_module(&mut self, ui: &mut TheUI) {
        if let Some(layout) = ui.get_code_layout("Code Editor") {
            if let Some(code_view) = layout.code_view_mut().as_code_view() {
                code_view.set_debug_module(TheDebugModule::default());
            }
        }
    }

    /// Sets the UI of the currently selected atom into the top toolbar.
    pub fn set_grid_selection_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(atom) = self.get_selected_atom(ui) {
            if let Some(layout) = ui.get_hlayout("Code Top Toolbar") {
                layout.clear();
                atom.to_layout(layout);
                layout.relayout(ctx);
                ctx.ui.redraw_all = true;
            }
        } else if let Some(layout) = ui.get_hlayout("Code Top Toolbar") {
            layout.clear();
            ctx.ui.redraw_all = true;
        }
    }

    /// Returns a clone of the currently selected atom (if any).
    pub fn get_selected_atom(&mut self, ui: &mut TheUI) -> Option<TheCodeAtom> {
        if let Some(grid_selection) = self.grid_selection {
            if let Some(layout) = ui.get_code_layout("Code Editor") {
                if let Some(code_view) = layout.code_view_mut().as_code_view() {
                    let grid = code_view.codegrid();

                    if let Some(atom) = grid.code.get(&grid_selection) {
                        return Some(atom.clone());
                    }
                }
            }
        }
        None
    }

    /// Set the atom at the current position.
    pub fn set_selected_atom(&mut self, ui: &mut TheUI, atom: TheCodeAtom) {
        if let Some(grid_selection) = self.grid_selection {
            if let Some(layout) = ui.get_code_layout("Code Editor") {
                if let Some(code_view) = layout.code_view_mut().as_code_view() {
                    code_view.set_grid_atom(grid_selection, atom);
                }
            }
        }
    }

    /// Start undo by setting the undo data.
    pub fn start_undo(&mut self, ui: &mut TheUI) {
        let mut undo = TheUndo::new(TheId::named("Code Editor"));
        undo.set_undo_data(self.get_codegrid_json(ui));
        self.undo = Some(undo);
    }

    /// Finish undo by adding the redo data and add to undo stack.
    pub fn finish_undo(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if self.undo.is_none() {
            return;
        }

        let mut undo = self.undo.take().unwrap();
        undo.set_redo_data(self.get_codegrid_json(ui));

        let grid = self.get_codegrid(ui).clone();
        self.bundle.insert_grid(grid);

        ctx.ui.undo_stack.add(undo);

        ctx.ui
            .send(TheEvent::CodeBundleChanged(self.bundle.clone(), true));
    }

    /// Get the codegrid as json
    pub fn get_codegrid_json(&mut self, ui: &mut TheUI) -> String {
        if let Some(layout) = ui.get_code_layout("Code Editor") {
            if let Some(code_view) = layout.code_view_mut().as_code_view() {
                return code_view.codegrid().to_json();
            }
        }
        "".to_string()
    }

    /// Set the codegrid from json
    pub fn set_codegrid_json(&mut self, json: String, ui: &mut TheUI) {
        if let Some(layout) = ui.get_code_layout("Code Editor") {
            if let Some(code_view) = layout.code_view_mut().as_code_view() {
                code_view.set_codegrid(TheCodeGrid::from_json(json.as_str()));
            }
        }
    }

    /// Create an atom for the given name.
    pub fn create_atom(&self, name: &str, id: Uuid) -> TheCodeAtom {
        match name {
            "Assignment" => TheCodeAtom::Assignment(TheValueAssignment::Assign),
            "Comparison" => TheCodeAtom::Comparison(TheValueComparison::Equal),
            "Argument" => TheCodeAtom::Argument("var".to_string()),
            "Return" => TheCodeAtom::Return,
            "Local Get" => TheCodeAtom::LocalGet("var".to_string()),
            "Local Set" => TheCodeAtom::LocalSet("var".to_string(), TheValueAssignment::Assign),
            "Object Get" => TheCodeAtom::ObjectGet("self".to_string(), "name".to_string()),
            "Object Set" => TheCodeAtom::ObjectSet(
                "self".to_string(),
                "name".to_string(),
                TheValueAssignment::Assign,
            ),
            "Get" => TheCodeAtom::Get("".to_string()),
            "Set" => TheCodeAtom::Set("".to_string(), TheValueAssignment::Assign),
            "Empty" => TheCodeAtom::Value(TheValue::Empty),
            "Integer" => TheCodeAtom::Value(TheValue::Int(0)),
            "Float" => TheCodeAtom::Value(TheValue::Float(0.0)),
            "Bool" => TheCodeAtom::Value(TheValue::Bool(false)),
            "Text" => TheCodeAtom::Value(TheValue::Text("".to_string())),
            "Object" => TheCodeAtom::Value(TheValue::CodeObject(TheCodeObject::default())),
            "List" => TheCodeAtom::Value(TheValue::List(vec![])),
            "Tile" => TheCodeAtom::Value(TheValue::Tile("name".into(), Uuid::nil())),
            "Int2" => TheCodeAtom::Value(TheValue::Int2(Vec2::new(0, 0))),
            "Float2" => TheCodeAtom::Value(TheValue::Float2(Vec2::new(0.0, 0.0))),
            "Float3" => TheCodeAtom::Value(TheValue::Float3(Vec3::new(0.0, 0.0, 0.0))),
            "Position" => TheCodeAtom::Value(TheValue::Position(Vec3::new(0.0, 0.0, 0.0))),
            "Add" => TheCodeAtom::Add,
            "Subtract" => TheCodeAtom::Subtract,
            "Multiply" => TheCodeAtom::Multiply,
            "Divide" => TheCodeAtom::Divide,
            "Modulus" => TheCodeAtom::Modulus,
            "RInt" => TheCodeAtom::RandInt(Vec2::new(0, 3)),
            "RFloat" => TheCodeAtom::RandFloat(Vec2::new(0.0, 1.0)),
            "Color" => TheCodeAtom::Value(TheValue::ColorObject(TheColor::default())),
            "Direction" => TheCodeAtom::Value(TheValue::Direction(Vec3::new(0.0, 0.0, -1.0))),
            _ => {
                if let Some((bundle_name, bundle_id, module)) = self.modules.get(&id) {
                    return TheCodeAtom::ModuleCall(
                        bundle_name.clone(),
                        *bundle_id,
                        module.name.clone(),
                        module.codegrid_id,
                    );
                }

                for e in &self.externals {
                    if e.name == name {
                        return TheCodeAtom::ExternalCall(
                            e.name.clone(),
                            e.description.clone(),
                            e.arg_names.clone(),
                            e.arg_values.clone(),
                            e.returns.clone(),
                        );
                    }
                }

                TheCodeAtom::EndOfCode
            }
        }
    }

    /// Builds the UI canvas
    pub fn build_canvas(&self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas: TheCanvas = TheCanvas::new();

        // Left code list
        /*
        let mut list_canvas: TheCanvas = TheCanvas::new();

        let mut code_layout = TheListLayout::new(TheId::named("Code Editor Code List"));
        code_layout.limiter_mut().set_max_width(150);
        self.get_code_list_items(0, &mut code_layout, ctx);

        code_layout.select_first_item(ctx);
        list_canvas.set_layout(code_layout);

        // ---

        let mut list_toolbar_canvas = TheCanvas::new();

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_margin(vec4i(2, 2, 2, 2));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_mode(TheHLayoutMode::SizeBased);

        let mut sdf_view = TheSDFView::new(TheId::named("Code List SDF View"));

        let mut sdf_canvas = TheSDFCanvas::new();
        sdf_canvas.background = crate::thecolor::TheColor::from_u8_array([118, 118, 118, 255]);
        sdf_canvas.selected = Some(0);
        sdf_canvas.add(
            TheSDF::Circle(TheDim::new(5, 2, 20, 20)),
            ThePattern::Solid(crate::thecolor::TheColor::from_u8(74, 74, 74, 255)),
        );
        sdf_view.set_status(0, "Show all keywords.".to_string());

        sdf_canvas.add(
            TheSDF::Hexagon(TheDim::new(40, 2, 20, 20)),
            ThePattern::Solid(crate::thecolor::TheColor::from_u8(74, 74, 74, 255)),
        );
        sdf_view.set_status(1, "Show all value types.".to_string());

        sdf_canvas.add(
            TheSDF::Rhombus(TheDim::new(75, 2, 20, 20)),
            ThePattern::Solid(crate::thecolor::TheColor::from_u8(74, 74, 74, 255)),
        );
        sdf_view.set_status(2, "Show all operators.".to_string());

        sdf_canvas.add(
            TheSDF::RoundedRect(TheDim::new(110, 2, 20, 20), (5.0, 5.0, 5.0, 5.0)),
            ThePattern::Solid(crate::thecolor::TheColor::from_u8(74, 74, 74, 255)),
        );
        sdf_view.set_status(3, "Show all available functions.".to_string());

        sdf_view.set_canvas(sdf_canvas);

        toolbar_hlayout.add_widget(Box::new(sdf_view));
        list_toolbar_canvas.set_layout(toolbar_hlayout);
        list_toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        list_canvas.set_top(list_toolbar_canvas);
        canvas.set_left(list_canvas);

        */
        // Top Toolbar
        let mut top_toolbar_canvas = TheCanvas::new();
        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Code Top Toolbar"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.set_padding(10);
        top_toolbar_canvas.set_layout(toolbar_hlayout);
        top_toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));

        // Bottom Toolbar
        let mut bottom_toolbar_canvas = TheCanvas::new();

        let mut compile_button = TheTraybarButton::new(TheId::named("Compile"));
        compile_button.set_text("Compile".to_string());

        let mut text = TheText::new(TheId::empty());
        text.set_text("Zoom".to_string());

        let mut zoom = TheSlider::new(TheId::named("Code Zoom"));
        zoom.set_value(TheValue::Float(1.0));
        zoom.set_range(TheValue::RangeF32(0.5..=3.0));
        zoom.set_continuous(true);
        zoom.limiter_mut().set_max_width(120);

        let mut status_text = TheText::new(TheId::named("Code Grid Status"));
        status_text.set_text("".to_string());

        let divider1 = TheHDivider::new(TheId::empty());
        let divider2 = TheHDivider::new(TheId::empty());

        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Code Bottom Toolbar Layout"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(compile_button));
        toolbar_hlayout.add_widget(Box::new(divider1));
        toolbar_hlayout.add_widget(Box::new(text));
        toolbar_hlayout.add_widget(Box::new(zoom));
        toolbar_hlayout.add_widget(Box::new(divider2));
        toolbar_hlayout.add_widget(Box::new(status_text));
        toolbar_hlayout.limiter_mut().set_max_height(27);

        bottom_toolbar_canvas.set_layout(toolbar_hlayout);
        bottom_toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));

        // ---

        let code_layout = TheCodeLayout::new(TheId::named("Code Editor"));

        canvas.set_layout(code_layout);
        canvas.set_top(top_toolbar_canvas);
        canvas.set_bottom(bottom_toolbar_canvas);
        canvas.top_is_expanding = false;

        canvas
    }

    /// Sets the bundle and returns the list canvas for it.
    pub fn set_bundle(
        &mut self,
        bundle: TheCodeBundle,
        ctx: &mut TheContext,
        width: i32,
        height: Option<i32>,
    ) -> TheCanvas {
        ctx.ui.relayout = true;
        self.bundle = bundle;

        let mut canvas: TheCanvas = TheCanvas::new();

        let mut settings_header = TheCanvas::new();
        let mut switchbar = TheSwitchbar::new(TheId::empty());
        switchbar.set_text("Functions".to_string());
        settings_header.set_widget(switchbar);

        canvas.set_top(settings_header);

        // Grid list

        let mut list_canvas: TheCanvas = TheCanvas::new();
        list_canvas.limiter_mut().set_max_width(width);
        if let Some(height) = height {
            list_canvas.limiter_mut().set_max_height(height);
        }

        let mut code_layout = TheListLayout::new(TheId::named("CodeGrid List"));

        let keys = self.bundle.sorted();

        for key in &keys {
            if let Some(grid) = self.bundle.grids.get(key) {
                let mut item =
                    TheListItem::new(TheId::named_with_id("CodeGrid List Item", grid.id));
                item.set_text(grid.name.clone());
                item.set_associated_layout(code_layout.id().clone());
                code_layout.add_item(item, ctx);
            }
        }

        if let Some(id) = self.bundle.selected_grid_id {
            if !code_layout.select_item(id, ctx, true) {
                code_layout.select_first_item(ctx);
            }
        } else {
            code_layout.select_first_item(ctx);
        }

        list_canvas.set_layout(code_layout);

        canvas.set_center(list_canvas);

        // Toolbar

        let mut add_button = TheTraybarButton::new(TheId::named("CodeGrid List Add"));
        add_button.set_icon_name("icon_role_add".to_string());
        add_button.set_status_text("Add new code.");
        let mut remove_button = TheTraybarButton::new(TheId::named("CodeGrid List Remove"));
        remove_button.set_icon_name("icon_role_remove".to_string());
        remove_button.set_disabled(true);
        remove_button.set_status_text("Remove code.");

        let mut text_edit = TheTextLineEdit::new(TheId::named("CodeGrid List Name"));
        text_edit.limiter_mut().set_max_width(180);

        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(5, 2, 5, 2));
        toolbar_hlayout.add_widget(Box::new(add_button));
        toolbar_hlayout.add_widget(Box::new(remove_button));
        toolbar_hlayout.add_widget(Box::new(TheHDivider::new(TheId::empty())));
        toolbar_hlayout.add_widget(Box::new(text_edit));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_bottom(toolbar_canvas);

        canvas
    }

    /// Returns the keywords context menu.
    pub fn create_keywords_context_menu_item(&self) -> TheContextMenuItem {
        let mut menu_item =
            TheContextMenuItem::new(str!("Keywords"), TheId::named("Code Keywords Menu"));

        let mut menu = TheContextMenu::named(str!("Keywords"));
        menu.id = TheId::named("Code Keywords Menu");
        menu.add(TheContextMenuItem::new(
            str!("Argument"),
            TheId::named("Code Keyword Argument"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Return"),
            TheId::named("Code Keyword Return"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Get"),
            TheId::named("Code Keyword Get"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Set"),
            TheId::named("Code Keyword Set"),
        ));

        menu_item.set_sub_menu(menu);
        menu_item
    }

    /// Returns the operators context menu.
    pub fn create_operators_context_menu_item(&self) -> TheContextMenuItem {
        let mut menu_item =
            TheContextMenuItem::new(str!("Operators"), TheId::named("Code Operators Menu"));

        let mut menu = TheContextMenu::named(str!("Operators"));
        menu.id = TheId::named("Code Operators Menu");
        menu.add(TheContextMenuItem::new(
            str!("Assignment"),
            TheId::named("Code Operator Assignment"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Comparison"),
            TheId::named("Code Operator Comparison"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Add"),
            TheId::named("Code Operator Add"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Subtract"),
            TheId::named("Code Operator Subtract"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Multiply"),
            TheId::named("Code Operator Multiply"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Divide"),
            TheId::named("Code Operator Divide"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Modulus"),
            TheId::named("Code Operator Modulus"),
        ));
        menu_item.set_sub_menu(menu);
        menu_item
    }

    /// Returns the values context menu.
    pub fn create_values_context_menu_item(&self) -> TheContextMenuItem {
        let mut menu_item =
            TheContextMenuItem::new(str!("Values"), TheId::named("Code Values Menu"));

        let mut menu = TheContextMenu::named(str!("Values"));
        menu.id = TheId::named("Code Values Menu");
        menu.add(TheContextMenuItem::new(
            str!("Empty"),
            TheId::named("Code Value Empty"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Integer"),
            TheId::named("Code Value Integer"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Float"),
            TheId::named("Code Value Float"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Bool"),
            TheId::named("Code Value Bool"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Text"),
            TheId::named("Code Value Text"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Object"),
            TheId::named("Code Value Object"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("List"),
            TheId::named("Code Value List"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Tile"),
            TheId::named("Code Value Tile"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Position"),
            TheId::named("Code Value Position"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Direction"),
            TheId::named("Code Value Direction"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Random Int"),
            TheId::named("Code Value RInt"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Random Float"),
            TheId::named("Code Value RFloat"),
        ));
        menu.add(TheContextMenuItem::new(
            str!("Color"),
            TheId::named("Code Value Color"),
        ));

        menu_item.set_sub_menu(menu);
        menu_item
    }

    /// Returns the functions context menu.
    pub fn create_functions_context_menu_item(&self) -> TheContextMenuItem {
        let mut menu_item =
            TheContextMenuItem::new(str!("Functions"), TheId::named("Code Functions Menu"));

        let mut menu = TheContextMenu::named(str!("Functions"));
        menu.id = TheId::named("Code Functions Menu");

        for e in &self.externals {
            menu.add(TheContextMenuItem::new(
                e.name.clone(),
                TheId::named(&format!("Code Functions {}", e.name.clone())),
            ));
        }

        menu_item.set_sub_menu(menu);
        menu_item
    }

    /// Returns the modules context menu.
    pub fn create_modules_context_menu_item(&self) -> TheContextMenuItem {
        let mut menu_item =
            TheContextMenuItem::new(str!("Modules"), TheId::named("Code Modules Menu"));

        let mut menu = TheContextMenu::named(str!("Modules"));
        menu.id = TheId::named("Code Functions Menu");

        for (bundle_name, _bundle_id, module) in self.modules.values() {
            menu.add(TheContextMenuItem::new(
                module.name.clone(),
                TheId::named(&format!("{}: {}", bundle_name, module.name.clone()).to_string()),
            ));
            /*
            let mut item = TheListItem::new(TheId::named_with_id(
                "Code Editor Code List Item",
                module.codegrid_id,
            ));
            item.set_text(module.name.clone());
            item.set_status_text(
                format!("{}: {}", bundle_name, module.name.clone())
                    .to_string()
                    .as_str(),
            );
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);*/
        }

        menu_item.set_sub_menu(menu);
        menu_item
    }

    /// Set the default state of the menu selection.
    pub fn init_menu_selection(&mut self, ctx: &mut TheContext) {
        ctx.ui.set_disabled("Code Keywords Menu");
        ctx.ui.set_disabled("Code Values Menu");
        ctx.ui.set_disabled("Code Operators Menu");
        ctx.ui.set_disabled("Code Functions Menu");
        ctx.ui.set_disabled("Code Modules Menu");
    }

    /// Insert a selected context menu item.
    pub fn insert_context_menu_id(&mut self, id: TheId, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(last) = id.name.split(' ').last() {
            let atom = self.create_atom(last, id.uuid);

            if atom != TheCodeAtom::EndOfCode {
                self.start_undo(ui);

                if let TheCodeAtom::ExternalCall(_, _, _, arg_values, _) = &atom {
                    if let Some((x, y)) = self.grid_selection {
                        for (index, value) in arg_values.iter().enumerate() {
                            let off = x + (index + 1) as u16 * 2;

                            if let Some(layout) = ui.get_code_layout("Code Editor") {
                                if let Some(code_view) = layout.code_view_mut().as_code_view() {
                                    let codegrid = code_view.codegrid_mut();
                                    codegrid
                                        .code
                                        .entry((off, y))
                                        .or_insert_with(|| TheCodeAtom::Value(value.clone()));
                                }
                            }
                        }
                    }
                }

                self.set_selected_atom(ui, atom);
                self.finish_undo(ui, ctx);
                self.set_grid_selection_ui(ui, ctx);
            }
        }
    }

    pub fn get_code_list_items(
        &self,
        index: u32,
        code_layout: &mut dyn TheListLayoutTrait,
        ctx: &mut TheContext,
    ) {
        code_layout.clear();
        if index == 0 {
            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Argument".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Return".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Get".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Set".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);
        }

        if index == 1 {
            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Empty".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Integer".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Float".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Bool".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Text".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Object".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("List".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Tile".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Position".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("RInt".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("RFloat".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Color".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item: TheListItem =
                TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Direction".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            // let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            // item.set_text("Float2".to_string());
            // item.set_associated_layout(code_layout.id().clone());
            // code_layout.add_item(item, ctx);

            // let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            // item.set_text("Float3".to_string());
            // item.set_associated_layout(code_layout.id().clone());
            // code_layout.add_item(item, ctx);
        }

        if index == 2 {
            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Assignment".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Comparison".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Add".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Subtract".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Multiply".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Divide".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);

            let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
            item.set_text("Modulus".to_string());
            item.set_associated_layout(code_layout.id().clone());
            code_layout.add_item(item, ctx);
        }

        if index == 3 {
            for e in &self.externals {
                let mut item = TheListItem::new(TheId::named("Code Editor Code List Item"));
                item.set_text(e.name.clone());
                item.set_status_text(e.description.clone().as_str());
                item.set_associated_layout(code_layout.id().clone());
                code_layout.add_item(item, ctx);
            }

            if self.allow_modules {
                for (bundle_name, _bundle_id, module) in self.modules.values() {
                    let mut item = TheListItem::new(TheId::named_with_id(
                        "Code Editor Code List Item",
                        module.codegrid_id,
                    ));
                    item.set_text(module.name.clone());
                    item.set_status_text(
                        format!("{}: {}", bundle_name, module.name.clone())
                            .to_string()
                            .as_str(),
                    );
                    item.set_associated_layout(code_layout.id().clone());
                    code_layout.add_item(item, ctx);
                }
            }
        }
    }

    /// Returns the bundle
    pub fn get_bundle(&self) -> TheCodeBundle {
        self.bundle.clone()
    }
}
