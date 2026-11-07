//! One recipe application path for both region geometry and isolated Prefab parts.
use crate::editor::UNDOMANAGER;
use crate::prelude::*;
use shared::geometry_recipe::{self, BINDING, Binding};

pub struct ApplyMZRecipe {
    id: TheId,
    nodeui: TheNodeUI,
    choices: Vec<(Uuid, String, String)>,
}
pub struct EditMZRecipe {
    id: TheId,
}

fn selected_binding(map: &Map) -> Option<(Uuid, Binding)> {
    let id = map
        .selected_geometry_objects
        .first()
        .copied()
        .or_else(|| map.selected_geometry_faces.first().map(|(id, _)| *id))?;
    let object = map.geometry_objects.iter().find(|o| o.id == id)?;
    Some((id, Binding::read(object)?))
}

/// This mutates source/cache data only. The action wrapper owns UI and undo.
pub(crate) fn apply_recipe(
    project: &mut Project,
    server_ctx: &ServerContext,
    choice: &(Uuid, String, String),
) -> Result<(), String> {
    let mut map = project
        .get_map(server_ctx)
        .ok_or("Open a 3D region or Prefab")?
        .clone();
    let selected = selected_binding(&map);
    let binding = if let Some((_, binding)) = &selected {
        let mut binding = binding.clone();
        binding.asset = choice.0;
        binding.source = choice.2.clone();
        binding
    } else {
        // The selected recipe can be a bundled asset not installed in this project yet.
        let mut host = project.clone();
        host.procedural_appearances
            .entry(choice.0)
            .or_insert_with(|| choice.2.clone());
        Binding::from_selection(&host, &map, choice.0)?
    };
    let previous = selected
        .as_ref()
        .and_then(|(id, _)| map.geometry_objects.iter().find(|o| o.id == *id));
    let output = geometry_recipe::generate(project, &map, &binding, previous)?;
    let output_id = output.id;
    project
        .procedural_appearances
        .entry(choice.0)
        .or_insert_with(|| choice.2.clone());
    if let Some((id, _)) = selected {
        map.geometry_objects.retain(|o| o.id != id);
    }
    map.geometry_objects.push(output.clone());
    map.clear_selection();
    map.selected_geometry_objects = vec![output_id];
    if let ProjectContext::Prefab(asset_id) = server_ctx.pc {
        let asset = project
            .block_props
            .get_mut(&asset_id)
            .ok_or("Prefab was removed")?;
        let owner = project
            .prefab_editor_part_by_object
            .get(&output_id)
            .copied();
        let part_id =
            if let Some(part) = owner.and_then(|id| asset.parts.iter_mut().find(|p| p.id == id)) {
                part.geometry_source = rusterix::BlockPropGeometrySource::MZ {
                    binding: output.properties.get_str(BINDING).unwrap().into(),
                    generated_cache: vec![output],
                };
                part.id
            } else {
                let mut part = rusterix::BlockPropPart::new_authored(&choice.1, vec![]);
                part.geometry_source = rusterix::BlockPropGeometrySource::MZ {
                    binding: output.properties.get_str(BINDING).unwrap().into(),
                    generated_cache: vec![output],
                };
                let id = part.id;
                asset.parts.push(part);
                id
            };
        project
            .prefab_editor_part_by_object
            .insert(output_id, part_id);
        project.prefab_editor_map = Some(map);
        crate::block_props::sync_prefab_editor(project, asset_id)?;
    } else {
        *project.get_map_mut(server_ctx).ok_or("Map was removed")? = map;
    }
    Ok(())
}

impl Action for ApplyMZRecipe {
    fn new() -> Self {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown("help".into(),"Apply an MZ recipe to selected objects or faces. Input geometry is retained. Select generated geometry and use Edit MZ Recipe to edit its published controls.".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            "recipe".into(),
            "Recipe".into(),
            "Recipes with a selection input, including imported project models.".into(),
            vec!["Pitched roof".into()],
            0,
        ));
        Self {
            id: TheId::named("Apply MZ Recipe"),
            nodeui,
            choices: vec![],
        }
    }
    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        "Generate fitted geometry from an editable MZ recipe.".into()
    }
    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }
    fn is_applicable(&self, map: &Map, _: &mut TheContext, server_ctx: &ServerContext) -> bool {
        (server_ctx.pc.is_region() || server_ctx.pc.is_prefab())
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && (!map.selected_geometry_objects.is_empty()
                || !map.selected_geometry_faces.is_empty())
    }
    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        self.choices = geometry_recipe::choices(project);
        let selected = project
            .get_map(server_ctx)
            .and_then(selected_binding)
            .and_then(|(_, b)| self.choices.iter().position(|c| c.0 == b.asset))
            .unwrap_or(0);
        if let Some(TheNodeUIItem::Selector(_, _, _, names, index)) =
            self.nodeui.get_item_mut("recipe")
        {
            *names = self.choices.iter().map(|c| c.1.clone()).collect();
            *index = selected as i32;
        }
    }
    fn apply_project(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let index = self.nodeui.get_i32_value("recipe").unwrap_or(0).max(0) as usize;
        let Some(choice) = self.choices.get(index) else {
            return;
        };
        let before = project.clone();
        if let Err(error) = apply_recipe(project, server_ctx, choice) {
            *project = before;
            ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
            return;
        }
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                "Apply MZ Recipe".into(),
                Box::new(before),
                Box::new(project.clone()),
            ),
            ctx,
        );
        crate::utils::editor_scene_full_rebuild(project, server_ctx);
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named(crate::docks::blocks::BLOCKS_DOCK_SYNC_EVENT),
            TheValue::Empty,
        ));
        ctx.ui.redraw_all = true;
        let _ = ui;
    }
    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }
    fn handle_event(
        &mut self,
        event: &TheEvent,
        _: &mut Project,
        _: &mut TheUI,
        _: &mut TheContext,
        _: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}

impl Action for EditMZRecipe {
    fn new() -> Self {
        Self {
            id: TheId::named("Edit MZ Recipe"),
        }
    }
    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        "Open the selected geometry's shared MZ recipe and published controls.".into()
    }
    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }
    fn is_applicable(&self, map: &Map, _: &mut TheContext, _: &ServerContext) -> bool {
        selected_binding(map).is_some()
    }
    fn apply(
        &self,
        map: &mut Map,
        _: &mut TheUI,
        ctx: &mut TheContext,
        _: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        if let Some((_, binding)) = selected_binding(map) {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Open Geometry Recipe"),
                TheValue::Id(binding.asset),
            ));
        }
        None
    }
    fn params(&self) -> TheNodeUI {
        TheNodeUI::default()
    }
    fn handle_event(
        &mut self,
        _: &TheEvent,
        _: &mut Project,
        _: &mut TheUI,
        _: &mut TheContext,
        _: &mut ServerContext,
    ) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_recipe_path_works_on_map_and_prefab_and_preserves_selected_input() {
        let mut p = Project::default();
        let id = p.regions[0].id;
        let mut ctx = ServerContext::default();
        ctx.pc = ProjectContext::Region(id);
        ctx.curr_region = id;
        let frame =
            rusterix::GeometryObject::box_("Frame", Vec3::new(0., 1., 0.), Vec3::new(6., 2., 4.));
        let map = &mut p.regions[0].map;
        map.geometry_objects = vec![frame.clone()];
        map.selected_geometry_objects = vec![frame.id];
        let choice = geometry_recipe::choices(&p).remove(0);
        let before = p.clone();
        apply_recipe(&mut p, &ctx, &choice).unwrap();
        assert_eq!(p.regions[0].map.geometry_objects[0], frame);
        assert_eq!(p.regions[0].map.geometry_objects.len(), 2);
        let output = p.regions[0].map.geometry_objects[1].id;
        apply_recipe(&mut p, &ctx, &choice).unwrap();
        assert_eq!(p.regions[0].map.geometry_objects.len(), 2);
        assert_eq!(p.regions[0].map.geometry_objects[1].id, output);
        // Same operation in the isolated Prop editor creates a recipe-backed part.
        let mut asset = rusterix::BlockPropAsset::new_authored("Frame", vec![frame.clone()]);
        let asset_id = asset.id;
        asset.parts[0].pivot = [0.1, 0., 0.];
        let original_part = asset.parts[0].clone();
        p.block_props.insert(asset_id, asset);
        crate::block_props::begin_prefab_editor(&mut p, asset_id).unwrap();
        ctx.pc = ProjectContext::Prefab(asset_id);
        p.prefab_editor_map
            .as_mut()
            .unwrap()
            .selected_geometry_objects = vec![frame.id];
        apply_recipe(&mut p, &ctx, &choice).unwrap();
        assert_eq!(p.block_props[&asset_id].parts[0], original_part);
        assert!(matches!(
            p.block_props[&asset_id].parts[1].geometry_source,
            rusterix::BlockPropGeometrySource::MZ { .. }
        ));
        let reopened =
            shared::project_io::decode_project(&shared::project_io::encode_project(&p).unwrap())
                .unwrap();
        assert!(matches!(
            reopened.block_props[&asset_id].parts[1].geometry_source,
            rusterix::BlockPropGeometrySource::MZ { .. }
        ));
        ctx.pc = ProjectContext::Region(id);
        let instance = rusterix::BlockPropInstance::new(asset_id);
        p.regions[0].map.selected_block_prop_instances = vec![instance.id];
        p.regions[0].map.block_prop_instances.push(instance);
        let unique = crate::block_props::make_selected_block_prop_unique(&mut p, &ctx).unwrap();
        let (changed, errors) = geometry_recipe::synchronize(&mut p);
        assert!(changed);
        assert!(errors.is_empty(), "{errors:?}");
        let asset = &p.block_props[&unique.unique_asset_id];
        let output = &asset.parts[1].geometry_source.geometry_objects()[0];
        let binding = Binding::read(output).unwrap();
        assert_eq!(
            binding.objects,
            vec![asset.parts[0].geometry_source.geometry_objects()[0].id]
        );
        let _undo =
            ProjectUndoAtom::ProjectEdit("Apply MZ Recipe".into(), Box::new(before), Box::new(p));
    }
}
