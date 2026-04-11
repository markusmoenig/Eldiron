use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use rusterix::{TileGroup, TileGroupMemberRef, TileSource};

pub struct DuplicateTile {
    id: TheId,
    nodeui: TheNodeUI,
}

impl DuplicateTile {
    fn append_board_position(project: &Project, span: Vec2<i32>) -> Vec2<i32> {
        let pack_cols = (project.tile_board_cols - 1).max(1);
        let mut occupied: FxHashSet<(i32, i32)> = FxHashSet::default();

        for pos in project.tile_board_tiles.values() {
            occupied.insert((pos.x, pos.y));
        }
        for pos in project.tile_board_groups.values() {
            let group_span = project
                .tile_groups
                .iter()
                .find(|(id, _)| project.tile_board_groups.get(*id) == Some(pos))
                .map(|(_, group)| Vec2::new(group.width as i32, group.height as i32))
                .unwrap_or(Vec2::new(1, 1));
            for dy in 0..group_span.y.max(1) {
                for dx in 0..group_span.x.max(1) {
                    occupied.insert((pos.x + dx, pos.y + dy));
                }
            }
        }
        for pos in project.tile_board_empty_slots() {
            occupied.insert((pos.x, pos.y));
        }

        for y in 0..(project.tile_board_rows.max(8) + 32) {
            for x in 0..=(pack_cols - span.x).max(0) {
                let mut fits = true;
                for dy in 0..span.y.max(1) {
                    for dx in 0..span.x.max(1) {
                        if occupied.contains(&(x + dx, y + dy)) {
                            fits = false;
                            break;
                        }
                    }
                    if !fits {
                        break;
                    }
                }
                if fits {
                    return Vec2::new(x, y);
                }
            }
        }

        Vec2::zero()
    }

    fn clone_tile_group(
        &self,
        project: &mut Project,
        group_id: Uuid,
        server_ctx: &mut ServerContext,
        ctx: &mut TheContext,
    ) {
        let Some(group) = project.tile_groups.get(&group_id).cloned() else {
            return;
        };

        let mut cloned_group = TileGroup::new(group.width, group.height);
        cloned_group.name = if group.name.is_empty() {
            "New Group".to_string()
        } else {
            format!("{} Copy", group.name)
        };
        cloned_group.tags = group.tags.clone();

        for member in &group.members {
            if let Some(mut tile) = project.tiles.get(&member.tile_id).cloned() {
                let new_tile_id = Uuid::new_v4();
                tile.id = new_tile_id;
                project.tiles.insert(new_tile_id, tile);
                cloned_group.members.push(TileGroupMemberRef {
                    tile_id: new_tile_id,
                    x: member.x,
                    y: member.y,
                });
            }
        }

        let new_group_id = cloned_group.id;
        project.add_tile_group(cloned_group);

        if let Some(node_group) = project.tile_node_groups.get(&group_id).cloned() {
            let mut cloned_node_group = node_group;
            cloned_node_group.group_id = new_group_id;
            cloned_node_group.graph_id = Uuid::new_v4();
            if !cloned_node_group.graph_name.is_empty() {
                cloned_node_group.graph_name = format!("{} Copy", cloned_node_group.graph_name);
            }
            project.add_tile_node_group(cloned_node_group);
        }

        let pos =
            Self::append_board_position(project, Vec2::new(group.width as i32, group.height as i32));
        project.ensure_tile_board_space(pos + Vec2::new(group.width as i32, group.height as i32));
        project.set_tile_board_position(TileSource::TileGroup(new_group_id), pos);

        server_ctx.curr_tile_source = Some(TileSource::TileGroup(new_group_id));
        server_ctx.tile_node_group_id = project.is_tile_node_group(&new_group_id).then_some(new_group_id);
        server_ctx.curr_tile_id = project
            .tile_groups
            .get(&new_group_id)
            .and_then(|group| group.members.first())
            .map(|member| member.tile_id);

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tilepicker"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tiles"),
            TheValue::Empty,
        ));
    }
}

impl Action for DuplicateTile {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_duplicate_tile_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_duplicate_tile")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_duplicate_tile_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        DOCKMANAGER.read().unwrap().dock == "Tiles"
            && (server_ctx.curr_tile_source.is_some() || server_ctx.curr_tile_id.is_some())
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(source) = server_ctx.curr_tile_source {
            match source {
                TileSource::TileGroup(group_id) => {
                    self.clone_tile_group(project, group_id, server_ctx, ctx);
                    return;
                }
                TileSource::TileGroupMember { group_id, .. } => {
                    if project.is_tile_node_group(&group_id) {
                        self.clone_tile_group(project, group_id, server_ctx, ctx);
                        return;
                    }
                }
                _ => {}
            }
        }

        if let Some(tile_id) = server_ctx.curr_tile_id {
            if let Some(mut tile) = project.tiles.get(&tile_id).cloned() {
                tile.id = Uuid::new_v4();
                project.tiles.insert(tile.id, tile);

                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Tilepicker"),
                    TheValue::Empty,
                ));

                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Tiles"),
                    TheValue::Empty,
                ));
            }
        }
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
