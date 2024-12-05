use shared::server::prelude::MapToolType;

use crate::editor::{CODEEDITOR, MAPRENDER, TILEDRAWER, UNDOMANAGER};
use crate::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum EditorDrawMode {
    Draw2D,
    DrawMixed,
    Draw3D,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MapTextureMode {
    Preview,
    Floor,
    Wall,
    Ceiling,
}

pub struct MapEditor {
    curr_tile_uuid: Option<Uuid>,

    curr_layer_role: Layer2DRole,
    texture_mode: MapTextureMode,

    icon_normal_border_color: RGBA,
    icon_selected_border_color: RGBA,
}

#[allow(clippy::new_without_default)]
impl MapEditor {
    pub fn new() -> Self {
        Self {
            curr_tile_uuid: None,

            curr_layer_role: Layer2DRole::Ground,
            texture_mode: MapTextureMode::Floor,

            icon_normal_border_color: [100, 100, 100, 255],
            icon_selected_border_color: [255, 255, 255, 255],
        }
    }

    pub fn init_ui(
        &mut self,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _project: &mut Project,
    ) -> TheCanvas {
        let mut center = TheCanvas::new();

        // let mut shared_layout = TheSharedHLayout::new(TheId::named("Editor Shared"));

        // let mut region_editor = TheRGBALayout::new(TheId::named("Region Editor"));
        // if let Some(rgba_view) = region_editor.rgba_view_mut().as_rgba_view() {
        //     rgba_view.set_mode(TheRGBAViewMode::Display);

        //     if let Some(buffer) = ctx.ui.icon("eldiron_map") {
        //         rgba_view.set_buffer(buffer.clone());
        //     }

        //     rgba_view.set_grid_color([255, 255, 255, 5]);
        //     rgba_view.set_hover_color(Some([255, 255, 255, 100]));
        //     rgba_view.set_wheel_scale(-0.2);
        // }

        // let mut region_editor_canvas = TheCanvas::new();
        // region_editor_canvas.set_layout(region_editor);
        // shared_layout.add_canvas(region_editor_canvas);

        // let mut render_canvas: TheCanvas = TheCanvas::new();
        // let render_view = TheRenderView::new(TheId::named("RenderView"));
        // render_canvas.set_widget(render_view);
        // shared_layout.add_canvas(render_canvas);

        //center.set_layout(shared_layout);

        let mut poly_canvas: TheCanvas = TheCanvas::new();
        let render_view = TheRenderView::new(TheId::named("PolyView"));
        poly_canvas.set_widget(render_view);

        center.set_center(poly_canvas);

        // Picker

        let mut tile_picker = TheCanvas::new();
        let mut vlayout = TheVLayout::new(TheId::named("Editor Icon Layout"));
        vlayout.set_background_color(Some(TheThemeColors::ListLayoutBackground));
        vlayout.limiter_mut().set_max_width(90);
        vlayout.set_margin(vec4i(0, 10, 0, 5));

        let mut icon_preview = TheIconView::new(TheId::named("Icon Preview"));
        icon_preview.set_alpha_mode(false);
        icon_preview.limiter_mut().set_max_size(vec2i(65, 65));
        icon_preview.set_border_color(Some([100, 100, 100, 255]));
        vlayout.add_widget(Box::new(icon_preview));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(2);
        vlayout.add_widget(Box::new(spacer));

        let mut view_mode_gb = TheGroupButton::new(TheId::named("Map Editor Camera"));
        view_mode_gb.add_text_status_icon(
            "".to_string(),
            "2D Camera".to_string(),
            "square".to_string(),
        );
        view_mode_gb.add_text_status_icon(
            "".to_string(),
            "3D Camera: Iso".to_string(),
            "cube".to_string(),
        );
        view_mode_gb.add_text_status_icon(
            "".to_string(),
            "3D Camera: First person".to_string(),
            "camera".to_string(),
        );
        view_mode_gb.set_item_width(26);
        vlayout.add_widget(Box::new(view_mode_gb));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(0);
        vlayout.add_widget(Box::new(spacer));

        let mut grid_sub_div = TheTextLineEdit::new(TheId::named("Grid Subdiv"));
        grid_sub_div.set_value(TheValue::Float(1.0));
        // opacity.set_default_value(TheValue::Float(1.0));
        grid_sub_div.set_info_text(Some("Subdiv".to_string()));
        grid_sub_div.set_range(TheValue::RangeI32(1..=10));
        grid_sub_div.set_continuous(true);
        grid_sub_div.limiter_mut().set_max_width(150);
        grid_sub_div.set_status_text("The subdivision level of the grid.");
        grid_sub_div.limiter_mut().set_max_width(75);
        vlayout.add_widget(Box::new(grid_sub_div));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(2);
        vlayout.add_widget(Box::new(spacer));

        let mut ground_icon = TheIconView::new(TheId::named("Ground Icon"));
        ground_icon.set_text(Some("FLOOR".to_string()));
        ground_icon.set_text_size(10.0);
        ground_icon.set_text_color([200, 200, 200, 255]);
        ground_icon.limiter_mut().set_max_size(vec2i(48, 48));
        ground_icon.set_border_color(Some(self.icon_selected_border_color));

        let mut wall_icon = TheIconView::new(TheId::named("Wall Icon"));
        wall_icon.set_text(Some("WALL".to_string()));
        wall_icon.set_text_size(10.0);
        wall_icon.set_text_color([200, 200, 200, 255]);
        wall_icon.limiter_mut().set_max_size(vec2i(48, 48));
        wall_icon.set_border_color(Some(self.icon_normal_border_color));

        let mut ceiling_icon = TheIconView::new(TheId::named("Ceiling Icon"));
        ceiling_icon.set_text(Some("CEILING".to_string()));
        ceiling_icon.set_text_size(10.0);
        ceiling_icon.set_text_color([200, 200, 200, 255]);
        ceiling_icon.limiter_mut().set_max_size(vec2i(48, 48));
        ceiling_icon.set_border_color(Some(self.icon_normal_border_color));

        // let mut cc_icon = TheIconView::new(TheId::named("Tile FX Icon"));
        // cc_icon.set_text(Some("FX".to_string()));
        // cc_icon.set_text_size(10.0);
        // cc_icon.set_text_color([200, 200, 200, 255]);
        // cc_icon.limiter_mut().set_max_size(vec2i(48, 48));
        // cc_icon.set_border_color(Some(self.icon_normal_border_color));

        vlayout.add_widget(Box::new(ground_icon));
        vlayout.add_widget(Box::new(wall_icon));
        vlayout.add_widget(Box::new(ceiling_icon));
        //vlayout.add_widget(Box::new(cc_icon));

        let mut spacer = TheIconView::new(TheId::empty());
        spacer.limiter_mut().set_max_height(2);
        vlayout.add_widget(Box::new(spacer));

        let mut text = TheText::new(TheId::named("Cursor Position"));
        text.set_text("()".to_string());
        text.set_text_color([200, 200, 200, 255]);
        vlayout.add_widget(Box::new(text));

        // let mut text = TheText::new(TheId::named("Cursor Height"));
        // text.set_text("H: -".to_string());
        // text.set_text_color([200, 200, 200, 255]);
        // vlayout.add_widget(Box::new(text));

        tile_picker.set_layout(vlayout);
        center.set_left(tile_picker);

        // Tool Params
        let mut toolbar_hlayout = TheHLayout::new(TheId::named("Game Tool Params"));
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(vec4i(10, 2, 5, 2));

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        toolbar_canvas.set_layout(toolbar_hlayout);

        center.set_top(toolbar_canvas);

        center
    }

    pub fn load_from_project(&mut self, _ui: &mut TheUI, _ctx: &mut TheContext, project: &Project) {
        TILEDRAWER
            .lock()
            .unwrap()
            .set_tiles(project.extract_tiles());
        MAPRENDER
            .lock()
            .unwrap()
            .set_textures(project.extract_tiles());
    }

    #[allow(clippy::suspicious_else_formatting)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        match event {
            TheEvent::Custom(id, value) => {
                if id.name == "Map Selection Changed" {
                    let mut floor_icon_id: Option<Uuid> = None;
                    //let mut wall_icon_id: Option<Uuid> = None;
                    //let mut ceiling_icon_id: Option<Uuid> = None;

                    if server_ctx.curr_map_tool_type == MapToolType::Sector {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            if region.map.selected_sectors.len() == 1 {
                                if let Some(sector) =
                                    region.map.find_sector(region.map.selected_sectors[0])
                                {
                                    if let Some(tile_id) = sector.floor_texture {
                                        floor_icon_id = Some(tile_id);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
                        if let Some(floor_icon_id) = floor_icon_id {
                            if let Some(tile) = TILEDRAWER.lock().unwrap().tiles.get(&floor_icon_id)
                            {
                                icon_view.set_rgba_tile(tile.clone());
                            }
                        } else {
                            let buffer = TheRGBABuffer::new(TheDim::sized(48, 48));
                            if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
                                icon_view.set_rgba_tile(TheRGBATile::buffer(buffer));
                            }
                        }
                    }
                } else if id.name == "Cursor Pos Changed" {
                    if let Some(text) = ui.get_text("Cursor Position") {
                        if let Some(v) = value.to_vec2f() {
                            text.set_text(format!("{}, {}", v.x, v.y));
                        }
                        redraw = true;
                    }

                    if let Some(layout) = ui.get_layout("Editor Icon Layout") {
                        layout.relayout(ctx);
                    }
                }
            }
            TheEvent::RenderViewScrollBy(id, coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if ui.ctrl || ui.logo {
                            region.map.grid_size += coord.y as f32;
                            region.map.grid_size = clamp(region.map.grid_size, 5.0, 100.0);
                        } else {
                            region.map.offset += Vec2f::new(-coord.x as f32, coord.y as f32);
                        }
                        region.editing_position_3d.x += coord.x as f32 / region.map.grid_size;
                        region.editing_position_3d.z += coord.y as f32 / region.map.grid_size;
                        server.update_region(region);
                        redraw = true;
                    }
                }
            }
            /*
            TheEvent::RenderViewScrollBy(id, amount) => {
                if id.name == "RenderView" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        region.editing_position_3d.x += amount.x as f32 / region.grid_size as f32;
                        region.editing_position_3d.z += amount.y as f32 / region.grid_size as f32;
                        server.set_editing_position_3d(region.editing_position_3d);
                        redraw = true;
                    }
                }
            }*/
            /*
            TheEvent::RenderViewLostHover(id) => {
                if id.name == "RenderView" {
                    RENDERER.lock().unwrap().hover_pos = None;
                }
            }
            TheEvent::RenderViewHoverChanged(id, coord) => {
                if id.name == "RenderView" {
                    if let Some(render_view) = ui.get_render_view("RenderView") {
                        let dim = render_view.dim();
                        let palette = project.palette.clone();
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let pos = RENDERER.lock().unwrap().get_hit_position_at(
                                *coord,
                                region,
                                &mut server.get_instance_draw_settings(server_ctx.curr_region),
                                dim.width as usize,
                                dim.height as usize,
                            );
                            if let Some((pos, _)) = pos {
                                RENDERER.lock().unwrap().hover_pos = Some(pos);

                                if let Some(text) = ui.get_text("Cursor Position") {
                                    text.set_text(format!("({}, {})", pos.x, pos.z));
                                    redraw = true;
                                }

                                if let Some(text) = ui.get_text("Cursor Height") {
                                    let h = region.heightmap.get_height(pos.x as f32, pos.z as f32);
                                    text.set_text(format!("H: {:.3}", h));
                                    redraw = true;
                                }

                                if let Some(layout) = ui.get_layout("Editor Icon Layout") {
                                    layout.relayout(ctx);
                                }

                                self.set_icon_previews(
                                    region,
                                    &palette,
                                    vec2i(pos.x, pos.z),
                                    ui,
                                    ctx,
                                );
                            }
                        }
                    }
                }
            }*/
            // TheEvent::RenderViewClicked(id, coord) => {
            //     if id.name == "RenderView" {
            //         self.processed_coords.clear();
            //         if let Some(render_view) = ui.get_render_view("RenderView") {
            //             let dim = render_view.dim();
            //             if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            //                 let pos = RENDERER.lock().unwrap().get_hit_position_at(
            //                     *coord,
            //                     region,
            //                     &mut server.get_instance_draw_settings(server_ctx.curr_region),
            //                     dim.width as usize,
            //                     dim.height as usize,
            //                 );

            //                 if let Some(pos) = pos {
            //                     redraw = self.action_at(
            //                         vec2i(pos.x, pos.z),
            //                         ui,
            //                         ctx,
            //                         project,
            //                         server,
            //                         server_ctx,
            //                         true,
            //                     );
            //                 }
            //             }
            //         }
            //     }
            // }
            // TheEvent::RenderViewDragged(id, coord) => {
            //     if id.name == "RenderView" {
            //         if let Some(render_view) = ui.get_render_view("RenderView") {
            //             let dim = render_view.dim();
            //             if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
            //                 let pos = RENDERER.lock().unwrap().get_hit_position_at(
            //                     *coord,
            //                     region,
            //                     &mut server.get_instance_draw_settings(server_ctx.curr_region),
            //                     dim.width as usize,
            //                     dim.height as usize,
            //                 );

            //                 if let Some(pos) = pos {
            //                     redraw = self.action_at(
            //                         vec2i(pos.x, pos.z),
            //                         ui,
            //                         ctx,
            //                         project,
            //                         server,
            //                         server_ctx,
            //                         true,
            //                     );
            //                 }
            //             }
            //         }
            //     }
            // }
            // TheEvent::TileEditorClicked(id, coord) => {
            //     if id.name == "Region Editor View" {
            //         self.processed_coords.clear();
            //         redraw = self.action_at(*coord, ui, ctx, project, server, server_ctx, false);
            //     }
            // }
            // TheEvent::TileEditorDragged(id, coord) => {
            //     if id.name == "Region Editor View" {
            //         redraw = self.action_at(*coord, ui, ctx, project, server, server_ctx, false);
            //     }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Map Editor Camera" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if *index == 0 {
                            region.map.camera = MapCamera::TwoD;
                        } else if *index == 1 {
                            region.map.camera = MapCamera::ThreeDIso;
                        } else if *index == 2 {
                            region.map.camera = MapCamera::ThreeDFirstPerson;
                        }
                        server.update_region(region);
                    }
                } /*else if id.name == "2D3D Group" {
                      if let Some(shared) = ui.get_sharedhlayout("Editor Shared") {
                          if *index == 0 {
                              project.map_mode = MapMode::TwoD;
                              shared.set_mode(TheSharedHLayoutMode::Left);
                              *RENDERMODE.lock().unwrap() = EditorDrawMode::Draw2D;
                              PRERENDERTHREAD.lock().unwrap().set_paused(true);
                              if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                  if let Some(layout) = ui.get_rgba_layout("Region Editor") {
                                      layout.set_zoom(region.zoom);
                                      layout.relayout(ctx);
                                  }
                              }
                          } else if *index == 1 {
                              project.map_mode = MapMode::Mixed;
                              shared.set_mode(TheSharedHLayoutMode::Shared);
                              *RENDERMODE.lock().unwrap() = EditorDrawMode::DrawMixed;
                              PRERENDERTHREAD.lock().unwrap().set_paused(false);
                          } else if *index == 2 {
                              project.map_mode = MapMode::ThreeD;
                              shared.set_mode(TheSharedHLayoutMode::Right);
                              *RENDERMODE.lock().unwrap() = EditorDrawMode::Draw3D;
                              PRERENDERTHREAD.lock().unwrap().set_paused(false);
                          }
                          ctx.ui.relayout = true;

                          // Set the region and textures to the RenderView if visible
                          if *index > 0 {
                              if let Some(region) = project.get_region(&server_ctx.curr_region) {
                                  RENDERER.lock().unwrap().set_region(region);
                                  RENDERER
                                      .lock()
                                      .unwrap()
                                      .set_textures(project.extract_tiles());
                              }
                          }
                      }
                  }*/
            }
            // else if id.name == "Editor Group" {
            //         server_ctx.conceptual_display = None;
            //         if *index == EditorMode::Draw as usize {
            //             // self.editor_mode = EditorMode::Draw;
            //             server_ctx.tile_selection = None;

            //             // Set the 3D editing position to selected character position
            //             // before voiding it. Otherwise the 3D view will just jump to an empty region.
            //             if let Some(character_instance_id) = server_ctx.curr_character_instance {
            //                 if let Some((TheValue::Position(p), _)) = server.get_character_property(
            //                     server_ctx.curr_region,
            //                     character_instance_id,
            //                     "position".into(),
            //                 ) {
            //                     if let Some(region) =
            //                         project.get_region_mut(&server_ctx.curr_region)
            //                     {
            //                         region.editing_position_3d = p;
            //                         server.set_editing_position_3d(region.editing_position_3d);
            //                     }
            //                 }
            //             }

            //             // Set the icon for the current brush
            //             if let Some(id) = self.curr_tile_uuid {
            //                 if let Some(t) = TILEDRAWER.lock().unwrap().tiles.get(&id) {
            //                     if let Some(icon_view) = ui.get_icon_view("Icon Preview") {
            //                         icon_view.set_rgba_tile(t.clone());
            //                     }
            //                 }
            //             }

            //             // if self.curr_layer_role == Layer2DRole::FX {
            //             //     ctx.ui
            //             //         .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 3));
            //             // } else {
            //             //     ctx.ui
            //             //         .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));
            //             // }

            //             if let Some(layout) = ui.get_sharedhlayout("Shared Panel Layout") {
            //                 layout.set_mode(TheSharedHLayoutMode::Right);
            //                 ctx.ui.relayout = true;
            //                 redraw = true;
            //             }

            //             server_ctx.curr_character_instance = None;
            //             server_ctx.curr_item_instance = None;
            //             server_ctx.curr_area = None;
            //         } else if *index == EditorMode::Pick as usize {
            //             // self.editor_mode = EditorMode::Pick;
            //             server_ctx.tile_selection = None;
            //         } else if *index == EditorMode::Erase as usize {
            //             // self.editor_mode = EditorMode::Erase;
            //             server_ctx.tile_selection = None;
            //         } else if *index == EditorMode::Select as usize {
            //             ui.set_widget_context_menu(
            //                 "Region Editor View",
            //                 Some(TheContextMenu {
            //                     items: vec![TheContextMenuItem::new(
            //                         "Create Area...".to_string(),
            //                         TheId::named("Create Area"),
            //                     )],
            //                     ..Default::default()
            //                 }),
            //             );
            //             // self.editor_mode = EditorMode::Select;
            //         }

            //         if *index == EditorMode::Code as usize {
            //             // self.editor_mode = EditorMode::Code;
            //             server_ctx.tile_selection = None;
            //             ctx.ui.send(TheEvent::Custom(
            //                 TheId::named("Set CodeGrid Panel"),
            //                 TheValue::Empty,
            //             ));
            //         } else if *index == EditorMode::Model as usize {
            //             // self.editor_mode = EditorMode::Model;
            //             server_ctx.tile_selection = None;
            //             ctx.ui.send(TheEvent::Custom(
            //                 TheId::named("Set Region Modeler"),
            //                 TheValue::Empty,
            //             ));
            //             if let Some(TheValue::Float(f)) = ui.get_widget_value("ModelFX Blend") {
            //                 server_ctx.conceptual_display = Some(f);
            //             }
            //         } else if *index == EditorMode::Tilemap as usize {
            //             // self.editor_mode = EditorMode::Tilemap;
            //             server_ctx.tile_selection = None;
            //             ctx.ui.send(TheEvent::Custom(
            //                 TheId::named("Set Tilemap Panel"),
            //                 TheValue::Empty,
            //             ));
            //         } else if *index == EditorMode::Render as usize {
            //             // self.editor_mode = EditorMode::Render;
            //             server_ctx.tile_selection = None;
            //             ctx.ui.send(TheEvent::Custom(
            //                 TheId::named("Set Region Render"),
            //                 TheValue::Empty,
            //             ));
            //         }
            //     }
            // }
            // TheEvent::TileEditorUp(_id) => {
            //     if self.editor_mode == EditorMode::Select {
            //         if let Some(tilearea) = &mut server_ctx.tile_selection {
            //             tilearea.ongoing = false;
            //         }
            //     }
            // }
            TheEvent::TileEditorHoverChanged(id, coord) => {
                if id.name == "Region Editor View" {
                    if let Some(text) = ui.get_text("Cursor Position") {
                        text.set_text(format!("({}, {})", coord.x, coord.y));
                        redraw = true;
                    }

                    if let Some(text) = ui.get_text("Cursor Height") {
                        if let Some(region) = project.get_region(&server_ctx.curr_region) {
                            let h = region.heightmap.get_height(coord.x as f32, coord.y as f32);
                            text.set_text(format!("H: {:.3}", h));
                        }
                        redraw = true;
                    }

                    if let Some(layout) = ui.get_layout("Editor Icon Layout") {
                        layout.relayout(ctx);
                    }

                    for r in &mut project.regions {
                        if r.id == server_ctx.curr_region {
                            self.set_icon_previews(r, &project.palette, *coord, ui, ctx);
                            break;
                        }
                    }
                }
            }
            TheEvent::GainedFocus(id) => {
                if id.name == "Region Editor View" || id.name == "RenderView" {
                    UNDOMANAGER.lock().unwrap().context = UndoManagerContext::Region;
                } else if id.name == "ModelFX RGBA Layout View" {
                    UNDOMANAGER.lock().unwrap().context = UndoManagerContext::MaterialFX;
                } else if id.name == "Palette Picker" {
                    UNDOMANAGER.lock().unwrap().context = UndoManagerContext::Palette;
                }
            }
            TheEvent::ValueChanged(id, value) => {
                if id.name == "Grid Subdiv" {
                    if let Some(value) = value.to_i32() {
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.map.subdivisions = value as f32;
                            server.update_region(region);
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, _state) => {
                // Region Content List Selection
                if id.name == "Region Content List Item" {
                    if let Some((TheValue::Position(p), character_id)) = server
                        .get_character_property(server_ctx.curr_region, id.uuid, "position".into())
                    {
                        // If it's a character instance, center it in the region editor.
                        server_ctx.curr_character_instance = Some(id.uuid);
                        server_ctx.curr_character = Some(character_id);
                        server_ctx.curr_item_instance = None;
                        server_ctx.curr_item = None;
                        server_ctx.curr_area = None;

                        // self.editor_mode = EditorMode::Pick;
                        // if let Some(button) = ui.get_group_button("Editor Group") {
                        //     button.set_index(EditorMode::Pick as i32);
                        //     ctx.ui.send(TheEvent::IndexChanged(
                        //         button.id().clone(),
                        //         EditorMode::Pick as usize,
                        //     ));
                        // }

                        // Set 3D editing position to Zero.
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.editing_position_3d = Vec3f::zero();
                            server.set_editing_position_3d(region.editing_position_3d);
                        }

                        // Set the character codegrid of the current character instance.
                        if let Some(region) = project.get_region(&server_ctx.curr_region) {
                            if let Some(character) = region.characters.get(&id.uuid) {
                                for grid in character.instance.grids.values() {
                                    if grid.name == "init" {
                                        CODEEDITOR.lock().unwrap().set_codegrid(grid.clone(), ui);
                                        CODEEDITOR.lock().unwrap().code_id =
                                            str!("Character Instance");

                                        // ctx.ui.send(TheEvent::Custom(
                                        //     TheId::named("Set CodeGrid Panel"),
                                        //     TheValue::Empty,
                                        // ));
                                        // self.set_editor_group_index(EditorMode::Code, ui, ctx)
                                    }
                                }
                            }
                        }

                        if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                            rgba_layout.scroll_to_grid(vec2i(p.x as i32, p.z as i32));
                            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                                region.scroll_offset = vec2i(
                                    p.x as i32 * region.grid_size,
                                    p.z as i32 * region.grid_size,
                                );
                            }
                        }
                    }
                    if let Some((TheValue::Position(p), item_id)) =
                        server.get_item_property(server_ctx.curr_region, id.uuid, "position".into())
                    {
                        // If it's an item instance, center it in the item editor.
                        server_ctx.curr_character_instance = None;
                        server_ctx.curr_character = None;
                        server_ctx.curr_item_instance = Some(id.uuid);
                        server_ctx.curr_item = Some(item_id);
                        server_ctx.curr_area = None;

                        // Set the item codegrid of the current item instance.
                        if let Some(region) = project.get_region(&server_ctx.curr_region) {
                            if let Some(character) = region.items.get(&id.uuid) {
                                for grid in character.instance.grids.values() {
                                    if grid.name == "init" {
                                        CODEEDITOR.lock().unwrap().set_codegrid(grid.clone(), ui);
                                        CODEEDITOR.lock().unwrap().code_id = str!("Item Instance");
                                        // ctx.ui.send(TheEvent::Custom(
                                        //     TheId::named("Set CodeGrid Panel"),
                                        //     TheValue::Empty,
                                        // ));
                                        // self.set_editor_group_index(EditorMode::Code, ui, ctx)
                                    }
                                }
                            }
                        }

                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            region.editing_position_3d = vec3f(p.x, 0.0, p.z);
                            server.set_editing_position_3d(region.editing_position_3d);
                            if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                                rgba_layout.scroll_to_grid(vec2i(p.x as i32, p.z as i32));
                                region.scroll_offset = vec2i(
                                    p.x as i32 * region.grid_size,
                                    p.z as i32 * region.grid_size,
                                );
                            }
                        }
                    } else if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if let Some(area) = region.areas.get(&id.uuid) {
                            server_ctx.curr_character_instance = None;
                            server_ctx.curr_character = None;
                            server_ctx.curr_item_instance = None;
                            server_ctx.curr_item = None;
                            server_ctx.curr_area = Some(area.id);

                            for grid in area.bundle.grids.values() {
                                if grid.name == "main" {
                                    CODEEDITOR.lock().unwrap().set_codegrid(grid.clone(), ui);
                                    CODEEDITOR.lock().unwrap().code_id = str!("Area Instance");
                                    // ctx.ui.send(TheEvent::Custom(
                                    //     TheId::named("Set CodeGrid Panel"),
                                    //     TheValue::Empty,
                                    // ));
                                    // self.set_editor_group_index(EditorMode::Code, ui, ctx)
                                }
                            }

                            // Add the area to the server
                            // ? server.insert_area(region.id, area.clone());

                            if let Some(p) = area.center() {
                                region.editing_position_3d = vec3f(p.0 as f32, 0.0, p.1 as f32);
                                server.set_editing_position_3d(region.editing_position_3d);
                                if let Some(rgba_layout) = ui.get_rgba_layout("Region Editor") {
                                    rgba_layout.scroll_to_grid(vec2i(p.0, p.1));
                                    region.scroll_offset =
                                        vec2i(p.0 * region.grid_size, p.1 * region.grid_size);
                                }
                            }
                        }
                    }
                }
                // Region Selection
                else if id.name == "Region Item" {
                    for r in &project.regions {
                        if r.id == id.uuid {
                            if let Some(rgba_layout) =
                                ui.canvas.get_layout(Some(&"Region Editor".into()), None)
                            {
                                if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                                    if let Some(rgba_view) =
                                        rgba_layout.rgba_view_mut().as_rgba_view()
                                    {
                                        rgba_view.set_mode(TheRGBAViewMode::TileEditor);
                                        let width = r.width * r.grid_size;
                                        let height = r.height * r.grid_size;
                                        let buffer =
                                            TheRGBABuffer::new(TheDim::new(0, 0, width, height));
                                        rgba_view.set_buffer(buffer);
                                        rgba_view.set_grid(Some(r.grid_size));
                                        ctx.ui.relayout = true;
                                    }
                                    rgba_layout.scroll_to(r.scroll_offset);
                                }
                            }

                            server_ctx.curr_region = r.id;
                            //self.redraw_region(ui, server, ctx, server_ctx);
                            redraw = true;
                        }
                    }
                }
                // An item in the tile list was selected
                else if id.name == "Tilemap Tile" {
                    self.curr_tile_uuid = Some(id.uuid);
                    server_ctx.curr_tile_id = Some(id.uuid);

                    if let Some(t) = TILEDRAWER.lock().unwrap().tiles.get(&id.uuid) {
                        if let Some(icon_view) = ui.get_icon_view("Icon Preview") {
                            icon_view.set_rgba_tile(t.clone());
                        }
                    }

                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        let prev = region.map.clone();

                        // Apply to the selected map elements
                        if self.texture_mode == MapTextureMode::Floor {
                            for sector_id in &region.map.selected_sectors.clone() {
                                if let Some(sector) = region.map.find_sector_mut(*sector_id) {
                                    sector.floor_texture = self.curr_tile_uuid;
                                }
                            }
                        } else if self.texture_mode == MapTextureMode::Wall {
                            let mut linedef_ids = Vec::new();
                            for sector_id in &region.map.selected_sectors {
                                if let Some(sector) = region.map.find_sector(*sector_id) {
                                    linedef_ids.extend(&sector.linedefs);
                                }
                            }

                            for linedef_id in linedef_ids {
                                if let Some(linedef) = region.map.find_linedef_mut(linedef_id) {
                                    linedef.texture = self.curr_tile_uuid;
                                }
                            }
                        }

                        let undo =
                            RegionUndoAtom::MapEdit(Box::new(prev), Box::new(region.map.clone()));

                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region.id, undo, ctx);
                        server.update_region(region);

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Minimap"),
                            TheValue::Empty,
                        ));
                    }
                } else if id.name == "Tilemap Editor Add Anim"
                    || id.name == "Tilemap Editor Add Multi"
                {
                    TILEDRAWER.lock().unwrap().tiles = project.extract_tiles();
                    server.update_tiles(project.extract_tiles());
                } else if id.name == "Ground Icon" {
                    self.curr_layer_role = Layer2DRole::Ground;
                    self.texture_mode = MapTextureMode::Floor;
                    server_ctx.curr_layer_role = Layer2DRole::Ground;

                    self.set_icon_border_colors(ui);
                    server_ctx.show_fx_marker = false;
                    redraw = true;
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Floor Selected"),
                        TheValue::Empty,
                    ));
                } else if id.name == "Wall Icon" {
                    self.curr_layer_role = Layer2DRole::Wall;
                    self.texture_mode = MapTextureMode::Wall;
                    server_ctx.curr_layer_role = Layer2DRole::Wall;

                    self.set_icon_border_colors(ui);
                    server_ctx.show_fx_marker = false;
                    redraw = true;
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Wall Selected"),
                        TheValue::Empty,
                    ));
                    // if self.editor_mode == EditorMode::Draw {
                    //     ctx.ui
                    //         .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));
                    // }
                } else if id.name == "Ceiling Icon" {
                    self.curr_layer_role = Layer2DRole::Ceiling;
                    self.texture_mode = MapTextureMode::Ceiling;
                    server_ctx.curr_layer_role = Layer2DRole::Ceiling;

                    self.set_icon_border_colors(ui);
                    server_ctx.show_fx_marker = false;
                    redraw = true;
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Ceiling Selected"),
                        TheValue::Empty,
                    ));
                    // if self.editor_mode == EditorMode::Draw {
                    //     ctx.ui
                    //         .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 0));
                    // }
                } else if id.name == "Tile FX Icon" {
                    self.curr_layer_role = Layer2DRole::FX;
                    self.set_icon_border_colors(ui);
                    server_ctx.show_fx_marker = true;
                    redraw = true;
                    // if self.editor_mode == EditorMode::Draw {
                    //     ctx.ui
                    //         .send(TheEvent::SetStackIndex(TheId::named("Main Stack"), 3));
                    // }
                }
            }
            _ => {}
        }
        redraw
    }

    fn set_icon_previews(
        &mut self,
        region: &mut Region,
        palette: &ThePalette,
        coord: Vec2i,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let mut found_ground_icon = false;
        let mut found_wall_icon = false;
        let mut found_ceiling_icon = false;

        let tile_coord = vec2f(coord.x as f32, coord.y as f32);

        if let Some(geo_ids) = region.geometry_areas.get(&vec3i(coord.x, 0, coord.y)) {
            for geo_id in geo_ids {
                if let Some(geo_obj) = region.geometry.get(geo_id) {
                    for node in &geo_obj.nodes {
                        let tiledrawer = TILEDRAWER.lock().unwrap();
                        if node.get_layer_role() == Layer2DRole::Ground && !found_ground_icon {
                            let mut buffer = TheRGBABuffer::new(TheDim::sized(48, 48));
                            if let Some(material) = tiledrawer.materials.get(&geo_obj.material_id) {
                                node.preview(
                                    &mut buffer,
                                    Some(material),
                                    palette,
                                    &tiledrawer.tiles,
                                    tile_coord,
                                    ctx,
                                );
                            } else {
                                node.preview(
                                    &mut buffer,
                                    None,
                                    palette,
                                    &FxHashMap::default(),
                                    tile_coord,
                                    ctx,
                                );
                            }
                            if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
                                icon_view.set_rgba_tile(TheRGBATile::buffer(buffer));
                                found_ground_icon = true;
                            }
                        } else if node.get_layer_role() == Layer2DRole::Wall && !found_wall_icon {
                            let mut buffer = TheRGBABuffer::new(TheDim::sized(48, 48));
                            if let Some(material) = tiledrawer.materials.get(&geo_obj.material_id) {
                                node.preview(
                                    &mut buffer,
                                    Some(material),
                                    palette,
                                    &tiledrawer.tiles,
                                    tile_coord,
                                    ctx,
                                );
                            } else {
                                node.preview(
                                    &mut buffer,
                                    None,
                                    palette,
                                    &FxHashMap::default(),
                                    tile_coord,
                                    ctx,
                                );
                            }
                            if let Some(icon_view) = ui.get_icon_view("Wall Icon") {
                                icon_view.set_rgba_tile(TheRGBATile::buffer(buffer));
                                found_wall_icon = true;
                            }
                        } else if node.get_layer_role() == Layer2DRole::Ceiling
                            && !found_ceiling_icon
                        {
                            let mut buffer = TheRGBABuffer::new(TheDim::sized(48, 48));
                            if let Some(material) = tiledrawer.materials.get(&geo_obj.material_id) {
                                node.preview(
                                    &mut buffer,
                                    Some(material),
                                    palette,
                                    &tiledrawer.tiles,
                                    tile_coord,
                                    ctx,
                                );
                            } else {
                                node.preview(
                                    &mut buffer,
                                    None,
                                    palette,
                                    &FxHashMap::default(),
                                    tile_coord,
                                    ctx,
                                );
                            }
                            if let Some(icon_view) = ui.get_icon_view("Ceiling Icon") {
                                icon_view.set_rgba_tile(TheRGBATile::buffer(buffer));
                                found_ceiling_icon = true;
                            }
                        }
                    }
                }
            }
        }

        if let Some(tile) = region.tiles.get(&(coord.x, coord.y)) {
            // Ground

            if !found_ground_icon {
                if let Some(ground) = tile.layers[0] {
                    if let Some(tile) = TILEDRAWER.lock().unwrap().tiles.get(&ground) {
                        if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
                            icon_view.set_rgba_tile(tile.clone());
                            found_ground_icon = true;
                        }
                    }
                }
            }

            // Wall
            if !found_wall_icon {
                if let Some(wall) = tile.layers[1] {
                    if let Some(tile) = TILEDRAWER.lock().unwrap().tiles.get(&wall) {
                        if let Some(icon_view) = ui.get_icon_view("Wall Icon") {
                            icon_view.set_rgba_tile(tile.clone());
                            found_wall_icon = true;
                        }
                    }
                }
            }

            // Ceiling
            if !found_ceiling_icon {
                if let Some(ceiling) = tile.layers[2] {
                    if let Some(tile) = TILEDRAWER.lock().unwrap().tiles.get(&ceiling) {
                        if let Some(icon_view) = ui.get_icon_view("Ceiling Icon") {
                            icon_view.set_rgba_tile(tile.clone());
                            found_ceiling_icon = true;
                        }
                    }
                }
            }
        }

        if !found_ground_icon {
            if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
                icon_view.set_rgba_tile(TheRGBATile::default());
            }
        }

        if !found_wall_icon {
            if let Some(icon_view) = ui.get_icon_view("Wall Icon") {
                icon_view.set_rgba_tile(TheRGBATile::default());
            }
        }

        if !found_ceiling_icon {
            if let Some(icon_view) = ui.get_icon_view("Ceiling Icon") {
                icon_view.set_rgba_tile(TheRGBATile::default());
            }
        }
    }

    fn set_icon_border_colors(&mut self, ui: &mut TheUI) {
        if let Some(icon_view) = ui.get_icon_view("Ground Icon") {
            icon_view.set_border_color(if self.curr_layer_role == Layer2DRole::Ground {
                Some(self.icon_selected_border_color)
            } else {
                Some(self.icon_normal_border_color)
            });
        }
        if let Some(icon_view) = ui.get_icon_view("Wall Icon") {
            icon_view.set_border_color(if self.curr_layer_role == Layer2DRole::Wall {
                Some(self.icon_selected_border_color)
            } else {
                Some(self.icon_normal_border_color)
            });
        }
        if let Some(icon_view) = ui.get_icon_view("Ceiling Icon") {
            icon_view.set_border_color(if self.curr_layer_role == Layer2DRole::Ceiling {
                Some(self.icon_selected_border_color)
            } else {
                Some(self.icon_normal_border_color)
            });
        }
    }

    /// Redraw the map of the current region on tick.
    pub fn redraw_region(
        &mut self,
        project: &Project,
        ui: &mut TheUI,
        server: &mut Server,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
        compute_delta: bool,
    ) {
        // Redraw complete region
        // if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Region Editor".into()), None) {
        //     if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
        //         if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
        //             server.draw_region(
        //                 &server_ctx.curr_region,
        //                 rgba_view.buffer_mut(),
        //                 &TILEDRAWER.lock().unwrap(),
        //                 server_ctx,
        //                 compute_delta,
        //                 vec2i(0, 0),
        //             );
        //             rgba_view.set_needs_redraw(true);
        //         }
        //     }
        // }

        // Redraw partial region
        // if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Region Editor".into()), None) {
        //     if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
        //         if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
        //             let rect = rgba_view.visible_rect();
        //             let dest_dim = rgba_view.buffer().dim();

        //             if rect.x + rect.width < dest_dim.width
        //                 && rect.y + rect.height < dest_dim.height
        //             {
        //                 let mut b = TheRGBABuffer::new(rect);

        //                 server.draw_region(
        //                     &server_ctx.curr_region,
        //                     &mut b,
        //                     &TILEDRAWER.lock().unwrap(),
        //                     server_ctx,
        //                     compute_delta,
        //                     vec2i(rect.x, dest_dim.height - (rect.y + rect.height)),
        //                 );
        //                 rgba_view.buffer_mut().copy_into(rect.x, rect.y, &b);
        //                 server.draw_region_selections(
        //                     &server_ctx.curr_region,
        //                     rgba_view.buffer_mut(),
        //                     &TILEDRAWER.lock().unwrap(),
        //                     ctx,
        //                     server_ctx,
        //                 );
        //                 rgba_view.set_needs_redraw(true);
        //             }
        //         }
        //     }
        // }
        if let Some(rgba_layout) = ui.canvas.get_layout(Some(&"Region Editor".into()), None) {
            if let Some(rgba_layout) = rgba_layout.as_rgba_layout() {
                if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
                    let mut rect = rgba_view.visible_rect();
                    let dest_dim = rgba_view.buffer().dim();

                    let mut tile_size = 24;

                    if let Some(region) = project.get_region(&server_ctx.curr_region) {
                        tile_size = region.grid_size;
                    }

                    // Adjust the rect boundaries to ensure tiles at the edges are included
                    rect.width = (rect.width + tile_size - 1) / tile_size * tile_size;
                    rect.height = (rect.height + tile_size - 1) / tile_size * tile_size;

                    // Make sure rect dimensions do not exceed the destination buffer size
                    rect.width = rect.width.min(dest_dim.width - rect.x);
                    rect.height = rect.height.min(dest_dim.height - rect.y);

                    // Check if we're still within the bounds
                    if rect.x + rect.width <= dest_dim.width
                        && rect.y + rect.height <= dest_dim.height
                    {
                        let mut b = TheRGBABuffer::new(rect);

                        server.draw_region(
                            &server_ctx.curr_region,
                            &mut b,
                            &TILEDRAWER.lock().unwrap(),
                            server_ctx,
                            compute_delta,
                            vec2i(rect.x, dest_dim.height - (rect.y + rect.height)),
                        );

                        rgba_view.buffer_mut().copy_into(rect.x, rect.y, &b);

                        // Draw selections
                        server.draw_region_selections(
                            &server_ctx.curr_region,
                            rgba_view.buffer_mut(),
                            &TILEDRAWER.lock().unwrap(),
                            ctx,
                            server_ctx,
                        );

                        rgba_view.set_needs_redraw(true);
                    }
                }
            }
        }
    }

    /// Redraw the map of the current region on tick.
    pub fn rerender_region(
        &mut self,
        ui: &mut TheUI,
        server: &mut Server,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
        _project: &Project,
        compute_delta: bool,
    ) {
        if let Some(render_view) = ui.get_render_view("PolyView") {
            let dim = *render_view.dim();

            let b = render_view.render_buffer_mut();
            b.resize(dim.width, dim.height);

            server.render_region(
                &server_ctx.curr_region,
                render_view.render_buffer_mut(),
                &mut MAPRENDER.lock().unwrap(),
                ctx,
                server_ctx,
                compute_delta,
            );

            /*

            if upscale != 1.0 {
                let width = (dim.width as f32 / upscale) as i32;
                let height = (dim.height as f32 / upscale) as i32;

                let b = render_view.render_buffer_mut();
                b.resize(width, height);

                server.render_region(
                    &server_ctx.curr_region,
                    b,
                    &mut RENDERER.lock().unwrap(),
                    ctx,
                    server_ctx,
                    compute_delta,
                );
            }
            */
            /*
            let width = (dim.width as f32 / upscale) as i32;
            let height = (dim.height as f32 / upscale) as i32;

            let b = render_view.render_buffer_mut();
            b.resize(width, height);

            server.render_region(
                &server_ctx.curr_region,
                b,
                &mut RENDERER.lock().unwrap(),
                ctx,
                server_ctx,
                compute_delta,
                );*/
        }
    }

    /*
    /// Perform the given action at the given coordinate.
    #[allow(clippy::too_many_arguments)]
    pub fn action_at(
        &mut self,
        coord: Vec2i,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        server_ctx: &mut ServerContext,
        three_d: bool,
    ) -> bool {
        let mut redraw = false;

        if self.editor_mode == EditorMode::Pick {
            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                region.editing_position_3d = vec3i(coord.x, 0, coord.y).into();
                server.set_editing_position_3d(region.editing_position_3d);
            }
        }

        if self.editor_mode == EditorMode::Model {
            let mut region_to_render: Option<Region> = None;
            let mut tiles_to_render: Vec<Vec2i> = vec![];

            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                let editing_mode = MODELFXEDITOR.lock().unwrap().editing_mode;
                if editing_mode == EditingMode::Geometry {
                    if !self.processed_coords.contains(&coord) {
                        // Add Geometry
                        let geo = MODELFXEDITOR.lock().unwrap().get_geo_node(ui);
                        if let Some(mut geo) = geo {
                            if geo.get_layer_role() == Layer2DRole::Ground {
                                let prev = region.heightmap.clone();
                                // Heightmap editing
                                geo.heightmap_edit(&coord, &mut region.heightmap);
                                self.processed_coords.insert(coord);
                                tiles_to_render.push(coord);
                                region_to_render = Some(region.clone());

                                let undo = RegionUndoAtom::HeightmapEdit(
                                    prev,
                                    region.heightmap.clone(),
                                    tiles_to_render.clone(),
                                );
                                UNDOMANAGER
                                    .lock()
                                    .unwrap()
                                    .add_region_undo(&region.id, undo, ctx);
                            } else {
                                let new_id = Uuid::new_v4();
                                geo.id = new_id;
                                geo.set_default_position(coord);
                                let obj_id = region.add_geo_node(geo);
                                if let Some((geo_obj, _)) = region.find_geo_node(new_id) {
                                    tiles_to_render.clone_from(&geo_obj.area);
                                }
                                server_ctx.curr_geo_object = Some(obj_id);
                                server_ctx.curr_geo_node = Some(new_id);
                                region_to_render = Some(region.clone());

                                server.update_region(region);

                                if let Some(obj) = region.geometry.get(&obj_id) {
                                    let undo = RegionUndoAtom::GeoFXObjectEdit(
                                        obj_id,
                                        None,
                                        Some(obj.clone()),
                                        tiles_to_render.clone(),
                                    );
                                    UNDOMANAGER
                                        .lock()
                                        .unwrap()
                                        .add_region_undo(&region.id, undo, ctx);
                                }

                                MODELFXEDITOR
                                    .lock()
                                    .unwrap()
                                    .set_geo_node_ui(server_ctx, project, ui, ctx);

                                self.processed_coords.insert(coord);
                            }
                            redraw = true;
                        }
                    }
                } else {
                    // Apply material

                    let mut region_to_render: Option<Region> = None;
                    let mut tiles_to_render: Vec<Vec2i> = vec![];

                    if let Some(material_id) = server_ctx.curr_material_object {
                        // Set the material to the current geometry node.
                        if !three_d {
                            if let Some(editor) = ui.get_rgba_layout("Region Editor") {
                                if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                    let p = rgba_view.float_pos();
                                    if let Some((obj, node_index)) =
                                        region.get_closest_geometry(p, self.curr_layer_role)
                                    {
                                        if let Some(geo_obj) = region.geometry.get_mut(&obj) {
                                            server_ctx.curr_geo_object = Some(geo_obj.id);
                                            server_ctx.curr_geo_node =
                                                Some(geo_obj.nodes[node_index].id);

                                            let prev = geo_obj.clone();

                                            geo_obj.material_id = material_id;
                                            geo_obj.update_area();

                                            tiles_to_render.clone_from(&geo_obj.area);

                                            let undo = RegionUndoAtom::GeoFXObjectEdit(
                                                geo_obj.id,
                                                Some(prev),
                                                Some(geo_obj.clone()),
                                                tiles_to_render.clone(),
                                            );
                                            UNDOMANAGER
                                                .lock()
                                                .unwrap()
                                                .add_region_undo(&region.id, undo, ctx);

                                            server.update_region(region);
                                            region_to_render = Some(region.clone());
                                        }
                                    }
                                }
                            }
                        } else if let Some((obj, node_index)) =
                            region.get_closest_geometry(Vec2f::from(coord), self.curr_layer_role)
                        {
                            if let Some(geo_obj) = region.geometry.get_mut(&obj) {
                                server_ctx.curr_geo_object = Some(geo_obj.id);
                                server_ctx.curr_geo_node = Some(geo_obj.nodes[node_index].id);

                                let prev = geo_obj.clone();

                                geo_obj.material_id = material_id;
                                geo_obj.update_area();

                                tiles_to_render.clone_from(&geo_obj.area);

                                let undo = RegionUndoAtom::GeoFXObjectEdit(
                                    geo_obj.id,
                                    Some(prev),
                                    Some(geo_obj.clone()),
                                    tiles_to_render.clone(),
                                );
                                UNDOMANAGER
                                    .lock()
                                    .unwrap()
                                    .add_region_undo(&region.id, undo, ctx);

                                server.update_region(region);
                                region_to_render = Some(region.clone());
                            }
                        }

                        // Render the region area covered by the object with the new material.
                        if let Some(region) = region_to_render {
                            PRERENDERTHREAD
                                .lock()
                                .unwrap()
                                .render_region(region, Some(tiles_to_render));
                        }
                    }
                }

                /*
                model.create_voxels(
                    region.grid_size as u8,
                    &vec3f(coord.x as f32, 0.0, coord.y as f32),
                    &palette,
                );

                let undo;
                if let Some(modelstore) = region.models.get_mut(&(coord.x, 0, coord.y)) {
                    let prev = Some(modelstore.clone());
                    if self.curr_layer_role == Layer2DRole::Ground {
                        modelstore.floor = model;
                    } else if self.curr_layer_role == Layer2DRole::Wall {
                        modelstore.wall = model;
                    } else if self.curr_layer_role == Layer2DRole::Ceiling {
                        modelstore.ceiling = model;
                    }
                    undo = RegionUndoAtom::ModelFXEdit(
                        vec3i(coord.x, 0, coord.y),
                        prev,
                        Some(modelstore.clone()),
                    );
                } else {
                    let mut modelstore = ModelFXStore::default();
                    if self.curr_layer_role == Layer2DRole::Ground {
                        modelstore.floor = model;
                    } else if self.curr_layer_role == Layer2DRole::Wall {
                        modelstore.wall = model;
                    } else if self.curr_layer_role == Layer2DRole::Ceiling {
                        modelstore.ceiling = model;
                    }
                    undo = RegionUndoAtom::ModelFXEdit(
                        vec3i(coord.x, 0, coord.y),
                        None,
                        Some(modelstore.clone()),
                    );
                    region.models.insert((coord.x, 0, coord.y), modelstore);
                }
                UNDOMANAGER
                    .lock()
                    .unwrap()
                    .add_region_undo(&region.id, undo, ctx);
                server.update_region(region);
                RENDERER.lock().unwrap().set_region(region);
                */
            }

            if let Some(region) = region_to_render {
                PRERENDERTHREAD
                    .lock()
                    .unwrap()
                    .render_region(region, Some(tiles_to_render));
            }
        } else if self.editor_mode == EditorMode::Select {
            let p = (coord.x, coord.y);

            if let Some(tilearea) = &mut server_ctx.tile_selection {
                if !tilearea.ongoing {
                    tilearea.start = p;
                    tilearea.end = p;
                    tilearea.ongoing = true;
                } else {
                    tilearea.grow_by(p);
                }
            } else {
                let tilearea = TileArea {
                    start: p,
                    end: p,
                    ..Default::default()
                };
                server_ctx.tile_selection = Some(tilearea);
            }
        } else if self.editor_mode == EditorMode::Erase {
            let palette = project.palette.clone();
            // If there is a character instance at the position we delete the instance.

            if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                // We only check to delete models and tiles.
                // Characters, items and areas need to be erased by the Sidebar Region Content List.

                /*
                if let Some(c) =
                    server.get_character_at(server_ctx.curr_region, vec2i(coord.x, coord.y))
                {
                    // Delete the character at the given position.

                    if let Some((value, _)) =
                        server.get_character_property(region.id, c.0, "name".to_string())
                    {
                        open_delete_confirmation_dialog(
                            "Delete Character Instance ?",
                            format!("Permanently delete '{}' ?", value.describe()).as_str(),
                            c.0,
                            ui,
                            ctx,
                        );
                    }
                } else if let Some(c) =
                    server.get_item_at(server_ctx.curr_region, vec2i(coord.x, coord.y))
                {
                    // Delete the item at the given position.

                    if let Some((value, _)) =
                        server.get_character_property(region.id, c.0, "name".to_string())
                    {
                        open_delete_confirmation_dialog(
                            "Delete Item Instance ?",
                            format!("Permanently delete '{}' ?", value.describe()).as_str(),
                            c.0,
                            ui,
                            ctx,
                        );
                    }
                } else {
                */
                //let area_id: Option<Uuid> = None;

                /*
                // Check for area at the given position.
                for area in region.areas.values() {
                    if area.area.contains(&(coord.x, coord.y)) {
                        // Ask to delete it.
                        open_delete_confirmation_dialog(
                            "Delete Area ?",
                            format!("Permanently delete area '{}' ?", area.name).as_str(),
                            area.id,
                            ui,
                            ctx,
                        );
                        area_id = Some(area.id);
                        break;
                    }
                    }*/

                let mut region_to_render: Option<Region> = None;
                let mut tiles_to_render: Vec<Vec2i> = vec![];

                // Delete the tile at the given position.

                if self.curr_layer_role == Layer2DRole::FX {
                    if let Some(tile) = region.tiles.get_mut(&(coord.x, coord.y)) {
                        tile.tilefx = None;
                    }
                }

                let mut changed = false;

                // Check for geometry to delete
                if let Some(geo_obj_ids) =
                    region.geometry_areas.get_mut(&vec3i(coord.x, 0, coord.y))
                {
                    let mut objects = vec![];
                    for obj_id in geo_obj_ids {
                        let mut remove_it = false;

                        if let Some(geo_obj) = region.geometry.get(obj_id) {
                            remove_it = Some(self.curr_layer_role) == geo_obj.get_layer_role();
                        }

                        if remove_it {
                            if let Some(geo_obj) = region.geometry.remove(obj_id) {
                                for a in &geo_obj.area {
                                    tiles_to_render.push(*a);
                                }
                                objects.push(geo_obj.clone());
                            }
                        }
                    }

                    if !objects.is_empty() {
                        changed = true;
                        region_to_render = Some(region.clone());

                        region.update_geometry_areas();
                        let undo =
                            RegionUndoAtom::GeoFXObjectsDeletion(objects, tiles_to_render.clone());
                        UNDOMANAGER
                            .lock()
                            .unwrap()
                            .add_region_undo(&region.id, undo, ctx);
                    }
                }

                // Check for tiles to delete
                if !changed {
                    if let Some(tile) = region.tiles.get_mut(&(coord.x, coord.y)) {
                        let prev = Some(tile.clone());
                        if self.curr_layer_role == Layer2DRole::Ground && tile.layers[0].is_some() {
                            tile.layers[0] = None;
                            changed = true;
                        } else if self.curr_layer_role == Layer2DRole::Wall
                            && tile.layers[1].is_some()
                        {
                            tile.layers[1] = None;
                            changed = true;
                        } else if self.curr_layer_role == Layer2DRole::Ceiling
                            && tile.layers[2].is_some()
                        {
                            tile.layers[2] = None;
                            changed = true;
                        }
                        if changed {
                            tiles_to_render.push(coord);
                            let undo = RegionUndoAtom::RegionTileEdit(
                                vec2i(coord.x, coord.y),
                                prev,
                                Some(tile.clone()),
                            );
                            UNDOMANAGER
                                .lock()
                                .unwrap()
                                .add_region_undo(&region.id, undo, ctx);
                        }
                    }

                    if changed {
                        region_to_render = Some(region.clone());
                    }
                }

                if changed {
                    server.update_region(region);
                    RENDERER.lock().unwrap().set_region(region);
                    self.set_icon_previews(region, &palette, coord, ui);
                    redraw = true;
                }

                if let Some(region) = region_to_render {
                    PRERENDERTHREAD
                        .lock()
                        .unwrap()
                        .render_region(region, Some(tiles_to_render));
                }
            }
        } else if self.editor_mode == EditorMode::Pick {
            let mut clicked_tile = false;
            let mut found_geo = false;

            // Check for character at the given position.
            if let Some(c) = server.get_character_at(server_ctx.curr_region, coord) {
                server_ctx.curr_character_instance = Some(c.0);
                server_ctx.curr_character = Some(c.1);
                server_ctx.curr_area = None;
                server_ctx.curr_item_instance = None;
                server_ctx.curr_item = None;

                // Set 3D editing position to Zero.
                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.editing_position_3d = Vec3f::zero();
                    server.set_editing_position_3d(region.editing_position_3d);
                }

                if let Some(layout) = ui.get_list_layout("Region Content List") {
                    layout.select_item(c.0, ctx, false);
                }

                if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                    || *SIDEBARMODE.lock().unwrap() == SidebarMode::Character
                {
                    // In Region mode, we need to set the character bundle of the current character instance.
                    if let Some(region) = project.get_region(&server_ctx.curr_region) {
                        if let Some(character) = region.characters.get(&c.0) {
                            for grid in character.instance.grids.values() {
                                if grid.name == "init" {
                                    CODEEDITOR.lock().unwrap().set_codegrid(grid.clone(), ui);
                                    CODEEDITOR.lock().unwrap().code_id = str!("Character Instance");
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Set CodeGrid Panel"),
                                        TheValue::Empty,
                                    ));
                                    self.set_editor_group_index(EditorMode::Code, ui, ctx);
                                }
                            }
                        }
                    }
                }
                //else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                // In Character mode, we need to set the character bundle of the current character.
                //}
            }
            // Check for an item at the given position.
            else if let Some(c) = server.get_item_at(server_ctx.curr_region, coord) {
                server_ctx.curr_character_instance = None;
                server_ctx.curr_character = None;
                server_ctx.curr_item_instance = Some(c.0);
                server_ctx.curr_item = Some(c.1);
                server_ctx.curr_area = None;

                if let Some(layout) = ui.get_list_layout("Region Content List") {
                    layout.select_item(c.0, ctx, false);
                }

                if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                    || *SIDEBARMODE.lock().unwrap() == SidebarMode::Item
                {
                    // In Region mode, we need to set the character bundle of the current character instance.
                    if let Some(region) = project.get_region(&server_ctx.curr_region) {
                        if let Some(item) = region.items.get(&c.0) {
                            for grid in item.instance.grids.values() {
                                if grid.name == "init" {
                                    CODEEDITOR.lock().unwrap().set_codegrid(grid.clone(), ui);
                                    CODEEDITOR.lock().unwrap().code_id = str!("Item Instance");
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Set CodeGrid Panel"),
                                        TheValue::Empty,
                                    ));
                                    self.set_editor_group_index(EditorMode::Code, ui, ctx);
                                }
                            }
                        }
                    }
                }
                //else if *SIDEBARMODE.lock().unwrap() == SidebarMode::Character {
                // In Character mode, we need to set the character bundle of the current character.
                //}
            } else if let Some(region) = project.get_region(&server_ctx.curr_region) {
                let found_area = false;

                // Check for area at the given position.
                // for area in region.areas.values() {
                //     if area.area.contains(&(coord.x, coord.y)) {
                //         for grid in area.bundle.grids.values() {
                //             if grid.name == "main" {
                //                 if *SIDEBARMODE.lock().unwrap() == SidebarMode::Region
                //                     || *SIDEBARMODE.lock().unwrap() == SidebarMode::Character
                //                 {
                //                     CODEEDITOR.lock().unwrap().set_codegrid(grid.clone(), ui);
                //                     ctx.ui.send(TheEvent::Custom(
                //                         TheId::named("Set CodeGrid Panel"),
                //                         TheValue::Empty,
                //                     ));
                //                 }
                //                 found_area = true;
                //                 server_ctx.curr_character_instance = None;
                //                 server_ctx.curr_character = None;
                //                 server_ctx.curr_area = Some(area.id);
                //                 if let Some(layout) = ui.get_list_layout("Region Content List") {
                //                     layout.select_item(area.id, ctx, false);
                //                 }
                //                 break;
                //             }
                //         }
                //     }
                // }

                if !found_area {
                    // No area, set the tile.

                    server_ctx.curr_character_instance = None;
                    if !three_d {
                        // Test against object SDFs float position in 2d
                        if let Some(editor) = ui.get_rgba_layout("Region Editor") {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                let p = rgba_view.float_pos();
                                if let Some((obj, node_index)) =
                                    region.get_closest_geometry(p, self.curr_layer_role)
                                {
                                    if let Some(geo) = region.geometry.get(&obj) {
                                        server_ctx.curr_geo_object = Some(geo.id);
                                        server_ctx.curr_geo_node = Some(geo.nodes[node_index].id);

                                        ctx.ui.send(TheEvent::Custom(
                                            TheId::named("Set Region Modeler"),
                                            TheValue::Empty,
                                        ));
                                        found_geo = true;
                                    }
                                }
                            }
                        }
                    } else if let Some((obj, node_index)) =
                        region.get_closest_geometry(Vec2f::from(coord), self.curr_layer_role)
                    {
                        if let Some(geo) = region.geometry.get(&obj) {
                            server_ctx.curr_geo_object = Some(geo.id);
                            server_ctx.curr_geo_node = Some(geo.nodes[node_index].id);

                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Set Region Modeler"),
                                TheValue::Empty,
                            ));
                            found_geo = true;
                        }
                    }

                    if !found_geo {
                        if let Some(tile) = region.tiles.get(&(coord.x, coord.y)) {
                            if self.curr_layer_role == Layer2DRole::FX {
                                // Set the tile preview.
                                if let Some(widget) = ui.get_widget("TileFX RGBA") {
                                    if let Some(tile_rgba) = widget.as_rgba_view() {
                                        if let Some(tile) = project.extract_region_tile(
                                            server_ctx.curr_region,
                                            (coord.x, coord.y),
                                        ) {
                                            let preview_size =
                                                TILEFXEDITOR.lock().unwrap().preview_size;
                                            tile_rgba.set_grid(Some(
                                                preview_size / tile.buffer[0].dim().width,
                                            ));
                                            tile_rgba.set_buffer(
                                                tile.buffer[0].scaled(preview_size, preview_size),
                                            );
                                        }
                                    }
                                }
                                if let Some(timeline) = &tile.tilefx {
                                    TILEFXEDITOR
                                        .lock()
                                        .unwrap()
                                        .set_timeline(timeline.clone(), ui);
                                }
                            } else {
                                for uuid in tile.layers.iter().flatten() {
                                    if TILEDRAWER.lock().unwrap().tiles.contains_key(uuid) {
                                        ctx.ui.send(TheEvent::StateChanged(
                                            TheId::named_with_id("Tilemap Tile", *uuid),
                                            TheWidgetState::Selected,
                                        ));
                                        clicked_tile = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            MODELFXEDITOR
                .lock()
                .unwrap()
                .set_geo_node_ui(server_ctx, project, ui, ctx);
            if clicked_tile {
                self.set_editor_group_index(EditorMode::Draw, ui, ctx);
            }
        } else if self.editor_mode == EditorMode::Draw {
        }
        redraw
        }*/

    // Sets the index of the editor group.
    // fn set_editor_group_index(&mut self, mode: EditorMode, ui: &mut TheUI, ctx: &mut TheContext) {
    //     if let Some(widget) = ui.get_group_button("Editor Group") {
    //         widget.set_index(mode as i32);
    //         ctx.ui
    //             .send(TheEvent::IndexChanged(widget.id().clone(), mode as usize));
    //     }
    // }
}
