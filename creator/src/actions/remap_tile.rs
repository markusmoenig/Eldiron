use crate::editor::{DOCKMANAGER, UNDOMANAGER};
use crate::prelude::*;

const REMAP_ALL_ID: &str = "actionRemapAll";
const REMAP_MODE_ID: &str = "actionRemapMode";

#[derive(Clone, Copy)]
enum RemapMode {
    Nearest,
    FloydSteinberg,
    Bayer4x4,
    Exact,
}

impl RemapMode {
    fn from_index(index: i32) -> Self {
        match index {
            1 => Self::FloydSteinberg,
            2 => Self::Bayer4x4,
            3 => Self::Exact,
            _ => Self::Nearest,
        }
    }
}

pub struct RemapTile {
    id: TheId,
    nodeui: TheNodeUI,
}

fn find_exact_color_index(palette: &ThePalette, color: &TheColor) -> Option<usize> {
    palette
        .colors
        .iter()
        .position(|entry| entry.as_ref() == Some(color))
}

fn nearest_palette_color_u8(palette: &ThePalette, color: [u8; 4]) -> Option<[u8; 4]> {
    let color = TheColor::from(color);
    let index = palette.find_closest_color_index(&color)?;
    palette
        .colors
        .get(index)
        .and_then(|entry| entry.as_ref())
        .map(TheColor::to_u8_array)
}

fn remap_texture_nearest(tex: &mut rusterix::Texture, palette: &ThePalette) {
    for y in 0..tex.height {
        for x in 0..tex.width {
            let mut col = tex.get_pixel(x as u32, y as u32);
            if col[3] == 0 {
                continue;
            }
            if let Some(mapped) = nearest_palette_color_u8(palette, col) {
                col = mapped;
                tex.set_pixel(x as u32, y as u32, col);
            }
        }
    }
}

fn remap_texture_exact(tex: &mut rusterix::Texture, palette: &ThePalette) {
    for y in 0..tex.height {
        for x in 0..tex.width {
            let mut col = tex.get_pixel(x as u32, y as u32);
            if col[3] == 0 {
                continue;
            }
            let color = TheColor::from(col);
            if let Some(index) = find_exact_color_index(palette, &color)
                && let Some(c) = palette.colors.get(index)
                && let Some(c) = c
            {
                col = c.to_u8_array();
                tex.set_pixel(x as u32, y as u32, col);
            }
        }
    }
}

fn remap_texture_floyd_steinberg(tex: &mut rusterix::Texture, palette: &ThePalette) {
    let width = tex.width as usize;
    let height = tex.height as usize;
    let mut work = vec![[0.0f32; 4]; width * height];

    for y in 0..height {
        for x in 0..width {
            let col = tex.get_pixel(x as u32, y as u32);
            work[y * width + x] = [
                col[0] as f32,
                col[1] as f32,
                col[2] as f32,
                col[3] as f32,
            ];
        }
    }

    let diffuse = |work: &mut [[f32; 4]], x: usize, y: usize, dx: isize, dy: isize, err: [f32; 3], factor: f32| {
        let nx = x as isize + dx;
        let ny = y as isize + dy;
        if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
            return;
        }
        let idx = ny as usize * width + nx as usize;
        if work[idx][3] == 0.0 {
            return;
        }
        work[idx][0] = (work[idx][0] + err[0] * factor).clamp(0.0, 255.0);
        work[idx][1] = (work[idx][1] + err[1] * factor).clamp(0.0, 255.0);
        work[idx][2] = (work[idx][2] + err[2] * factor).clamp(0.0, 255.0);
    };

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if work[idx][3] == 0.0 {
                continue;
            }
            let source = [
                work[idx][0].round().clamp(0.0, 255.0) as u8,
                work[idx][1].round().clamp(0.0, 255.0) as u8,
                work[idx][2].round().clamp(0.0, 255.0) as u8,
                work[idx][3].round().clamp(0.0, 255.0) as u8,
            ];

            if let Some(mapped) = nearest_palette_color_u8(palette, source) {
                tex.set_pixel(x as u32, y as u32, [mapped[0], mapped[1], mapped[2], source[3]]);
                let err = [
                    source[0] as f32 - mapped[0] as f32,
                    source[1] as f32 - mapped[1] as f32,
                    source[2] as f32 - mapped[2] as f32,
                ];
                diffuse(&mut work, x, y, 1, 0, err, 7.0 / 16.0);
                diffuse(&mut work, x, y, -1, 1, err, 3.0 / 16.0);
                diffuse(&mut work, x, y, 0, 1, err, 5.0 / 16.0);
                diffuse(&mut work, x, y, 1, 1, err, 1.0 / 16.0);
            }
        }
    }
}

fn remap_texture_bayer4x4(tex: &mut rusterix::Texture, palette: &ThePalette) {
    const BAYER_4X4: [[f32; 4]; 4] = [
        [0.0, 8.0, 2.0, 10.0],
        [12.0, 4.0, 14.0, 6.0],
        [3.0, 11.0, 1.0, 9.0],
        [15.0, 7.0, 13.0, 5.0],
    ];

    for y in 0..tex.height {
        for x in 0..tex.width {
            let mut col = tex.get_pixel(x as u32, y as u32);
            if col[3] == 0 {
                continue;
            }

            let threshold = (BAYER_4X4[(y as usize) % 4][(x as usize) % 4] / 16.0) - 0.5;
            let offset = threshold * 48.0;
            let adjusted = [
                (col[0] as f32 + offset).clamp(0.0, 255.0) as u8,
                (col[1] as f32 + offset).clamp(0.0, 255.0) as u8,
                (col[2] as f32 + offset).clamp(0.0, 255.0) as u8,
                col[3],
            ];

            if let Some(mapped) = nearest_palette_color_u8(palette, adjusted) {
                col = [mapped[0], mapped[1], mapped[2], col[3]];
                tex.set_pixel(x as u32, y as u32, col);
            }
        }
    }
}

fn remap_tile_to_palette(tile: &mut rusterix::Tile, palette: &ThePalette, mode: RemapMode) -> bool {
    let prev = tile.clone();

    for tex in &mut tile.textures {
        match mode {
            RemapMode::Nearest => remap_texture_nearest(tex, palette),
            RemapMode::FloydSteinberg => remap_texture_floyd_steinberg(tex, palette),
            RemapMode::Bayer4x4 => remap_texture_bayer4x4(tex, palette),
            RemapMode::Exact => remap_texture_exact(tex, palette),
        }
    }

    if prev.textures == tile.textures {
        return false;
    }

    for tex in &mut tile.textures {
        tex.generate_normals(true);
    }

    true
}

impl Action for RemapTile {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Selector(
            REMAP_MODE_ID.into(),
            "".into(),
            "".into(),
            vec![
                "nearest".into(),
                "floyd-steinberg".into(),
                "bayer-4x4".into(),
                "exact".into(),
            ],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            REMAP_ALL_ID.into(),
            "".into(),
            "".into(),
            false,
        ));

        Self {
            id: TheId::named(&fl!("action_remap_tile")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_remap_tile_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Dock
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        DOCKMANAGER.read().unwrap().dock == "Tiles" && server_ctx.curr_tile_id.is_some()
    }

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        let remap_all = self.nodeui.get_bool_value(REMAP_ALL_ID).unwrap_or(false);
        let remap_mode = RemapMode::from_index(self.nodeui.get_i32_value(REMAP_MODE_ID).unwrap_or(0));
        let palette = project.palette.clone();

        if remap_all {
            let mut edits = Vec::new();

            for tile in project.tiles.values_mut() {
                let prev = tile.clone();
                if remap_tile_to_palette(tile, &palette, remap_mode) {
                    edits.push((prev, tile.clone()));
                }
            }

            if !edits.is_empty() {
                UNDOMANAGER
                    .write()
                    .unwrap()
                    .add_undo(ProjectUndoAtom::TileBatchEdit(edits), ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Tiles"),
                    TheValue::Empty,
                ));
            }
        } else if let Some(tile_id) = server_ctx.curr_tile_id
            && let Some(tile) = project.tiles.get_mut(&tile_id)
        {
            let prev = tile.clone();
            if remap_tile_to_palette(tile, &palette, remap_mode) {
                UNDOMANAGER
                    .write()
                    .unwrap()
                    .add_undo(ProjectUndoAtom::TileEdit(prev, tile.clone()), ctx);
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
