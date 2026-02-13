use crate::docks::tiles_editor_undo::*;
use crate::editor::TOOLLIST;
use crate::prelude::*;

pub struct TilesEditorDock {
    zoom: f32,
    show_grid: bool,
    tile_node: Uuid,
    palette_node: Uuid,
    grid_node: Uuid,
    body_markers_node: Uuid,

    // Per-context undo stacks (keyed by tile_id for tiles, avatar_id for avatar frames)
    tile_undos: FxHashMap<Uuid, TileEditorUndo>,
    current_tile_id: Option<Uuid>,
    /// The current undo key — derived from the editing context.
    current_undo_key: Option<Uuid>,
    max_undo: usize,

    /// When true, the minimap cycles through animation frames.
    anim_preview: bool,
    paste_preview_texture: Option<rusterix::Texture>,
    paste_preview_pos: Option<Vec2<i32>>,
}

impl Dock for TilesEditorDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            zoom: 5.0,
            show_grid: true,
            tile_node: Uuid::new_v4(),
            palette_node: Uuid::new_v4(),
            grid_node: Uuid::new_v4(),
            body_markers_node: Uuid::new_v4(),
            tile_undos: FxHashMap::default(),
            current_tile_id: None,
            current_undo_key: None,
            max_undo: 30,
            anim_preview: false,
            paste_preview_texture: None,
            paste_preview_pos: None,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut rgba_layout = TheRGBALayout::new(TheId::named("Tile Editor Dock RGBA Layout"));
        if let Some(rgba_view) = rgba_layout.rgba_view_mut().as_rgba_view() {
            rgba_view.set_supports_external_zoom(true);
            rgba_view.set_background([116, 116, 116, 255]);
            // rgba_view.set_grid(Some(1));
            // rgba_view.set_grid_color([20, 20, 20, 255]);
            // rgba_view.set_dont_show_grid(true);
            rgba_view.set_dont_show_grid(!self.show_grid);
            rgba_view.set_show_transparency(true);
            rgba_view.set_mode(TheRGBAViewMode::TileEditor);
            let mut c = WHITE;
            c[3] = 128;
            rgba_view.set_hover_color(Some(c));
        }

        canvas.set_layout(rgba_layout);

        let mut stack_canvas = TheCanvas::new();
        let mut stack_layout = TheStackLayout::new(TheId::named("Pixel Editor Stack Layout"));
        stack_layout.limiter_mut().set_max_width(305);

        // Tree

        let mut palette_canvas = TheCanvas::default();
        let mut palette_tree_layout = TheTreeLayout::new(TheId::named("Tile Editor Tree"));
        palette_tree_layout.limiter_mut().set_max_width(305);
        let root = palette_tree_layout.get_root();

        // Tile
        let mut tile_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Tile", self.tile_node));
        tile_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Tile Size"));
        item.set_text(fl!("size"));

        let mut edit = TheTextLineEdit::new(TheId::named("Tile Size Edit"));
        edit.set_value(TheValue::Int(0));
        item.add_widget_column(150, Box::new(edit));
        tile_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Tile Frames"));
        item.set_text(fl!("frames"));

        let mut edit = TheTextLineEdit::new(TheId::named("Tile Frame Edit"));
        edit.set_value(TheValue::Int(0));
        item.add_widget_column(150, Box::new(edit));
        tile_node.add_widget(Box::new(item));

        let mut item = TheTreeIcons::new(TheId::named("Tile Frame Icons"));
        item.set_icon_size(40);
        item.set_icon_count(1);
        item.set_selected_index(Some(0));
        tile_node.add_widget(Box::new(item));

        root.add_child(tile_node);

        // Palette

        let mut palette_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Color", self.palette_node));
        palette_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Palette Opacity"));
        item.set_text(fl!("opacity"));

        let mut edit = TheTextLineEdit::new(TheId::named("Palette Opacity Edit"));
        edit.set_value(TheValue::Float(1.0));
        edit.set_range(TheValue::RangeF32(0.0..=1.0));
        item.add_widget_column(150, Box::new(edit));
        palette_node.add_widget(Box::new(item));

        // let mut item = TheTreeIcons::new(TheId::named("Palette Item"));
        // item.set_icon_count(256);
        // item.set_icons_per_row(14);
        // item.set_selected_index(Some(0));

        // palette_node.add_widget(Box::new(item));
        root.add_child(palette_node);

        // Grid
        let mut grid_node: TheTreeNode =
            TheTreeNode::new(TheId::named_with_id("Grid", self.grid_node));
        grid_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Grid Enabled"));
        let mut cb = TheCheckButton::new(TheId::named("Grid Enabled CB"));
        cb.set_state(TheWidgetState::Selected);
        item.add_widget_column(150, Box::new(cb));
        item.set_text(fl!("enabled"));

        grid_node.add_widget(Box::new(item));

        root.add_child(grid_node);

        //

        palette_canvas.set_layout(palette_tree_layout);

        stack_layout.add_canvas(palette_canvas);

        // Avatar

        let mut avatar_canvas = TheCanvas::default();
        let mut avatar_tree_layout = TheTreeLayout::new(TheId::named("Avatar Editor Tree"));
        avatar_tree_layout.limiter_mut().set_max_width(305);
        let root = avatar_tree_layout.get_root();

        let mut body_markers_node: TheTreeNode = TheTreeNode::new(TheId::named_with_id(
            &fl!("body_markers"),
            self.body_markers_node,
        ));
        body_markers_node.set_open(true);

        // •	Skin Light – rgb(255, 0, 255)
        // •	Skin Dark – rgb(200, 0, 200)
        // •	Torso / Chest – rgb(0, 0, 255)
        // •	Legs / Pants – rgb(0, 255, 0)
        // •	Hair – rgb(255, 255, 0)
        // •	Eyes / Face Detail – rgb(0, 255, 255)
        // •	Hands – rgb(255, 128, 0)
        // •	Feet – rgb(255, 80, 0)

        let mut item = TheTreeItem::new(TheId::named("Body: Skin Light"));
        item.set_text(fl!("skin_light"));
        item.set_background_color(TheColor::from_u8_array_3([255, 0, 255]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Skin Dark"));
        item.set_text(fl!("skin_dark"));
        item.set_background_color(TheColor::from_u8_array_3([200, 0, 200]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Torso"));
        item.set_text(fl!("torso"));
        item.set_background_color(TheColor::from_u8_array_3([0, 0, 255]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Legs"));
        item.set_text(fl!("legs"));
        item.set_background_color(TheColor::from_u8_array_3([0, 255, 0]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Hair"));
        item.set_text(fl!("hair"));
        item.set_background_color(TheColor::from_u8_array_3([255, 255, 0]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Eyes"));
        item.set_text(fl!("eyes"));
        item.set_background_color(TheColor::from_u8_array_3([0, 255, 255]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Hands"));
        item.set_text(fl!("hands"));
        item.set_background_color(TheColor::from_u8_array_3([255, 128, 0]));
        body_markers_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Body: Feet"));
        item.set_text(fl!("feet"));
        item.set_background_color(TheColor::from_u8_array_3([255, 80, 0]));
        body_markers_node.add_widget(Box::new(item));

        root.add_child(body_markers_node);

        let mut anchors_node: TheTreeNode = TheTreeNode::new(TheId::named(&fl!("anchors")));
        anchors_node.set_open(true);

        let mut item = TheTreeItem::new(TheId::named("Anchor: Main"));
        item.set_text(fl!("avatar_anchor_main"));
        anchors_node.add_widget(Box::new(item));

        let mut item = TheTreeItem::new(TheId::named("Anchor: Off"));
        item.set_text(fl!("avatar_anchor_off"));
        anchors_node.add_widget(Box::new(item));

        // let mut item = TheTreeItem::new(TheId::named("Body: Extra"));
        // item.set_text(fl!("extra"));
        // item.set_background_color(TheColor::from_u8_array_3([255, 0, 0]));
        // body_markers_node.add_widget(Box::new(item));

        root.add_child(anchors_node);

        avatar_canvas.set_layout(avatar_tree_layout);

        stack_layout.add_canvas(avatar_canvas);

        stack_canvas.set_layout(stack_layout);
        canvas.set_left(stack_canvas);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.editing_context_changed(ui, ctx, project, server_ctx);
    }

    fn minimized(&mut self, _ui: &mut TheUI, ctx: &mut TheContext) {
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Tiles"),
            TheValue::Empty,
        ));
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;

        match event {
            TheEvent::Custom(id, value) => {
                if let TheValue::Id(tile_id) = value
                    && id.name == "Tile Picked"
                {
                    if let Some(tile) = project.tiles.get(tile_id) {
                        self.set_tile(tile, ui, ctx, server_ctx, false);
                    }
                    self.editing_context_changed(ui, ctx, project, server_ctx);
                } else if let TheValue::Id(tile_id) = value
                    && id.name == "Tile Updated"
                {
                    if let Some(tile) = project.tiles.get(tile_id) {
                        self.set_tile(tile, ui, ctx, server_ctx, true);

                        // Update the current frame
                        if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
                            if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node)
                            {
                                // Update the frame icon
                                if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                                    if server_ctx.curr_tile_frame_index < tile.textures.len() {
                                        widget.set_icon(
                                            server_ctx.curr_tile_frame_index,
                                            tile.textures[server_ctx.curr_tile_frame_index]
                                                .to_rgba(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                } else if id.name == "Editing Texture Updated" {
                    self.refresh_from_editing_context(project, ui, ctx, server_ctx);
                } else if id.name == "Tile Editor Undo Available" {
                    if let Some(atom) = TOOLLIST
                        .write()
                        .unwrap()
                        .get_current_editor_tool()
                        .get_undo_atom(project)
                    {
                        if let Some(atom) = atom.downcast_ref::<TileEditorUndoAtom>() {
                            self.add_undo(atom.clone(), ctx);
                        }
                    }
                }
            }
            TheEvent::ValueChanged(id, value) => {
                // The Size of the Tile has been edited
                if id.name == "Tile Size Edit" {
                    if let Some(size) = value.to_i32() {
                        if let Some(tile_id) = self.current_tile_id {
                            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                                if !tile.is_empty() {
                                    if size != tile.textures[0].width as i32 {
                                        let new_tile = tile.resized(size as usize, size as usize);
                                        let atom = TileEditorUndoAtom::TileEdit(
                                            tile.id,
                                            tile.clone(),
                                            new_tile.clone(),
                                        );
                                        *tile = new_tile;
                                        self.add_undo(atom, ctx);
                                        self.set_tile(tile, ui, ctx, server_ctx, false);
                                    }
                                }
                            }
                        }
                    }
                } else
                // The frame count of the Tile has been edited
                if id.name == "Tile Frame Edit" {
                    if let Some(frames) = value.to_i32() {
                        if let Some(tile_id) = self.current_tile_id {
                            if let Some(tile) = project.tiles.get_mut(&tile_id) {
                                if frames != tile.textures.len() as i32 {
                                    let mut new_tile = tile.clone();
                                    new_tile.set_frames(frames as usize);
                                    let atom = TileEditorUndoAtom::TileEdit(
                                        tile.id,
                                        tile.clone(),
                                        new_tile.clone(),
                                    );
                                    *tile = new_tile;
                                    self.add_undo(atom, ctx);
                                    self.set_tile(tile, ui, ctx, server_ctx, false);
                                }
                            }
                        }
                    }
                } else
                // The palette opacity has been edited
                if id.name == "Palette Opacity Edit" {
                    if let Some(opacity) = value.to_f32() {
                        server_ctx.palette_opacity = opacity;
                    }
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Tile Frame Icons" {
                    // New frame index selected - update the editor display
                    self.set_frame_index(*index as usize, project, ui, ctx, server_ctx);
                }
                // else if id.name == "Palette Item" {
                //     project.palette.current_index = *index as u16;
                // }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name == "Grid Enabled CB" {
                    self.show_grid = *state == TheWidgetState::Selected;
                    if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
                        && let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view()
                    {
                        rgba_view.set_dont_show_grid(!self.show_grid);
                        editor.relayout(ctx);
                    }
                    redraw = true;
                } else if *state == TheWidgetState::Selected && id.name.starts_with("Body: ") {
                    server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::None;
                    let color = match id.name.as_str() {
                        "Body: Skin Light" => Some([255, 0, 255, 255]),
                        "Body: Skin Dark" => Some([200, 0, 200, 255]),
                        "Body: Torso" => Some([0, 0, 255, 255]),
                        "Body: Legs" => Some([0, 255, 0, 255]),
                        "Body: Hair" => Some([255, 255, 0, 255]),
                        "Body: Eyes" => Some([0, 255, 255, 255]),
                        "Body: Hands" => Some([255, 128, 0, 255]),
                        "Body: Feet" => Some([255, 80, 0, 255]),
                        _ => None,
                    };
                    if let Some(c) = color {
                        server_ctx.body_marker_color = Some(c);
                        redraw = true;
                    }
                } else if *state == TheWidgetState::Selected && id.name == "Anchor: Main" {
                    server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::Main;
                    self.sync_anchor_overlay(project, ui, ctx, server_ctx);
                    redraw = true;
                } else if *state == TheWidgetState::Selected && id.name == "Anchor: Off" {
                    server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::Off;
                    self.sync_anchor_overlay(project, ui, ctx, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::TileZoomBy(id, delta) => {
                if id.name == "Tile Editor Dock RGBA Layout View" {
                    self.zoom += *delta * 0.5;
                    self.zoom = self.zoom.clamp(1.0, 60.0);
                    if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
                        editor.set_zoom(self.zoom);
                        editor.relayout(ctx);
                    }
                }
            }
            TheEvent::TileEditorHoverChanged(id, pos) => {
                if id.name == "Tile Editor Dock RGBA Layout View"
                    && self.paste_preview_texture.is_some()
                {
                    self.paste_preview_pos = Some(*pos);
                    self.sync_paste_preview(ui, ctx);
                    redraw = true;
                }
            }
            TheEvent::TileEditorClicked(id, coord) => {
                if id.name == "Tile Editor Dock RGBA Layout View"
                    && self.paste_preview_texture.is_some()
                {
                    self.paste_preview_pos = Some(*coord);
                    if self.apply_paste_at_preview(project, ui, ctx, server_ctx) {
                        self.clear_paste_preview(ui, ctx);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_tile_editor_paste_applied"),
                        ));
                    } else {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_tile_editor_paste_no_valid_target"),
                        ));
                    }
                    redraw = true;
                } else if id.name == "Tile Editor Dock RGBA Layout View"
                    && matches!(server_ctx.editing_ctx, PixelEditingContext::AvatarFrame(..))
                    && server_ctx.avatar_anchor_slot != AvatarAnchorEditSlot::None
                    && self.apply_avatar_anchor_at(*coord, project, ctx, server_ctx)
                {
                    self.sync_anchor_overlay(project, ui, ctx, server_ctx);
                    redraw = true;
                }
            }
            TheEvent::Copy => {
                if server_ctx.editing_ctx != PixelEditingContext::None {
                    if let Some(texture) = project.get_editing_texture(&server_ctx.editing_ctx) {
                        let selection = if let Some(editor) =
                            ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
                        {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                rgba_view.selection()
                            } else {
                                FxHashSet::default()
                            }
                        } else {
                            FxHashSet::default()
                        };

                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            if selection.is_empty() {
                                let img = arboard::ImageData {
                                    width: texture.width,
                                    height: texture.height,
                                    bytes: std::borrow::Cow::Borrowed(&texture.data),
                                };
                                let _ = clipboard.set_image(img);
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_tile_editor_copy_texture"),
                                ));
                            } else {
                                let min_x = selection.iter().map(|(x, _)| *x).min().unwrap_or(0);
                                let max_x = selection.iter().map(|(x, _)| *x).max().unwrap_or(0);
                                let min_y = selection.iter().map(|(_, y)| *y).min().unwrap_or(0);
                                let max_y = selection.iter().map(|(_, y)| *y).max().unwrap_or(0);

                                let out_w = (max_x - min_x + 1).max(1) as usize;
                                let out_h = (max_y - min_y + 1).max(1) as usize;
                                let mut out = vec![0_u8; out_w * out_h * 4];

                                for (x, y) in selection {
                                    if x >= 0
                                        && y >= 0
                                        && (x as usize) < texture.width
                                        && (y as usize) < texture.height
                                    {
                                        let src_i =
                                            ((y as usize) * texture.width + (x as usize)) * 4;
                                        let dx = (x - min_x) as usize;
                                        let dy = (y - min_y) as usize;
                                        let dst_i = (dy * out_w + dx) * 4;
                                        out[dst_i..dst_i + 4]
                                            .copy_from_slice(&texture.data[src_i..src_i + 4]);
                                    }
                                }

                                let img = arboard::ImageData {
                                    width: out_w,
                                    height: out_h,
                                    bytes: std::borrow::Cow::Owned(out),
                                };
                                let _ = clipboard.set_image(img);
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_tile_editor_copy_selection"),
                                ));
                            }
                        }
                    }
                }
            }
            TheEvent::Cut => {
                if server_ctx.editing_ctx != PixelEditingContext::None {
                    let selection =
                        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                rgba_view.selection()
                            } else {
                                FxHashSet::default()
                            }
                        } else {
                            FxHashSet::default()
                        };

                    if selection.is_empty() {
                        return redraw;
                    }

                    // Copy selected pixels first.
                    if let Some(texture) = project.get_editing_texture(&server_ctx.editing_ctx) {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let min_x = selection.iter().map(|(x, _)| *x).min().unwrap_or(0);
                            let max_x = selection.iter().map(|(x, _)| *x).max().unwrap_or(0);
                            let min_y = selection.iter().map(|(_, y)| *y).min().unwrap_or(0);
                            let max_y = selection.iter().map(|(_, y)| *y).max().unwrap_or(0);

                            let out_w = (max_x - min_x + 1).max(1) as usize;
                            let out_h = (max_y - min_y + 1).max(1) as usize;
                            let mut out = vec![0_u8; out_w * out_h * 4];

                            for (x, y) in &selection {
                                if *x >= 0
                                    && *y >= 0
                                    && (*x as usize) < texture.width
                                    && (*y as usize) < texture.height
                                {
                                    let src_i = ((*y as usize) * texture.width + (*x as usize)) * 4;
                                    let dx = (*x - min_x) as usize;
                                    let dy = (*y - min_y) as usize;
                                    let dst_i = (dy * out_w + dx) * 4;
                                    out[dst_i..dst_i + 4]
                                        .copy_from_slice(&texture.data[src_i..src_i + 4]);
                                }
                            }

                            let img = arboard::ImageData {
                                width: out_w,
                                height: out_h,
                                bytes: std::borrow::Cow::Owned(out),
                            };
                            let _ = clipboard.set_image(img);
                            ctx.ui.send(TheEvent::SetStatusText(
                                TheId::empty(),
                                fl!("status_tile_editor_cut_selection"),
                            ));
                        }
                    }

                    let editing_ctx = server_ctx.editing_ctx;
                    let before = project.get_editing_texture(&editing_ctx).cloned();
                    if let Some(texture) = project.get_editing_texture_mut(&editing_ctx) {
                        let before = if let Some(before) = before {
                            before
                        } else {
                            return redraw;
                        };
                        let mut changed = false;
                        for (x, y) in selection {
                            if x >= 0
                                && y >= 0
                                && (x as usize) < texture.width
                                && (y as usize) < texture.height
                            {
                                let i = ((y as usize) * texture.width + (x as usize)) * 4;
                                if texture.data[i..i + 4] != [0, 0, 0, 0] {
                                    texture.data[i..i + 4].copy_from_slice(&[0, 0, 0, 0]);
                                    changed = true;
                                }
                            }
                        }
                        if changed {
                            texture.generate_normals(true);
                            let after = texture.clone();
                            let atom = TileEditorUndoAtom::TextureEdit(editing_ctx, before, after);
                            self.add_undo(atom, ctx);

                            match editing_ctx {
                                PixelEditingContext::Tile(tile_id, _) => {
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Tile Updated"),
                                        TheValue::Id(tile_id),
                                    ));
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Update Tilepicker"),
                                        TheValue::Empty,
                                    ));
                                }
                                PixelEditingContext::AvatarFrame(..) => {
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Editing Texture Updated"),
                                        TheValue::Empty,
                                    ));
                                }
                                PixelEditingContext::None => {}
                            }

                            redraw = true;
                        }
                    }
                }
            }
            TheEvent::Paste(_, _) => {
                if server_ctx.editing_ctx != PixelEditingContext::None {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(img) = clipboard.get_image() {
                            // Convert RGBA image data to a texture
                            let width = img.width;
                            let height = img.height;
                            let data: Vec<u8> = img.bytes.into_owned();

                            if width > 0 && height > 0 {
                                let pasted = rusterix::Texture::new(data, width, height);
                                self.paste_preview_texture = Some(pasted);
                                if self.paste_preview_pos.is_none() {
                                    self.paste_preview_pos = Some(Vec2::zero());
                                }
                                self.sync_paste_preview(ui, ctx);
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_tile_editor_paste_preview_active"),
                                ));
                                redraw = true;
                            }
                        }
                    }
                }
            }
            TheEvent::KeyCodeDown(TheValue::KeyCode(key)) => {
                if *key == TheKeyCode::Escape && self.paste_preview_texture.is_some() {
                    self.clear_paste_preview(ui, ctx);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("status_tile_editor_paste_preview_canceled"),
                    ));
                    redraw = true;
                } else if *key == TheKeyCode::Return && self.paste_preview_texture.is_some() {
                    if self.apply_paste_at_preview(project, ui, ctx, server_ctx) {
                        self.clear_paste_preview(ui, ctx);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_tile_editor_paste_applied"),
                        ));
                        redraw = true;
                    } else {
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_tile_editor_paste_no_valid_target"),
                        ));
                    }
                } else if *key == TheKeyCode::Space && !ui.focus_widget_supports_text_input(ctx) {
                    if server_ctx.editing_ctx != PixelEditingContext::None {
                        self.anim_preview = !self.anim_preview;
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Update Minimap"),
                            TheValue::Empty,
                        ));
                        redraw = true;
                    }
                }
            }
            TheEvent::KeyDown(TheValue::Char(c)) => {
                if !ui.focus_widget_supports_text_input(ctx) {
                    let c = c.to_ascii_lowercase();
                    if c == 'h' {
                        if self.apply_flip(true, project, ui, ctx, server_ctx) {
                            redraw = true;
                        }
                    } else if c == 'v' && self.apply_flip(false, project, ui, ctx, server_ctx) {
                        redraw = true;
                    }
                }
            }
            _ => {}
        }

        redraw
    }

    fn supports_undo(&self) -> bool {
        true
    }

    fn has_changes(&self) -> bool {
        // Check if any tile has changes (index >= 0, meaning not fully undone)
        self.tile_undos.values().any(|undo| undo.has_changes())
    }

    fn undo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) {
        if let Some(key) = self.current_undo_key {
            if let Some(undo) = self.tile_undos.get_mut(&key) {
                undo.undo(project, ui, ctx);
                self.set_undo_state_to_ui(ctx);
            }
        }
    }

    fn redo(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        _server_ctx: &mut ServerContext,
    ) {
        if let Some(key) = self.current_undo_key {
            if let Some(undo) = self.tile_undos.get_mut(&key) {
                undo.redo(project, ui, ctx);
                self.set_undo_state_to_ui(ctx);
            }
        }
    }

    fn set_undo_state_to_ui(&self, ctx: &mut TheContext) {
        if let Some(key) = self.current_undo_key {
            if let Some(undo) = self.tile_undos.get(&key) {
                if undo.has_undo() {
                    ctx.ui.set_enabled("Undo");
                } else {
                    ctx.ui.set_disabled("Undo");
                }

                if undo.has_redo() {
                    ctx.ui.set_enabled("Redo");
                } else {
                    ctx.ui.set_disabled("Redo");
                }
                return;
            }
        }

        // No tile selected or no undo stack
        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");
    }

    fn editor_tools(&self) -> Option<Vec<Box<dyn EditorTool>>> {
        Some(vec![
            Box::new(TileDrawTool::new()),
            Box::new(TileSelectTool::new()),
        ])
    }

    fn draw_minimap(
        &self,
        buffer: &mut TheRGBABuffer,
        project: &Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        buffer.fill(BLACK);

        // Determine which frame to display
        let display_ctx = if self.anim_preview {
            let frame_count = server_ctx.editing_ctx.get_frame_count(project);
            if frame_count > 0 {
                let frame = server_ctx.animation_counter % frame_count;
                server_ctx.editing_ctx.with_frame(frame)
            } else {
                server_ctx.editing_ctx
            }
        } else {
            server_ctx.editing_ctx
        };

        if let Some(texture) = project.get_editing_texture(&display_ctx) {
            let stride: usize = buffer.stride();

            let src_pixels = &texture.data;
            let src_w = texture.width as f32;
            let src_h = texture.height as f32;

            let dim = buffer.dim();
            let dst_w = dim.width as f32;
            let dst_h = dim.height as f32;

            let scale = (dst_w / src_w).min(dst_h / src_h);
            let draw_w = src_w * scale;
            let draw_h = src_h * scale;

            let offset_x = ((dst_w - draw_w) * 0.5).round() as usize;
            let offset_y = ((dst_h - draw_h) * 0.5).round() as usize;

            let dst_rect = (
                offset_x,
                offset_y,
                draw_w.round() as usize,
                draw_h.round() as usize,
            );

            ctx.draw.blend_scale_chunk(
                buffer.pixels_mut(),
                &dst_rect,
                stride,
                src_pixels,
                &(src_w as usize, src_h as usize),
            );

            return true;
        }
        false
    }

    fn supports_minimap_animation(&self) -> bool {
        true
    }
}

impl TilesEditorDock {
    fn apply_avatar_anchor_at(
        &mut self,
        coord: Vec2<i32>,
        project: &mut Project,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) -> bool {
        let editing_ctx = server_ctx.editing_ctx;
        let Some(before) = project.get_editing_avatar_frame(&editing_ctx) else {
            return false;
        };
        let before_main = before.weapon_main_anchor;
        let before_off = before.weapon_off_anchor;

        let clicked = Some((coord.x as i16, coord.y as i16));
        if let Some(frame) = project.get_editing_avatar_frame_mut(&editing_ctx) {
            match server_ctx.avatar_anchor_slot {
                AvatarAnchorEditSlot::Main => {
                    if frame.weapon_main_anchor == clicked {
                        frame.weapon_main_anchor = None;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_avatar_anchor_clear_main"),
                        ));
                    } else {
                        frame.weapon_main_anchor = clicked;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_avatar_anchor_set_main"),
                        ));
                    }
                }
                AvatarAnchorEditSlot::Off => {
                    if frame.weapon_off_anchor == clicked {
                        frame.weapon_off_anchor = None;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_avatar_anchor_clear_off"),
                        ));
                    } else {
                        frame.weapon_off_anchor = clicked;
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_avatar_anchor_set_off"),
                        ));
                    }
                }
                AvatarAnchorEditSlot::None => return false,
            }

            let after_main = frame.weapon_main_anchor;
            let after_off = frame.weapon_off_anchor;
            if before_main != after_main || before_off != after_off {
                let atom = TileEditorUndoAtom::AvatarAnchorEdit(
                    editing_ctx,
                    before_main,
                    before_off,
                    after_main,
                    after_off,
                );
                self.add_undo(atom, ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Editing Texture Updated"),
                    TheValue::Empty,
                ));
                return true;
            }
        }
        false
    }

    fn sync_anchor_overlay(
        &mut self,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &ServerContext,
    ) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
            && let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view()
        {
            let points =
                if let Some(frame) = project.get_editing_avatar_frame(&server_ctx.editing_ctx) {
                    let mut p = vec![];
                    if let Some((x, y)) = frame.weapon_main_anchor {
                        p.push((Vec2::new(x as i32, y as i32), [255, 80, 80, 255]));
                    }
                    if let Some((x, y)) = frame.weapon_off_anchor {
                        p.push((Vec2::new(x as i32, y as i32), [80, 200, 255, 255]));
                    }
                    p
                } else {
                    vec![]
                };
            rgba_view.set_anchor_points(points);
            editor.relayout(ctx);
        }
    }

    fn apply_flip(
        &mut self,
        horizontal: bool,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        if self.paste_preview_texture.is_some() {
            return false;
        }

        let selection = if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                rgba_view.selection()
            } else {
                FxHashSet::default()
            }
        } else {
            FxHashSet::default()
        };

        let editing_ctx = server_ctx.editing_ctx;
        let before = project.get_editing_texture(&editing_ctx).cloned();
        let Some(texture) = project.get_editing_texture_mut(&editing_ctx) else {
            return false;
        };
        let Some(before) = before else {
            return false;
        };

        let mut after_data = texture.data.clone();
        let w = texture.width as i32;
        let h = texture.height as i32;

        if selection.is_empty() {
            for y in 0..h {
                for x in 0..w {
                    let sx = if horizontal { w - 1 - x } else { x };
                    let sy = if horizontal { y } else { h - 1 - y };
                    let src_i = ((sy as usize) * texture.width + (sx as usize)) * 4;
                    let dst_i = ((y as usize) * texture.width + (x as usize)) * 4;
                    after_data[dst_i..dst_i + 4].copy_from_slice(&texture.data[src_i..src_i + 4]);
                }
            }
        } else {
            let min_x = selection.iter().map(|(x, _)| *x).min().unwrap_or(0);
            let max_x = selection.iter().map(|(x, _)| *x).max().unwrap_or(0);
            let min_y = selection.iter().map(|(_, y)| *y).min().unwrap_or(0);
            let max_y = selection.iter().map(|(_, y)| *y).max().unwrap_or(0);

            for (x, y) in &selection {
                let sx = if horizontal { min_x + (max_x - *x) } else { *x };
                let sy = if horizontal { *y } else { min_y + (max_y - *y) };
                if sx >= 0
                    && sy >= 0
                    && sx < w
                    && sy < h
                    && selection.contains(&(sx, sy))
                    && *x >= 0
                    && *y >= 0
                    && *x < w
                    && *y < h
                {
                    let src_i = ((sy as usize) * texture.width + (sx as usize)) * 4;
                    let dst_i = ((*y as usize) * texture.width + (*x as usize)) * 4;
                    after_data[dst_i..dst_i + 4].copy_from_slice(&texture.data[src_i..src_i + 4]);
                }
            }
        }

        if after_data == texture.data {
            return false;
        }

        texture.data = after_data;
        texture.generate_normals(true);

        let after = texture.clone();
        let atom = TileEditorUndoAtom::TextureEdit(editing_ctx, before, after);
        self.add_undo(atom, ctx);

        match editing_ctx {
            PixelEditingContext::Tile(tile_id, _) => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Tile Updated"),
                    TheValue::Id(tile_id),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Tilepicker"),
                    TheValue::Empty,
                ));
            }
            PixelEditingContext::AvatarFrame(..) => {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Editing Texture Updated"),
                    TheValue::Empty,
                ));
            }
            PixelEditingContext::None => {}
        }
        true
    }

    /// Switch to a different tile and update undo button states
    pub fn switch_to_tile(
        &mut self,
        tile: &rusterix::Tile,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        self.current_tile_id = Some(tile.id);
        self.current_undo_key = Some(tile.id);

        // Verify frame index is valid for the new tile
        if server_ctx.curr_tile_frame_index >= tile.textures.len() {
            server_ctx.curr_tile_frame_index = 0;
        }

        server_ctx.editing_ctx =
            PixelEditingContext::Tile(tile.id, server_ctx.curr_tile_frame_index);

        self.set_undo_state_to_ui(ctx);
    }

    /// Set the current frame/texture index
    pub fn set_frame_index(
        &mut self,
        index: usize,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        // Verify the index is valid for current tile
        if let Some(tile_id) = self.current_tile_id {
            if let Some(tile) = project.tiles.get(&tile_id) {
                if index < tile.textures.len() {
                    server_ctx.curr_tile_frame_index = index;
                    server_ctx.editing_ctx = PixelEditingContext::Tile(tile_id, index);

                    // Update the TreeIcons selection
                    if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
                        if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node) {
                            if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                                widget.set_selected_index(Some(index));
                            }
                        }
                    }

                    // Refresh the display with the new frame
                    self.update_editor_display(tile, ui, ctx, server_ctx);
                    self.sync_anchor_overlay(project, ui, ctx, server_ctx);
                }
            }
        }
    }

    /// Update just the editor display (for when frame index changes)
    fn update_editor_display(
        &mut self,
        tile: &rusterix::Tile,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let frame_index = server_ctx
                    .curr_tile_frame_index
                    .min(tile.textures.len().saturating_sub(1));

                if frame_index < tile.textures.len() {
                    let buffer = tile.textures[frame_index].to_rgba();
                    let icon_width = tile.textures[frame_index].width;
                    let icon_height = tile.textures[frame_index].height;

                    self.zoom = (view_width as f32 / icon_width as f32)
                        .min(view_height as f32 / icon_height as f32);

                    rgba_view.set_buffer(buffer);
                    editor.set_zoom(self.zoom);
                    editor.relayout(ctx);
                }
            }
        }
    }

    /// Update the frame icons in the tree (call after editing a texture)
    pub fn update_frame_icons(&self, tile: &rusterix::Tile, ui: &mut TheUI) {
        if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
            if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node) {
                if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                    // Update all frame icons
                    for (index, texture) in tile.textures.iter().enumerate() {
                        widget.set_icon(index, texture.to_rgba());
                    }
                }
            }
        }
    }

    /// Add an undo atom to the appropriate undo stack (keyed by context)
    pub fn add_undo(&mut self, atom: TileEditorUndoAtom, ctx: &mut TheContext) {
        let key = match &atom {
            TileEditorUndoAtom::TileEdit(tile_id, _, _) => Some(*tile_id),
            TileEditorUndoAtom::TextureEdit(editing_ctx, _, _) => match editing_ctx {
                PixelEditingContext::Tile(tile_id, _) => Some(*tile_id),
                PixelEditingContext::AvatarFrame(avatar_id, _, _, _) => Some(*avatar_id),
                PixelEditingContext::None => None,
            },
            TileEditorUndoAtom::AvatarAnchorEdit(editing_ctx, _, _, _, _) => match editing_ctx {
                PixelEditingContext::Tile(tile_id, _) => Some(*tile_id),
                PixelEditingContext::AvatarFrame(avatar_id, _, _, _) => Some(*avatar_id),
                PixelEditingContext::None => None,
            },
        };
        if let Some(key) = key {
            let undo = self
                .tile_undos
                .entry(key)
                .or_insert_with(TileEditorUndo::new);
            undo.add(atom);
            undo.truncate_to_limit(self.max_undo);
            self.set_undo_state_to_ui(ctx);
        }
    }

    fn sync_paste_preview(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout")
            && let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view()
        {
            if let (Some(texture), Some(pos)) =
                (&self.paste_preview_texture, self.paste_preview_pos)
            {
                rgba_view.set_paste_preview(Some((texture.to_rgba(), pos)));
            } else {
                rgba_view.set_paste_preview(None);
            }
            editor.relayout(ctx);
        }
    }

    fn clear_paste_preview(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.paste_preview_texture = None;
        self.paste_preview_pos = None;
        self.sync_paste_preview(ui, ctx);
    }

    fn apply_paste_at_preview(
        &mut self,
        project: &mut Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let Some(pasted) = self.paste_preview_texture.clone() else {
            return false;
        };
        let Some(anchor) = self.paste_preview_pos else {
            return false;
        };

        let selection = if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                rgba_view.selection()
            } else {
                FxHashSet::default()
            }
        } else {
            FxHashSet::default()
        };

        let editing_ctx = server_ctx.editing_ctx;
        let before = project.get_editing_texture(&editing_ctx).cloned();
        if let Some(texture) = project.get_editing_texture_mut(&editing_ctx) {
            let before = if let Some(before) = before {
                before
            } else {
                return false;
            };
            let mut changed = false;

            if selection.is_empty() {
                for sy in 0..pasted.height {
                    for sx in 0..pasted.width {
                        let tx = anchor.x + sx as i32;
                        let ty = anchor.y + sy as i32;
                        if tx >= 0
                            && ty >= 0
                            && (tx as usize) < texture.width
                            && (ty as usize) < texture.height
                        {
                            let src_i = (sy * pasted.width + sx) * 4;
                            let dst_i = ((ty as usize) * texture.width + (tx as usize)) * 4;
                            texture.data[dst_i..dst_i + 4]
                                .copy_from_slice(&pasted.data[src_i..src_i + 4]);
                            changed = true;
                        }
                    }
                }
            } else {
                for sy in 0..pasted.height {
                    for sx in 0..pasted.width {
                        let tx = anchor.x + sx as i32;
                        let ty = anchor.y + sy as i32;
                        if tx >= 0
                            && ty >= 0
                            && (tx as usize) < texture.width
                            && (ty as usize) < texture.height
                            && selection.contains(&(tx, ty))
                        {
                            let src_i = (sy * pasted.width + sx) * 4;
                            let dst_i = ((ty as usize) * texture.width + (tx as usize)) * 4;
                            texture.data[dst_i..dst_i + 4]
                                .copy_from_slice(&pasted.data[src_i..src_i + 4]);
                            changed = true;
                        }
                    }
                }
            }

            if !changed {
                return false;
            }

            texture.generate_normals(true);
            let after = texture.clone();
            let atom = TileEditorUndoAtom::TextureEdit(editing_ctx, before, after);
            self.add_undo(atom, ctx);

            match editing_ctx {
                PixelEditingContext::Tile(tile_id, _) => {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Tile Updated"),
                        TheValue::Id(tile_id),
                    ));
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Tilepicker"),
                        TheValue::Empty,
                    ));
                }
                PixelEditingContext::AvatarFrame(..) => {
                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Editing Texture Updated"),
                        TheValue::Empty,
                    ));
                }
                PixelEditingContext::None => {}
            }
            return true;
        }
        false
    }

    /// Set the tile for the editor.
    pub fn set_tile(
        &mut self,
        tile: &rusterix::Tile,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
        update_only: bool,
    ) {
        // Switch to this tile's undo stack
        if !update_only {
            self.switch_to_tile(tile, ctx, server_ctx);

            if let Some(tree_layout) = ui.get_tree_layout("Tile Editor Tree") {
                if let Some(tile_node) = tree_layout.get_node_by_id_mut(&self.tile_node) {
                    // Set the tile size
                    if let Some(widget) = tile_node.widgets[0].as_tree_item() {
                        if let Some(embedded) = widget.embedded_widget_mut() {
                            if !tile.is_empty() {
                                embedded.set_value(TheValue::Int(tile.textures[0].width as i32));
                            }
                        }
                    }
                    // Set the frames editor
                    if let Some(widget) = tile_node.widgets[1].as_tree_item() {
                        if let Some(embedded) = widget.embedded_widget_mut() {
                            if !tile.is_empty() {
                                embedded.set_value(TheValue::Int(tile.textures.len() as i32));
                            }
                        }
                    }
                    // Set the frames editor
                    if let Some(widget) = tile_node.widgets[2].as_tree_icons() {
                        widget.set_icon_count(tile.textures.len());
                        for (index, texture) in tile.textures.iter().enumerate() {
                            widget.set_icon(index, texture.to_rgba());
                        }
                    }
                }
            }
        }

        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                // Use current frame index, ensure it's valid
                let frame_index = server_ctx
                    .curr_tile_frame_index
                    .min(tile.textures.len().saturating_sub(1));

                if frame_index < tile.textures.len() {
                    let buffer = tile.textures[frame_index].to_rgba();

                    if !update_only {
                        rgba_view.set_grid(Some(1));
                        rgba_view.set_dont_show_grid(!self.show_grid);

                        let icon_width = tile.textures[frame_index].width;
                        let icon_height = tile.textures[frame_index].height;

                        self.zoom = (view_width as f32 / icon_width as f32)
                            .min(view_height as f32 / icon_height as f32);
                    }
                    rgba_view.set_buffer(buffer);
                }
            }
            if !update_only {
                editor.set_zoom(self.zoom);
                editor.relayout(ctx);
            }
        }
    }

    /// Called whenever the editing context changes (activate, tile picked, avatar frame selected).
    /// Use this to adjust UI elements based on the current PixelEditingContext.
    pub fn editing_context_changed(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        if self.paste_preview_texture.is_some() {
            self.clear_paste_preview(ui, ctx);
        }
        match server_ctx.editing_ctx {
            PixelEditingContext::Tile(tile_id, _) => {
                server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::None;
                if let Some(tile) = project.tiles.get(&tile_id) {
                    self.set_tile(tile, ui, ctx, server_ctx, false);
                    if let Some(stack) = ui.get_stack_layout("Pixel Editor Stack Layout") {
                        stack.set_index(0);
                    }
                }
            }
            PixelEditingContext::AvatarFrame(..) => {
                self.set_undo_key_from_context(&server_ctx.editing_ctx);
                self.refresh_from_editing_context(project, ui, ctx, server_ctx);
                if let Some(stack) = ui.get_stack_layout("Pixel Editor Stack Layout") {
                    stack.set_index(1);
                }
            }
            PixelEditingContext::None => {
                server_ctx.avatar_anchor_slot = AvatarAnchorEditSlot::None;
                if let Some(tile_id) = server_ctx.curr_tile_id {
                    if let Some(tile) = project.tiles.get(&tile_id) {
                        self.set_tile(tile, ui, ctx, server_ctx, false);
                    }
                }
            }
        }
        self.sync_anchor_overlay(project, ui, ctx, server_ctx);
    }

    /// Set the undo key based on the current editing context.
    pub fn set_undo_key_from_context(&mut self, editing_ctx: &PixelEditingContext) {
        self.current_undo_key = match editing_ctx {
            PixelEditingContext::None => None,
            PixelEditingContext::Tile(tile_id, _) => Some(*tile_id),
            PixelEditingContext::AvatarFrame(avatar_id, _, _, _) => Some(*avatar_id),
        };
    }

    /// Refresh the editor display from the current editing context.
    pub fn refresh_from_editing_context(
        &mut self,
        project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        if let Some(texture) = project.get_editing_texture(&server_ctx.editing_ctx) {
            self.set_editing_texture(texture, ui, ctx);
        }
        self.sync_anchor_overlay(project, ui, ctx, server_ctx);
    }

    /// Display the given texture in the editor.
    pub fn set_editing_texture(
        &mut self,
        texture: &rusterix::Texture,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        if let Some(editor) = ui.get_rgba_layout("Tile Editor Dock RGBA Layout") {
            let view_width = editor.dim().width - 16;
            let view_height = editor.dim().height - 16;

            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                let buffer = texture.to_rgba();
                let icon_width = texture.width;
                let icon_height = texture.height;

                self.zoom = (view_width as f32 / icon_width as f32)
                    .min(view_height as f32 / icon_height as f32);

                rgba_view.set_grid(Some(1));
                rgba_view.set_dont_show_grid(!self.show_grid);
                rgba_view.set_buffer(buffer);
                editor.set_zoom(self.zoom);
                editor.relayout(ctx);
            }
        }
    }
}
