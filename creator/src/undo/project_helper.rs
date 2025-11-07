use crate::editor::{DOCKMANAGER, RUSTERIX};
use crate::prelude::*;
use theframework::prelude::*;

/// Generate a tree node for the given region
pub fn gen_region_tree_node(region: &Region) -> TheTreeNode {
    let mut node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(&region.name, region.id));

    let mut item = TheTreeItem::new(TheId::named_with_reference("Region Item", region.id));
    item.set_text("Name".into());

    let name = format!("Region Item Name Edit: {}", region.name);
    let mut edit = TheTextLineEdit::new(TheId::named_with_id(&name, region.id));
    edit.set_text(region.name.clone());
    item.add_widget_column(200, Box::new(edit));
    node.add_widget(Box::new(item));

    for (id, character) in &region.characters {
        let mut item = TheTreeItem::new(TheId::named_with_id("Region Content List Item", *id));
        item.add_value_column(200, TheValue::Text("Character Instance".to_string()));
        item.set_text(character.name.clone());
        node.add_widget(Box::new(item));
    }

    node
}

/// Rerender the current region.
pub fn update_region(ctx: &mut TheContext) {
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

/// Set the project context and the current docker.
pub fn set_project_context(
    ctx: &mut TheContext,
    ui: &mut TheUI,
    project: &Project,
    server_ctx: &mut ServerContext,
    pc: ProjectContext,
) {
    // println!("set_project_context {:?}", pc);
    if let Some(old_id) = server_ctx.pc.id() {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(node) = tree_layout.get_node_by_id_mut(&old_id) {
                if let Some(snapper) = node.widget.as_any().downcast_mut::<TheSnapperbar>() {
                    snapper.set_selected(false);
                }
            }
        }
    }

    server_ctx.pc = pc;

    match pc {
        ProjectContext::Region(id) => {
            if let Some(region) = project.get_region(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Region: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::Character(id) => {
            if let Some(region) = project.characters.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Character: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Tiles".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::CharacterCode(id) => {
            if let Some(region) = project.characters.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Character: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Code".into(), ui, ctx, project, server_ctx);
        }
        ProjectContext::CharacterData(id) => {
            if let Some(region) = project.characters.get(&id) {
                ui.set_widget_value(
                    "Project Context",
                    ctx,
                    TheValue::Text(format!("Character: {}", region.name)),
                );
            }
            DOCKMANAGER
                .write()
                .unwrap()
                .set_dock("Data".into(), ui, ctx, project, server_ctx);
        }
        _ => {}
    }

    if let Some(new_id) = pc.id() {
        if let Some(tree_layout) = ui.get_tree_layout("Project Tree") {
            if let Some(node) = tree_layout.get_node_by_id_mut(&new_id) {
                if let Some(snapper) = node.widget.as_any().downcast_mut::<TheSnapperbar>() {
                    snapper.set_selected(true);
                }
            }
        }
    }

    ctx.ui.relayout = true;
}
