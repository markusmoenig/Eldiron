use crate::hud::{Hud, HudMode};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;

use crate::editor::{CONFIG, CONFIGEDITOR};

pub struct ConfigTool {
    id: TheId,

    hud: Hud,
}

impl Tool for ConfigTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Config Tool"),

            hud: Hud::new(HudMode::Effects),
        }
    }
    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Config Tool.")
    }
    fn icon_name(&self) -> String {
        str!("gear")
    }
    fn accel(&self) -> Option<char> {
        None //Some('x')
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::ConfigEditor as usize,
                ));

                ui.set_widget_value("ConfigEdit", ctx, TheValue::Text(project.config.clone()));
                server_ctx.curr_map_tool_type = MapToolType::General;

                true
            }
            DeActivate => true,
            _ => false,
        }
    }

    fn draw_hud(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        self.hud.draw(buffer, map, ctx, server_ctx, None);
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
                if self.hud.clicked(coord.x, coord.y, map, ui, ctx, server_ctx) {
                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                    return None;
                }

                let prev = map.clone();
                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();

                    let grid_pos = server_ctx.local_to_map_grid(
                        Vec2::new(dim.width as f32, dim.height as f32),
                        Vec2::new(coord.x as f32, coord.y as f32),
                        map,
                        map.subdivisions,
                    );

                    let mut found_selected = false;

                    for (i, l) in map.lights.iter().enumerate() {
                        let lp = l.position();
                        let lp = Vec2::new(lp.x, lp.z);
                        let d = lp.distance(grid_pos);
                        if d < 1.0 {
                            map.selected_light = Some(i as u32);
                            crate::editor::RUSTERIX.write().unwrap().set_dirty();
                            undo_atom = Some(RegionUndoAtom::MapEdit(
                                Box::new(prev.clone()),
                                Box::new(map.clone()),
                            ));
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Map Selection Changed"),
                                TheValue::Empty,
                            ));
                            found_selected = true;
                            break;
                        }
                    }

                    if !found_selected {
                        if let Some(effect) = &server_ctx.curr_effect {
                            if let Some(light) = effect.to_light(grid_pos) {
                                map.selected_light = Some(map.lights.len() as u32);
                                map.lights.push(light);
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
                }
            }
            MapDragged(_coord) => {
                // if self.hud.dragged(coord.x, coord.y, map, ui, ctx, server_ctx) {
                //     crate::editor::RUSTERIX.lock().unwrap().set_dirty();
                //     return None;
                // }
                // crate::editor::RUSTERIX.lock().unwrap().set_dirty();
            }
            MapUp(_) => {}
            MapHover(coord) => {
                if let Some(render_view) = ui.get_render_view("PolyView") {
                    let dim = *render_view.dim();
                    map.curr_mouse_pos = Some(Vec2::new(coord.x as f32, coord.y as f32));
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

                    crate::editor::RUSTERIX.write().unwrap().set_dirty();
                }
            }
            MapDelete => {
                if let Some(index) = map.selected_light {
                    let prev = map.clone();
                    _ = map.lights.remove(index as usize);
                    map.selected_light = None;
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
            MapEscape => {}
        }
        undo_atom
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::ValueChanged(id, value) => {
                if id.name == "ConfigEdit" {
                    if let Some(config_string) = value.to_string() {
                        project.config = config_string;
                        if let Ok(toml) = project.config.parse::<Table>() {
                            *CONFIG.write().unwrap() = toml;
                        }
                        CONFIGEDITOR.write().unwrap().read_defaults();
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
