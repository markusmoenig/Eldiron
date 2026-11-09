//! Import/export and advanced asset actions. Normal control publishing is visual.
use super::*;
use denrim_materials::geometry::group::{Control, Modifier};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Controls {
    controls: Vec<Control>,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Timeline {
    events: Vec<denrim_materials::geometry::rig::Event>,
}

fn template(document: &Document) -> Result<Vec<Control>, String> {
    let mut controls = vec![];
    for axis in 0..3 {
        let mut scale = [0.; 3];
        scale[axis] = 1.;
        controls.push(Control {
            expression: None,
            id: ["width", "height", "depth"][axis].into(),
            label: ["Width", "Height", "Depth"][axis].into(),
            category: "Shape".into(),
            help: "Scale relative to the authored size; 1 is unchanged".into(),
            order: axis as u16,
            value: 1.,
            reference: 1.,
            min: 0.25,
            max: 3.,
            modifiers: vec![Modifier::Affine {
                parts: vec!["*".into()],
                joints: vec!["*".into()],
                pivot: [0.; 3],
                scale,
                translation: [0.; 3],
                weight: None,
                frames: true,
            }],
        });
    }
    let mesh = document.evaluate().map_err(|e| e.to_string())?.mesh;
    let palette = mesh
        .parts
        .iter()
        .find_map(|p| p.surface.as_ref().map(|s| s.palette))
        .unwrap_or(0);
    let color_parts = mesh
        .parts
        .iter()
        .filter(|p| p.surface.as_ref().is_some_and(|s| s.palette == palette))
        .map(|p| p.id.clone())
        .collect();
    controls.push(Control {
        expression: None,
        id: "color".into(),
        label: "Color".into(),
        category: "Appearance".into(),
        help: "Project palette slot".into(),
        order: 0,
        value: palette as f32,
        reference: palette as f32,
        min: 0.,
        max: 255.,
        modifiers: vec![Modifier::Palette { parts: color_parts }],
    });
    Ok(controls)
}
pub(super) fn portable(project: &Project, source: &str) -> Result<String, String> {
    let doc = Document::parse(source).map_err(|e| e.to_string())?;
    let mut doc = shared::appearance_assets::resolve_dependencies(project, &doc)?;
    fn embed(doc: &mut Document, project: &Project) -> Result<(), String> {
        for tile in doc.rows.iter_mut().flat_map(|r| &mut r.tiles) {
            match &mut tile.op {
                Operation::Asset { document, .. }
                | Operation::Group { document, .. }
                | Operation::Input {
                    preview: document, ..
                } => embed(document, project)?,
                Operation::Texture {
                    source,
                    bundle,
                    size,
                    ..
                } => {
                    let spec = denrim_materials::geometry::Texture {
                        source: source.clone(),
                        bundle: bundle.clone(),
                        size: *size,
                    };
                    *bundle = Some(
                        textures::asset(&spec, &project.palette, Some(project))?
                            .save()
                            .map_err(|e| e.to_string())?,
                    );
                }
                _ => (),
            }
        }
        Ok(())
    }
    embed(&mut doc, project)?;
    let text = doc.save().map_err(|e| e.to_string())?;
    if text.len() > 4 * 1024 * 1024 {
        return Err("The portable model exceeds 4 MB; reduce embedded image resolution or frame count before exporting".into());
    }
    Ok(text)
}
impl GeometryEditor {
    pub(super) fn asset_actions(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        let result: Option<Result<(), String>> = match event {
            TheEvent::IndexChanged(id, index) if id.name == "MZ Geometry Actions" => {
                layout::set_choice(ui, "MZ Geometry Actions", 0);
                Some((|| {
                    match index {
                        5 => {
                            self.finish_drag(project, ctx, false);
                            let selected = self.selected.ok_or("Select a geometry asset")?;
                            let op = self
                                .selected_operation
                                .clone()
                                .ok_or("Select a Group tile, then publish its controls")?;
                            let doc = self.document.as_ref().ok_or("Select a model")?;
                            let tile = doc
                                .rows
                                .iter()
                                .flat_map(|r| &r.tiles)
                                .find(|t| t.id == op)
                                .ok_or("Select a Group tile")?;
                            let Operation::Group {
                                controls, document, ..
                            } = &tile.op
                            else {
                                return Err("Select a Group tile. Its published controls will appear on that tile.".into());
                            };
                            let controls = if controls.is_empty() {
                                template(document)?
                            } else {
                                controls.clone()
                            };
                            let text = format!(
                                "# Published group controls. Parts and joints accept stable IDs or '*'.\n# value/reference define the neutral pose; min/max define the editor range.\n{}",
                                toml::to_string_pretty(&Controls { controls })
                                    .map_err(|e| e.to_string())?
                            );
                            self.control_target = Some((
                                selected,
                                self.group_path.clone(),
                                op,
                                project.procedural_appearances[&selected].clone(),
                            ));
                            let mut canvas = TheCanvas::new();
                            canvas.limiter_mut().set_max_size(Vec2::new(720, 520));
                            let mut editor = TheTextAreaEdit::new(TheId::named("Dialog Value"));
                            if let Some(bytes) = crate::Embedded::get("parser/TOML.sublime-syntax")
                            {
                                if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                                    editor.add_syntax_from_string(source);
                                    editor.set_code_type("TOML");
                                }
                            }
                            editor.set_text(text);
                            canvas.set_widget(editor);
                            ui.show_dialog(
                                "Publish group controls",
                                canvas,
                                vec![TheDialogButtonRole::Accept, TheDialogButtonRole::Reject],
                                ctx,
                            );
                        }
                        2 => {
                            let selected = self.selected.ok_or("Select a model")?;
                            let op = self
                                .selected_operation
                                .clone()
                                .ok_or("Select a Motion tile")?;
                            let tile = self
                                .document
                                .as_ref()
                                .ok_or("Select a model")?
                                .rows
                                .iter()
                                .flat_map(|r| &r.tiles)
                                .find(|t| t.id == op)
                                .ok_or("Select a Motion tile")?;
                            let Operation::Motion { events, .. } = &tile.op else {
                                return Err(
                                    "Select a Motion tile to edit its animation events".into()
                                );
                            };
                            self.control_target = Some((
                                selected,
                                self.group_path.clone(),
                                op,
                                project.procedural_appearances[&selected].clone(),
                            ));
                            let mut canvas = TheCanvas::new();
                            canvas.limiter_mut().set_max_size(Vec2::new(600, 400));
                            let mut editor = TheTextAreaEdit::new(TheId::named("Dialog Value"));
                            editor.set_text(format!(
                                "# Named presentation events, ordered by phase (0–1).
# Combat and projectile logic remains controlled by the ruleset.
# Example:
# [[events]]
# time = 0.5
# name = \"release\"
{}",
                                toml::to_string_pretty(&Timeline {
                                    events: events.clone()
                                })
                                .map_err(|e| e.to_string())?
                            ));
                            canvas.set_widget(editor);
                            ui.show_dialog(
                                "Animation events",
                                canvas,
                                vec![TheDialogButtonRole::Accept, TheDialogButtonRole::Reject],
                                ctx,
                            );
                        }
                        3 => ctx.ui.open_file_requester(
                            TheId::named("MZ Model Import"),
                            "Import model".into(),
                            TheFileExtension::new("MZ geometry".into(), vec!["json".into()]),
                        ),
                        4 => ctx.ui.save_file_requester(
                            TheId::named_with_id(
                                "MZ Model Export",
                                self.selected.ok_or("Select a model to export")?,
                            ),
                            "Export model".into(),
                            TheFileExtension::new("MZ geometry".into(), vec!["json".into()]),
                        ),
                        _ => (),
                    }
                    Ok(())
                })())
            }
            TheEvent::DialogValueOnClose(role, name, _, value)
                if name == "Publish group controls" || name == "Animation events" =>
            {
                Some((|| {
                    let target = self.control_target.take();
                    if *role != TheDialogButtonRole::Accept {
                        return Ok(());
                    }
                    let (selected, path, operation, before) =
                        target.ok_or("The group selection changed")?;
                    if self.selected != Some(selected)
                        || self.group_path != path
                        || project.procedural_appearances.get(&selected) != Some(&before)
                    {
                        return Err("The model changed while controls were open. Reopen Publish controls to use its current version.".into());
                    }
                    let TheValue::Text(text) = value else {
                        return Err("Controls must be TOML".into());
                    };
                    let mut next = self.document.clone().ok_or("Select a model")?;
                    let tile = next
                        .rows
                        .iter_mut()
                        .flat_map(|r| &mut r.tiles)
                        .find(|t| t.id == operation)
                        .ok_or("The Group tile was removed")?;
                    if name == "Animation events" {
                        let data: Timeline =
                            toml::from_str(text).map_err(|e| format!("Events: {e}"))?;
                        let Operation::Motion { events, .. } = &mut tile.op else {
                            return Err("Select a Motion tile".into());
                        };
                        *events = data.events;
                    } else {
                        let data: Controls =
                            toml::from_str(text).map_err(|e| format!("Controls: {e}"))?;
                        let Operation::Group { controls, .. } = &mut tile.op else {
                            return Err("Select a Group tile".into());
                        };
                        *controls = data.controls;
                    }
                    self.commit(next, project, ctx)
                })())
            }
            TheEvent::FileRequesterResult(id, paths)
                if id.name == "MZ Model Export" || id.name == "MZ Model Import" =>
            {
                Some((|| {
                    let Some(path) = paths.first() else {
                        return Ok(());
                    };
                    if id.name == "MZ Model Export" {
                        let source = project
                            .procedural_appearances
                            .get(&id.uuid)
                            .ok_or("The exported model was removed")?;
                        std::fs::write(path, portable(project, source)?)
                            .map_err(|e| e.to_string())?;
                    } else {
                        if std::fs::metadata(path).map_err(|e| e.to_string())?.len()
                            > 4 * 1024 * 1024
                        {
                            return Err("Model document exceeds 4 MB".into());
                        }
                        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
                        let mut doc = Document::load(&text).map_err(|e| e.to_string())?;
                        let selected = Uuid::new_v4();
                        doc.id = selected.to_string();
                        let text = doc.save().map_err(|e| e.to_string())?;
                        project
                            .procedural_appearances
                            .insert(selected, text.clone());
                        UNDOMANAGER.write().unwrap().add_undo(
                            ProjectUndoAtom::ProceduralAppearanceEdit(selected, None, Some(text)),
                            ctx,
                        );
                        self.selected = Some(selected);
                        self.group_path.clear();
                        self.key = None;
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Procedural Tiles Restored"),
                            TheValue::Empty,
                        ));
                    }
                    Ok(())
                })())
            }
            _ => None,
        };
        if let Some(result) = result {
            if let Err(error) = result {
                Self::status(ctx, error)
            }
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_controls_roundtrip_toml_and_deform_the_same_group() {
        let doc = Document::load(denrim_materials::geometry::SWORD_EXAMPLE).unwrap();
        let controls = template(&doc).unwrap();
        let text = toml::to_string_pretty(&Controls {
            controls: controls.clone(),
        })
        .unwrap();
        let mut decoded: Controls = toml::from_str(&text).unwrap();
        assert_eq!(decoded.controls, controls);
        let mut mesh = doc.evaluate().unwrap().mesh;
        let (lo, hi) = mesh.bounds();
        decoded.controls[0].value = 1.5;
        denrim_materials::geometry::group::apply(&mut mesh, &decoded.controls).unwrap();
        let (after_lo, after_hi) = mesh.bounds();
        assert!(((after_hi[0] - after_lo[0]) / (hi[0] - lo[0]) - 1.5).abs() < 0.001);
        assert!((after_hi[1] - hi[1]).abs() < 0.001);
    }

    #[test]
    fn exported_models_include_linked_geometry_and_materials_without_the_source_project() {
        let mut p = Project::default();
        let model = Uuid::new_v4();
        let mut sword = Document::load(denrim_materials::geometry::SWORD_EXAMPLE).unwrap();
        let material = Uuid::new_v4();
        let a=denrim_materials::integration::EditableMaterialAsset::parse(&material.to_string(),"project 1\nmaterial \"m\" called \"Surface\"\n  mode pbr\n  row \"r\"\n    use \"Color\" id \"color\"\n  output base color from row \"r\" xyz\n","m").unwrap();
        p.procedural_materials.insert(material, a.save().unwrap());
        sword
            .rows
            .iter_mut()
            .find(|r| r.id == sword.output)
            .unwrap()
            .tiles
            .push(Tile {
                id: "surface".into(),
                op: Operation::Texture {
                    preserve_uv: true,
                    parts: vec![],
                    source: material.to_string(),
                    bundle: None,
                    size: 32,
                    axes: [0, 1],
                    origin: [0.; 2],
                    span: [1.; 2],
                },
            });
        p.procedural_appearances
            .insert(model, sword.save().unwrap());
        let parent = Document {
            version: 1,
            id: Uuid::new_v4().to_string(),
            label: "Shared sword".into(),
            rows: vec![Row {
                id: "asset".into(),
                label: "Sword".into(),
                tiles: vec![Tile {
                    id: "linked".into(),
                    op: Operation::Asset {
                        source: model.to_string(),
                        document: Box::new(sword),
                    },
                }],
            }],
            output: "asset".into(),
            grip: None,
        };
        let exported = portable(&p, &parent.save().unwrap()).unwrap();
        let fresh = Project::default();
        let doc = shared::appearance_assets::resolve(&fresh, &Document::load(&exported).unwrap())
            .unwrap();
        let e = doc.evaluate().unwrap();
        let t = e
            .mesh
            .parts
            .iter()
            .find_map(|part| part.texture.as_ref())
            .unwrap();
        assert!(t.bundle.is_some());
        assert!(textures::asset(t, &fresh.palette, None).is_ok());
        assert!(!e.mesh.parts[0].vertices.is_empty());
    }
}
