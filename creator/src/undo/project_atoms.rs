// use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use theframework::prelude::*;

// #[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProjectUndoAtom {
    AddRegion(Uuid),
    RemoveRegion(usize, Region),
}

use ProjectUndoAtom::*;

impl ProjectUndoAtom {
    pub fn undo(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        match self {
            AddRegion(id) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        project.remove_region(&id);
                        region_node.remove_child_by_uuid(id);
                    }
                }
            }
            RemoveRegion(index, region) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let mut region = region.clone();
                    region.map.name = region.name.clone();

                    let mut node = gen_region_tree_node(&region);
                    node.set_open(true);
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.add_child_at(*index, node);
                    }
                    let region_id: Uuid = region.id;
                    project.regions.insert(*index, region);

                    server_ctx.curr_region = region_id;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Region(region_id),
                    );
                    update_region(ctx);
                    ctx.ui.relayout = true;
                }
            }
        }
    }
    pub fn redo(
        &self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        match self {
            AddRegion(id) => {
                // Add Region
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    let mut region = Region::new();
                    region.id = *id;
                    region.map.name = region.name.clone();

                    let mut node = gen_region_tree_node(&region);
                    node.set_open(true);
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.add_child(node);
                    }
                    let region_id: Uuid = region.id;
                    project.regions.push(region);

                    server_ctx.curr_region = region_id;
                    set_project_context(
                        ctx,
                        ui,
                        project,
                        server_ctx,
                        ProjectContext::Region(region_id),
                    );
                    update_region(ctx);
                    ctx.ui.relayout = true;
                }
            }
            RemoveRegion(_, region) => {
                if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
                    if let Some(region_node) =
                        tree_layout.get_node_by_id_mut(&server_ctx.tree_regions_id)
                    {
                        region_node.remove_child_by_uuid(&region.id);
                    }
                    project.remove_region(&region.id);

                    if let Some(region) = project.regions.first() {
                        if let Some(region_node) = tree_layout.get_node_by_id_mut(&region.id) {
                            region_node.set_open(true);
                        }
                        server_ctx.curr_region = region.id;
                        set_project_context(
                            ctx,
                            ui,
                            project,
                            server_ctx,
                            ProjectContext::Region(region.id),
                        );
                        update_region(ctx);
                    }
                    ctx.ui.relayout = true;
                }
            }
        }
    }
}
