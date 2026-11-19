use super::*;
use shared::rulesets::{
    document::RulesState,
    equipment_family::{BoneArmorFamily, FamilyPlan},
};

pub(super) struct FamilyWizard {
    spec: BoneArmorFamily,
    preview: Option<FamilyPlan>,
    message: String,
}

/// Undo a workbench draft edit without changing project content.
pub(super) struct RulesUndo(RulesState);
impl From<RulesState> for RulesUndo {
    fn from(rules: RulesState) -> Self {
        Self(rules)
    }
}
impl RulesUndo {
    pub fn restore(self, project: &mut Project) -> Self {
        Self(std::mem::replace(&mut project.rules_document, self.0))
    }
}

fn note(form: &mut dyn TheTextLayoutTrait, label: &str, text: impl Into<String>) {
    let text = text.into();
    let mut line = String::new();
    let mut first = true;
    for word in text.split_whitespace() {
        if line.len() + word.len() > 66 && !line.is_empty() {
            let mut widget = TheText::new(TheId::empty());
            widget.set_text(std::mem::take(&mut line));
            form.add_pair(
                if first { label.into() } else { String::new() },
                Box::new(widget),
            );
            first = false;
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        let mut widget = TheText::new(TheId::empty());
        widget.set_text(line);
        form.add_pair(
            if first { label.into() } else { String::new() },
            Box::new(widget),
        );
    }
}

impl RulesEditor {
    pub(super) fn family_form(&mut self, ui: &mut TheUI, project: &Project) {
        let Some(wizard) = &self.family else { return };
        self.fields.clear();
        self.lists.clear();
        self.nested_fields.clear();
        self.nested_lists.clear();
        self.nested_tables.clear();
        self.choices.clear();
        self.form_path = None;
        let Some(form) = ui.get_text_layout(FORM) else {
            return;
        };
        form.clear();
        form.set_scroll_offset(0);
        note(
            form,
            "Create armor family",
            "Bone pattern: head, torso, legs, hands, feet",
        );
        for (id, label, value) in [
            ("ID", "Stable ID prefix", &wizard.spec.id),
            ("Name", "Display name", &wizard.spec.display_name),
            ("Plate", "Plate ingredient ID", &wizard.spec.plate),
            ("Strap", "Binding ingredient ID", &wizard.spec.strap),
        ] {
            let mut edit = TheTextLineEdit::new(TheId::named(&format!("Rules Family {id}")));
            edit.limiter_mut().set_max_width(420);
            edit.set_value(TheValue::Text(value.clone()));
            edit.set_status_text(
                "Definition IDs are stable references. Display names can change independently.",
            );
            form.add_pair(label.into(), Box::new(edit));
        }
        if let Ok(document) = project.rules_document.document() {
            let catalogue = shared::rulesets::catalogue::Catalogue::from_table(document.table());
            let mut menu = TheDropdownMenu::new(TheId::named("Rules Family Skill"));
            let skills = catalogue.ids("skills");
            for skill in &skills {
                menu.add_option(skill.clone());
            }
            if let Some(index) = skills.iter().position(|s| s == &wizard.spec.skill) {
                menu.set_selected_index(index as i32);
            }
            menu.limiter_mut().set_max_width(420);
            form.add_pair("Crafting skill".into(), Box::new(menu));
            for class in catalogue.ids("classes") {
                let mut checkbox =
                    TheCheckButton::new(TheId::named(&format!("Rules Family Class {class}")));
                checkbox.set_value(TheValue::Bool(wizard.spec.classes.contains(&class)));
                checkbox.set_status_text("Grant this armor category and crafting commands. Unrestricted classes stay unrestricted.");
                form.add_pair(format!("Allow {class}"), Box::new(checkbox));
            }
        }
        let mut create = TheCheckButton::new(TheId::named("Rules Family Materials"));
        create.set_value(TheValue::Bool(wizard.spec.create_missing_materials));
        form.add_pair("Define missing ingredients".into(), Box::new(create));
        note(
            form,
            "World sourcing",
            "Ingredients need authored world sources and production chains.",
        );
        note(form, "", "This guide grants no materials or vendor stock.");
        let mut preview = TheTraybarButton::new(TheId::named("Rules Family Preview"));
        preview.set_text("Preview".into());
        preview.set_fixed_size(false);
        form.add_pair(" ".into(), Box::new(preview));
        if !wizard.message.is_empty() {
            for line in wizard.message.lines() {
                note(form, "Review", line);
            }
        }
        if let Some(plan) = &wizard.preview {
            for line in &plan.summary {
                note(form, "", line);
            }
            note(
                form,
                "New material definitions",
                if plan.added_materials.is_empty() {
                    "none".into()
                } else {
                    plan.added_materials.join(", ")
                },
            );
        }
        for (id, label) in [("Apply", "Create Family"), ("Cancel", "Cancel")] {
            let mut button = TheTraybarButton::new(TheId::named(&format!("Rules Family {id}")));
            button.set_text(label.into());
            button.set_fixed_size(false);
            if id == "Apply" && wizard.preview.is_none() {
                button.set_disabled(true);
            }
            form.add_pair(" ".into(), Box::new(button));
        }
    }

    /// Validate the candidate before updating the workbench draft.
    fn apply_family(&mut self, project: &mut Project) -> Result<(), String> {
        let wizard = self.family.as_ref().ok_or("Open the family guide first")?;
        let plan = wizard
            .preview
            .as_ref()
            .ok_or("Preview the family before creating it")?;
        let mut document = project.rules_document.document()?;
        plan.apply(&mut document)?;
        let source = document.to_toml()?;
        let undo = RulesUndo::from(project.rules_document.clone());
        project.rules_document.current = source;
        self.undo.push(undo);
        self.redo.clear();
        self.selected = Some(vec![
            "equipment_families".into(),
            format!("{}_armor", wizard.spec.id),
        ]);
        self.family = None;
        self.dirty = true;
        Ok(())
    }

    pub(super) fn family_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut CreatorContext,
    ) -> Option<bool> {
        match event {
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Rules Family Open" =>
            {
                let mut spec = BoneArmorFamily::default();
                if let Ok(document) = project.rules_document.document() {
                    let skills =
                        shared::rulesets::catalogue::Catalogue::from_table(document.table())
                            .ids("skills");
                    if !skills.contains(&spec.skill) {
                        spec.skill = skills.first().cloned().unwrap_or_default();
                    }
                }
                self.family = Some(FamilyWizard {
                    spec,
                    preview: None,
                    message: String::new(),
                });
                self.family_form(ui, project);
            }
            TheEvent::ValueChanged(id, value) if id.name.starts_with("Rules Family ") => {
                let Some(wizard) = &mut self.family else {
                    return Some(false);
                };
                match (id.name.as_str(), value) {
                    ("Rules Family ID", TheValue::Text(text)) => {
                        wizard.spec.id = text.trim().into()
                    }
                    ("Rules Family Name", TheValue::Text(text)) => {
                        wizard.spec.display_name = text.clone()
                    }
                    ("Rules Family Plate", TheValue::Text(text)) => {
                        wizard.spec.plate = text.trim().into()
                    }
                    ("Rules Family Strap", TheValue::Text(text)) => {
                        wizard.spec.strap = text.trim().into()
                    }
                    ("Rules Family Materials", TheValue::Bool(value)) => {
                        wizard.spec.create_missing_materials = *value
                    }
                    ("Rules Family Skill", TheValue::Int(index)) => {
                        if let Ok(document) = project.rules_document.document() {
                            if let Some(skill) =
                                shared::rulesets::catalogue::Catalogue::from_table(document.table())
                                    .ids("skills")
                                    .get(*index as usize)
                            {
                                wizard.spec.skill = skill.clone();
                            }
                        }
                    }
                    (_, TheValue::Bool(checked)) if id.name.starts_with("Rules Family Class ") => {
                        let class = id
                            .name
                            .trim_start_matches("Rules Family Class ")
                            .to_string();
                        wizard.spec.classes.retain(|v| v != &class);
                        if *checked {
                            wizard.spec.classes.push(class);
                        }
                    }
                    _ => return Some(false),
                }
                wizard.preview = None;
                wizard.message = "Settings changed. Preview again before creating.".into();
                ui.set_widget_disabled_state("Rules Family Apply", ctx, true);
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Rules Family Preview" =>
            {
                let Some(wizard) = &mut self.family else {
                    return Some(false);
                };
                match project
                    .rules_document
                    .document()
                    .and_then(|d| wizard.spec.preview(&d))
                {
                    Ok(plan) => {
                        wizard.preview = Some(plan);
                        wizard.message =
                            "Review the additions below. Creation is one undoable edit.".into();
                    }
                    Err(error) => {
                        wizard.preview = None;
                        wizard.message = error;
                    }
                }
                self.family_form(ui, project);
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Rules Family Apply" =>
            {
                match self.apply_family(project) {
                    Ok(()) => {
                        self.catalogue(ui, project);
                        self.form(ui, project);
                        self.set_undo_state_to_ui(ctx);
                        if let Err(error) =
                            crate::docks::nodes::sync_live_rules(project, &server.reference)
                        {
                            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                        }
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Content List"),
                            TheValue::Empty,
                        ));
                    }
                    Err(error) => {
                        if let Some(wizard) = &mut self.family {
                            wizard.message = error;
                            wizard.preview = None;
                        }
                        self.family_form(ui, project);
                    }
                }
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == "Rules Family Cancel" =>
            {
                self.family = None;
                self.form(ui, project);
            }
            _ => return None,
        }
        Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn click(
        dock: &mut RulesEditor,
        name: &str,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut CreatorContext,
    ) {
        assert!(dock.handle_event(
            &TheEvent::StateChanged(TheId::named(name), TheWidgetState::Clicked),
            ui,
            ctx,
            project,
            server
        ));
    }
    #[test]
    fn guided_family_preview_apply_undo_redo_and_project_archive_preserve_ids() {
        let mut dock = RulesEditor::new();
        let mut ctx = TheContext::new(1200, 750, 1.0);
        let mut ui = TheUI::new();
        ui.init(&mut ctx);
        ui.canvas = dock.setup(&mut ctx);
        let mut project = Project::new();
        let mut server = CreatorContext::default();
        let before = project.rules_document.clone();
        let count = project.items.len();
        click(
            &mut dock,
            "Rules Family Open",
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert!(ui.get_widget("Rules Family ID").is_some());
        let spec = &mut dock.family.as_mut().unwrap().spec;
        spec.id = "test_bone".into();
        spec.plate = "test_bone_plate".into();
        spec.strap = "test_leather_strap".into();
        click(
            &mut dock,
            "Rules Family Preview",
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert!(dock.family.as_ref().unwrap().preview.is_some());
        if let Ok(path) = std::env::var("ELDIRON_BONE_FAMILY_SNAPSHOT") {
            let mut pixels = vec![0; 1200 * 750 * 4];
            ui.draw(&mut pixels, &mut ctx);
            for id in [
                "Rules Family Open",
                "Rules Family ID",
                "Rules Family Preview",
            ] {
                let dim = *ui.get_widget(id).unwrap().dim();
                assert!(
                    dim.width > 30 && dim.height > 10,
                    "{id} is clipped: {dim:?}"
                );
            }
            std::fs::write(
                path,
                TheRGBABuffer::from(pixels, 1200, 750).to_png().unwrap(),
            )
            .unwrap();
        }

        assert_eq!(
            project.rules_document, before,
            "preview cannot mutate authored rules"
        );
        dock.handle_event(
            &TheEvent::ValueChanged(
                TheId::named("Rules Family Class Warrior"),
                TheValue::Bool(true),
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert!(
            dock.family.as_ref().unwrap().preview.is_none(),
            "changing settings invalidates preview"
        );
        click(
            &mut dock,
            "Rules Family Preview",
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        click(
            &mut dock,
            "Rules Family Apply",
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert!(dock.family.is_none());
        assert_eq!(project.items.len(), count);
        let after = project.rules_document.clone();
        let ids: Vec<_> = project.items.keys().copied().collect();
        let bytes = shared::project_io::encode_project(&project).unwrap();
        let saved = shared::project_io::decode_project(&bytes).unwrap();
        assert_eq!(saved.rules_document, before);
        assert_eq!(
            saved
                .items
                .keys()
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            ids.iter().copied().collect()
        );
        let mut custom = shared::item::Item::new();
        custom.name = "Unrelated custom item".into();
        let custom_id = custom.id;
        project.add_item(custom);
        dock.undo(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(project.rules_document, before);
        assert_eq!(project.items.len(), count + 1);
        assert!(project.items.contains_key(&custom_id));
        dock.redo(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(project.rules_document, after);
        assert!(ids.iter().all(|id| project.items.contains_key(id)));
        assert!(project.items.contains_key(&custom_id));
        click(
            &mut dock,
            "Rules Family Open",
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        click(
            &mut dock,
            "Rules Family Preview",
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert!(dock.family.as_ref().unwrap().preview.is_none());
        assert!(
            dock.family
                .as_ref()
                .unwrap()
                .message
                .contains("already exists")
        );
        let count = project.items.len();
        click(
            &mut dock,
            "Rules Family Apply",
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert_eq!(project.rules_document, after);
        assert_eq!(project.items.len(), count);
    }
}
