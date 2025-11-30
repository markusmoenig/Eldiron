use crate::editor::RUSTERIX;
use crate::prelude::*;
use rusterix::ShapeStack;
use rusterix::prelude::*;
use theframework::prelude::*;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HudMode {
    Selection,
    Vertex,
    Linedef,
    Sector,
    Effects,
    Rect,
    Terrain,
}

pub struct Hud {
    mode: HudMode,

    icon_rects: Vec<TheDim>,

    pub selected_icon_index: i32,

    subdiv_rects: Vec<TheDim>,

    show_softrigs: bool,
    poses_rects: Vec<TheDim>,
    add_pose_rect: TheDim,

    play_button_rect: TheDim,
    timeline_rect: TheDim,

    profile2d_rect: TheDim,

    mouse_pos: Vec2<i32>,

    is_playing: bool,
    light_icon: Option<TheRGBABuffer>,
}

impl Hud {
    pub fn new(mode: HudMode) -> Self {
        Self {
            mode,

            icon_rects: vec![],
            selected_icon_index: 0,

            subdiv_rects: vec![],

            show_softrigs: false,
            poses_rects: vec![],
            add_pose_rect: TheDim::rect(0, 0, 0, 0),

            play_button_rect: TheDim::rect(0, 0, 0, 0),
            timeline_rect: TheDim::rect(0, 0, 0, 0),

            profile2d_rect: TheDim::rect(0, 0, 0, 0),

            mouse_pos: Vec2::zero(),

            is_playing: false,
            light_icon: None,
        }
    }

    pub fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        id: Option<u32>,
        assets: &Assets,
    ) {
        if (self.mode == HudMode::Linedef || self.mode == HudMode::Sector)
            && self.light_icon.is_none()
        {
            if let Some(li) = ctx.ui.icon("light_small") {
                self.light_icon = Some(li.clone());
            }
        }

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;
        let stride = buffer.stride();

        let info_height = 20;
        let bg_color = [50, 50, 50, 255];
        let dark_bg_color = [30, 30, 30, 255];
        let text_color = [150, 150, 150, 255];
        let sel_text_color = [220, 220, 220, 255];

        self.poses_rects.clear();
        self.subdiv_rects = vec![];

        ctx.draw.rect(
            buffer.pixels_mut(),
            &(0, 0, width, info_height),
            stride,
            &bg_color,
        );

        if let Some(v) = server_ctx.hover_cursor {
            ctx.draw.text(
                buffer.pixels_mut(),
                &(10, 2),
                stride,
                &format!("{:.2}, {:.2}", v.x, v.y),
                TheFontSettings {
                    size: 13.0,
                    ..Default::default()
                },
                &text_color,
                &bg_color,
            );
        }

        if let Some(v) = &server_ctx.background_progress {
            ctx.draw.text(
                buffer.pixels_mut(),
                &(550, 2),
                stride,
                v,
                TheFontSettings {
                    size: 13.0,
                    ..Default::default()
                },
                &text_color,
                &bg_color,
            );
        }

        self.show_softrigs = server_ctx.get_map_context() == MapContext::Shader
            || server_ctx.get_map_context() == MapContext::Character
            || server_ctx.get_map_context() == MapContext::Item;

        // SoftRigs
        if self.show_softrigs {
            let x = 0;
            let poses_width = 100;
            let poses_height = 25_i32;
            let mut y = height - poses_height as usize - map.softrigs.len() * poses_height as usize;

            // Base State
            let rect = TheDim::rect(x, y as i32, poses_width, poses_height);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &dark_bg_color,
            );

            let r = rect.to_buffer_utuple();
            ctx.draw.text_rect(
                buffer.pixels_mut(),
                &(r.0 + 4, r.1, r.2 - 8, r.3),
                stride,
                "Base State",
                TheFontSettings {
                    size: 11.5,
                    ..Default::default()
                },
                if map.editing_rig.is_none() {
                    &sel_text_color
                } else {
                    &text_color
                },
                &dark_bg_color,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );

            self.poses_rects.push(rect);

            // Draw Rigs
            y += poses_height as usize;
            for (i, (id, rig)) in map.softrigs.iter().enumerate() {
                let selected = match map.editing_rig {
                    Some(selected_id) => *id == selected_id,
                    None => false,
                };

                let rect = TheDim::rect(x, y as i32, poses_width, poses_height);
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &dark_bg_color,
                );

                if rig.in_editor_playlist {
                    let r = rect.to_buffer_utuple();
                    ctx.draw.text_rect(
                        buffer.pixels_mut(),
                        &(r.0, r.1, 20, r.3),
                        stride,
                        "X",
                        TheFontSettings {
                            size: 11.5,
                            ..Default::default()
                        },
                        if selected {
                            &sel_text_color
                        } else {
                            &text_color
                        },
                        &dark_bg_color,
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }

                let r = rect.to_buffer_utuple();
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &(r.0 + 20, r.1, r.2 - 25, r.3),
                    stride,
                    &map.softrigs[i].name,
                    TheFontSettings {
                        size: 11.5,
                        ..Default::default()
                    },
                    if selected {
                        &sel_text_color
                    } else {
                        &text_color
                    },
                    &dark_bg_color,
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );

                self.poses_rects.push(rect);
                y += poses_height as usize;
            }

            // Plus buttton
            y -= poses_height as usize;
            let rect = TheDim::rect(poses_width, y as i32, poses_height, poses_height);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &bg_color,
            );

            ctx.draw.text_rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                "+",
                TheFontSettings {
                    size: 11.5,
                    ..Default::default()
                },
                // if map.animation.current_state.is_none() {
                //     &sel_text_color
                // } else {
                //     &text_color
                // },
                &text_color,
                &dark_bg_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );

            self.add_pose_rect = rect;

            // Play button and timeline

            let rect = TheDim::rect(150, y as i32, poses_height, poses_height);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &bg_color,
            );

            ctx.draw.text_rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                "P",
                TheFontSettings {
                    size: 11.5,
                    ..Default::default()
                },
                // if map.animation.current_state.is_none() {
                //     &sel_text_color
                // } else {
                //     &text_color
                // },
                &text_color,
                &dark_bg_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );

            self.play_button_rect = rect;

            if self.is_playing {
                map.tick(1.0 / 30.0);
            }

            let rect = TheDim::rect(150 + poses_height, y as i32, 150, poses_height);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &dark_bg_color,
            );

            self.timeline_rect = rect;
        }

        // Icons

        let icon_size = 40;
        let mut icons = 0;

        if server_ctx.get_map_context() == MapContext::Region {
            icons = if self.mode == HudMode::Vertex {
                0
            } else if self.mode == HudMode::Linedef {
                0
            } else {
                1
            };
        } else if server_ctx.get_map_context() == MapContext::Shader {
            icons = 1;
        } else if server_ctx.get_map_context() == MapContext::Screen {
            icons = if self.mode == HudMode::Sector { 2 } else { 0 };
        }

        if self.mode == HudMode::Effects
            || self.mode == HudMode::Rect
            || self.mode == HudMode::Terrain
        {
            icons = 0;
        }

        self.icon_rects.clear();
        let x = width as i32 - (icon_size * icons) - 1;
        for i in 0..icons {
            let rect = TheDim::rect(x + (i * icon_size), 20, icon_size, icon_size);
            ctx.draw.rect(
                buffer.pixels_mut(),
                &rect.to_buffer_utuple(),
                stride,
                &bg_color,
            );

            let r = rect.to_buffer_utuple();
            ctx.draw.text_rect(
                buffer.pixels_mut(),
                &(r.0, 1, r.2, 19),
                stride,
                &self.get_icon_text(i, server_ctx),
                TheFontSettings {
                    size: 10.0,
                    ..Default::default()
                },
                &text_color,
                &bg_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );

            let r = &rect.to_buffer_utuple();
            ctx.draw.rect(
                buffer.pixels_mut(),
                &(r.0 + 1, r.1 + 1, r.2 - 2, r.3 - 2),
                stride,
                &[30, 30, 30, 255],
            );

            if let Some(id) = id {
                let (tile, has_light) = self.get_icon(i, map, id, icon_size as usize);
                if let Some(tile) = tile {
                    let texture = tile.textures[0].resized(icon_size as usize, icon_size as usize);
                    ctx.draw.copy_slice(
                        buffer.pixels_mut(),
                        &texture.data,
                        &rect.to_buffer_utuple(),
                        stride,
                    );
                }
                if has_light {
                    if let Some(light_icon) = &self.light_icon {
                        ctx.draw.blend_slice(
                            buffer.pixels_mut(),
                            light_icon.pixels(),
                            &(
                                rect.x as usize + 1,
                                rect.y as usize + 1,
                                light_icon.dim().width as usize,
                                light_icon.dim().height as usize,
                            ),
                            stride,
                        );
                    }
                }
            }

            if i == self.selected_icon_index {
                ctx.draw.rect_outline(
                    buffer.pixels_mut(),
                    &rect.to_buffer_utuple(),
                    stride,
                    &sel_text_color,
                );
            }

            self.icon_rects.push(rect);
        }

        // Show Profile
        if server_ctx.get_map_context() == MapContext::Region {
            let x = 390;
            let size = 20;
            self.profile2d_rect = TheDim::rect(x, 0, 60, size);

            let txt = "Region";

            let r = self.profile2d_rect.to_buffer_utuple();
            ctx.draw.text_rect(
                buffer.pixels_mut(),
                &(r.0, 1, r.2, 19),
                stride,
                txt,
                TheFontSettings {
                    size: 13.0,
                    ..Default::default()
                },
                &if self.profile2d_rect.contains(self.mouse_pos) {
                    sel_text_color
                } else {
                    text_color
                },
                &bg_color,
                TheHorizontalAlign::Center,
                TheVerticalAlign::Center,
            );

            if let Some(_editing_surface) = &server_ctx.editing_surface {
                let txt = ">  Surface Profile";
                let r = self.profile2d_rect.to_buffer_utuple();
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &(r.0 + 55, 1, r.2 + 60, 19),
                    stride,
                    txt,
                    TheFontSettings {
                        size: 13.0,
                        ..Default::default()
                    },
                    &text_color,
                    &bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
        }

        // Show Subdivs
        if (map.camera == MapCamera::TwoD
            || server_ctx.get_map_context() == MapContext::Shader
            || server_ctx.get_map_context() == MapContext::Screen)
            && self.mode != HudMode::Terrain
        {
            let x = 150;

            let size = 20;
            for i in 0..10 {
                let rect = TheDim::rect(x + (i * size), 0, size, size);

                let r = rect.to_buffer_utuple();
                ctx.draw.text_rect(
                    buffer.pixels_mut(),
                    &(r.0, 1, r.2, 19),
                    stride,
                    &(i + 1).to_string(),
                    TheFontSettings {
                        size: 13.0,
                        ..Default::default()
                    },
                    &if (i + 1) as f32 == map.subdivisions || rect.contains(self.mouse_pos) {
                        sel_text_color
                    } else {
                        text_color
                    },
                    &bg_color,
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
                self.subdiv_rects.push(rect);
            }
        }

        // Terrain: Height
        if self.mode == HudMode::Terrain {
            if let Some(v) = server_ctx.hover_height {
                ctx.draw.text(
                    buffer.pixels_mut(),
                    &(150, 2),
                    stride,
                    &format!("Elevation {v:.2}"),
                    TheFontSettings {
                        size: 13.0,
                        ..Default::default()
                    },
                    &text_color,
                    &bg_color,
                );
            }
        }

        // Preview

        if server_ctx.get_map_context() == MapContext::Character
            || server_ctx.get_map_context() == MapContext::Item
        {
            if self.is_playing {
                let size = 128;
                let mut texture = Texture::alloc(size as usize, size as usize);

                let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
                stack.render_geometry(&mut texture, map, assets, false, &FxHashMap::default());

                map.properties.set("shape", Value::Texture(texture));
            }

            if let Some(Value::Texture(texture)) = map.properties.get("shape") {
                let w = texture.width as i32;
                let h = texture.height as i32;
                let preview_rect = TheDim::rect(width as i32 - w - 1, height as i32 - h - 1, w, h);
                ctx.draw.copy_slice(
                    buffer.pixels_mut(),
                    &texture.data,
                    &preview_rect.to_buffer_utuple(),
                    stride,
                );
            }
        } else if server_ctx.get_map_context() == MapContext::Shader {
            if let Some(Value::Texture(texture)) = map.properties.get("material") {
                let w = texture.width as i32;
                let h = texture.height as i32;
                let preview_rect = TheDim::rect(width as i32 - w - 1, height as i32 - h - 1, w, h);
                ctx.draw.copy_slice(
                    buffer.pixels_mut(),
                    &texture.data,
                    &preview_rect.to_buffer_utuple(),
                    stride,
                );
            }
        } else if server_ctx.get_map_context() == MapContext::Screen {
            // Show the widget previews in the icon rects
            if let Some(id) = map.selected_sectors.get(0) {
                if let Some(sector) = map.find_sector(*id) {
                    if let Some(Value::Source(PixelSource::ShapeFXGraphId(id))) =
                        sector.properties.get("screen_graph")
                    {
                        if let Some(graph) = map.shapefx_graphs.get(id)
                            && self.icon_rects.len() == 2
                        {
                            let w = self.icon_rects[0].width;
                            let h = self.icon_rects[0].height;

                            let textures =
                                graph.create_screen_widgets(w as usize, h as usize, assets);

                            for i in 0..2 {
                                ctx.draw.copy_slice(
                                    buffer.pixels_mut(),
                                    &textures[i as usize].data,
                                    &self.icon_rects[i as usize].to_buffer_utuple(),
                                    stride,
                                );
                            }
                        }
                    }
                }
            }

            //let mut stack = ShapeStack::new(Vec2::new(-5.0, -5.0), Vec2::new(5.0, 5.0));
            //stack.render_geometry(&mut texture, map, assets, false, &FxHashMap::default());

            /*
            if let Some(Value::Texture(texture)) = map.properties.get("shape") {
                let w = texture.width as i32;
                let h = texture.height as i32;
                let preview_rect = TheDim::rect(width as i32 - w - 1, height as i32 - h - 1, w, h);
                ctx.draw.copy_slice(
                    buffer.pixels_mut(),
                    &texture.data,
                    &preview_rect.to_buffer_utuple(),
                    stride,
                );
            }*/
        }
    }

    pub fn clicked(
        &mut self,
        x: i32,
        y: i32,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if server_ctx.get_map_context() != MapContext::Region
            && server_ctx.get_map_context() != MapContext::Shader
            && server_ctx.get_map_context() != MapContext::Screen
            && server_ctx.get_map_context() != MapContext::Character
            && server_ctx.get_map_context() != MapContext::Item
        {
            return false;
        }

        for (i, rect) in self.icon_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                self.selected_icon_index = i as i32;
                server_ctx.selected_hud_icon_index = i as i32;
                if self.mode == HudMode::Linedef {
                    server_ctx.selected_wall_row = Some(i as i32);
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Map Selection Changed"),
                        TheValue::Empty,
                    ));
                }
                return true;
            }
        }
        for (i, rect) in self.subdiv_rects.iter().enumerate() {
            if rect.contains(Vec2::new(x, y)) {
                map.subdivisions = (i + 1) as f32;
                return true;
            }
        }

        if self.profile2d_rect.contains(Vec2::new(x, y)) && server_ctx.editing_surface.is_some() {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Render SceneManager Map"),
                TheValue::Empty,
            ));
            server_ctx.editing_surface = None;
            RUSTERIX.write().unwrap().set_dirty();
            return true;
        }

        // Parse Softrigs
        if self.show_softrigs {
            for (i, rect) in self.poses_rects.iter().enumerate() {
                if rect.contains(Vec2::new(x, y)) {
                    if i == 0 {
                        // Base state selected (no animation)
                        map.editing_rig = None;
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Base State Selected"),
                            TheValue::Empty,
                        ));
                    } else if let Some((_, softrig)) = map.softrigs.get_index_mut(i - 1) {
                        if x < 22 {
                            softrig.in_editor_playlist = !softrig.in_editor_playlist;
                        } else {
                            map.editing_rig = Some(softrig.id);
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("SoftRig Selected"),
                                TheValue::Id(softrig.id),
                            ));
                        }
                    }

                    /*
                    // Animation state: get the (Uuid, SkeletalAnimation) by index (i - 1)
                    let anim_index = i - 1;
                    if let Some((anim_id, anim)) = map.skeletal_animations.get_index(anim_index) {
                        if ui.shift && map.editing_anim_pose.map_or(true, |(id, _)| id != *anim_id)
                        {
                            // Optional shift behavior (e.g., preview?)
                            // Example: queue animation without switching state
                            // map.preview_animation = Some(*anim_id);
                        } else {
                            // Set the currently edited pose (first frame for now)
                            map.editing_anim_pose = Some((*anim_id, 0));
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Anim State Selected"),
                                TheValue::Id(*anim_id),
                            ));
                        }
                    }*/

                    return true;
                }
            }

            // Add SoftRig
            if self.add_pose_rect.contains(Vec2::new(x, y)) {
                let rig = SoftRig::new("Idle".into());
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("SoftRig Selected"),
                    TheValue::Id(rig.id),
                ));
                map.editing_rig = Some(rig.id);
                map.softrigs.insert(rig.id, rig);

                return true;
            }
            // Play Button
            if self.play_button_rect.contains(Vec2::new(x, y)) {
                if !self.is_playing {
                    let animator = SoftRigAnimator {
                        keyframes: map
                            .softrigs
                            .values()
                            .filter(|rig| rig.in_editor_playlist)
                            .map(|rig| rig.id)
                            .collect(),
                        // total_duration: 0.5,
                        ..Default::default()
                    };
                    map.soft_animator = Some(animator);
                } else {
                    map.soft_animator = None;
                }
                self.is_playing = !self.is_playing;
                return true;
            }
            // Timeline
            if self.timeline_rect.contains(Vec2::new(x, y)) {
                // let offset = x - self.timeline_rect.x;
                // let progress = offset as f32 / self.timeline_rect.width as f32;
                // map.animation.transition_progress = progress;
                // println!("{:?}", map.animation);
                return true;
            }
        }

        if map.camera == MapCamera::TwoD && y < 20 {
            return true;
        }

        false
    }

    pub fn dragged(
        &mut self,
        _x: i32,
        _y: i32,
        _map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        /*
        if self.timeline_rect.contains(Vec2::new(x, y)) {
            let offset = x - self.timeline_rect.x;
            let progress = offset as f32 / self.timeline_rect.width as f32;
            map.animation.transition_progress = progress;
            return true;
        }*/

        false
    }

    pub fn hovered(
        &mut self,
        x: i32,
        y: i32,
        _map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.mouse_pos = Vec2::new(x, y);

        /*
        if self.rect_geo_rect.contains(self.mouse_pos) {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                "Show or hide geometry created with the Rect tool.".to_string(),
            ));
        } else {
            ctx.ui
                .send(TheEvent::SetStatusText(TheId::empty(), "".into()));
        }*/
        false
    }

    #[allow(clippy::collapsible_if)]
    pub fn get_icon_text(&self, index: i32, server_ctx: &mut ServerContext) -> String {
        let mut text: String = "".into();
        if server_ctx.get_map_context() == MapContext::Region {
            if self.mode == HudMode::Sector {
                if index == 0 {
                    text = "TILE".into();
                }
            }
        } else if server_ctx.get_map_context() == MapContext::Shader {
            if index == 0 {
                text = "GRAPH".into();
            }
        } else if server_ctx.get_map_context() == MapContext::Screen {
            if index == 0 {
                text = "NORM".into();
            } else if index == 1 {
                text = "ACTIVE".into();
            }
        }

        text
    }

    #[allow(clippy::collapsible_if)]
    pub fn get_icon(
        &self,
        index: i32,
        map: &Map,
        id: u32,
        icon_size: usize,
    ) -> (Option<rusterix::Tile>, bool) {
        if self.mode == HudMode::Sector {
            if let Some(sector) = map.find_sector(id) {
                if index == 0 {
                    let has_light = sector.properties.get("floor_light").is_some();
                    if let Some(pixelsource) = sector.properties.get_default_source() {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.read().unwrap().assets,
                            icon_size,
                            &sector.properties,
                            map,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                } else if index == 1 {
                    let has_light = sector.properties.get("ceiling_light").is_some();
                    if let Some(Value::Source(pixelsource)) =
                        &sector.properties.get("ceiling_source")
                    {
                        if let Some(tile) = pixelsource.to_tile(
                            &RUSTERIX.read().unwrap().assets,
                            icon_size,
                            &sector.properties,
                            map,
                        ) {
                            return (Some(tile), has_light);
                        }
                    }
                    return (None, has_light);
                }
            }
        }

        (None, false)
    }
}
