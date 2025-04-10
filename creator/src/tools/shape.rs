use crate::editor::SHAPEPICKER;
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;
use rusterix::Shape;

pub struct ShapeTool {
    id: TheId,

    edit_mode_index: i32,
}

impl Tool for ShapeTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Shape Tool"),

            edit_mode_index: 0,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Effects Tool (X). Apply lighting and effects to the map.")
    }
    fn icon_name(&self) -> String {
        str!("pentagram")
    }
    fn accel(&self) -> Option<char> {
        Some('g')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::ShapePicker as usize,
                ));

                server_ctx.curr_map_tool_type = MapToolType::Effects;

                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    // Material Group
                    let mut gb = TheGroupButton::new(TheId::named("Effects Mode Group"));
                    gb.add_text_status(str!("Add"), str!("Add the current effects to new tiles."));
                    gb.add_text_status(str!("Edit"), str!("Edit the effects of existing tiles."));
                    gb.add_text_status(
                        str!("Delete"),
                        str!("Delete the effects on existing tiles."),
                    );
                    gb.set_item_width(85);

                    gb.set_index(self.edit_mode_index);

                    layout.add_widget(Box::new(gb));
                }

                true
            }
            DeActivate => true,
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        let mut undo_atom: Option<RegionUndoAtom> = None;

        match map_event {
            MapKey(c) => {
                match c {
                    '1'..='9' => map.subdivisions = (c as u8 - b'0') as f32,
                    '0' => map.subdivisions = 10.0,
                    _ => {}
                }
                crate::editor::RUSTERIX.write().unwrap().set_dirty();
            }
            MapClicked(coord) => {
                let prev = map.clone();
                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();

                    let grid_pos = server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    );

                    if let Some(shape_type) = SHAPEPICKER.read().unwrap().curr_shape_type {
                        let old_subdiv = map.subdivisions;
                        map.subdivisions = 10.0;
                        let mut shape = Shape::new_with_type(shape_type);
                        shape.center = grid_pos;

                        let ids = shape.create(map);

                        map.subdivisions = old_subdiv;
                        map.selected_vertices = vec![];
                        map.selected_linedefs = vec![];
                        map.selected_sectors = ids;

                        crate::editor::RUSTERIX.write().unwrap().set_dirty();
                        undo_atom = Some(RegionUndoAtom::MapEdit(
                            Box::new(prev),
                            Box::new(map.clone()),
                        ));
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Map Selection Changed"),
                            TheValue::Empty,
                        ));
                    }
                }
            }
            MapDragged(_coord) => {}
            MapUp(_) => {}
            MapHover(coord) => {
                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();
                    map.curr_mouse_pos = Some(server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    ));
                    let mut hover = server_ctx.geometry_at(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                    );
                    hover.0 = None;
                    hover.2 = None;

                    server_ctx.hover = hover;
                    let cp = server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    );
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Cursor Pos Changed"),
                        TheValue::Float2(cp),
                    ));
                    server_ctx.hover_cursor = Some(cp);
                }
            }
            MapDelete => {}
            MapEscape => {}
        }
        undo_atom
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        #[allow(clippy::single_match)]
        match &event {
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Effects Mode Group" {
                    self.edit_mode_index = *index as i32;
                }
            }
            _ => {}
        }
        false
    }
}
