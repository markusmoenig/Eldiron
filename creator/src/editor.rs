use crate::Embedded;
use crate::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::scepter::{ScepterEvent, ScepterRegionRequest, ScepterService};
#[cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use crate::self_update::{SelfUpdateEvent, SelfUpdater};
#[cfg(not(target_arch = "wasm32"))]
use eldiron_scepter::{
    AttributesGet, AttributesPatch, GridPoint, RegionPaintCells, RegionPaintRect, RegionRef,
    RegionRenderPreview, ScriptGet, ScriptPatch, ScriptTarget, ScriptTargetKind, TileSelector,
};
use rusterix::render_settings::RendererBackend;
use rusterix::server::message::AudioCommand;
use rusterix::{
    PlayerCamera, Rusterix, SceneManager, SceneManagerResult, Texture, Value, ValueContainer,
};
#[cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use self_update::update::Release;
use shared::rusterix_utils::*;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::Receiver;
#[cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use std::sync::{
    Arc, Mutex,
    mpsc::{Sender, channel},
};

#[cfg(all(
    feature = "self-update",
    any(target_os = "windows", target_os = "linux", target_os = "macos")
))]
use std::thread;

pub static PREVIEW_ICON: LazyLock<RwLock<(TheRGBATile, i32)>> =
    LazyLock::new(|| RwLock::new((TheRGBATile::default(), 0)));

pub static SIDEBARMODE: LazyLock<RwLock<SidebarMode>> =
    LazyLock::new(|| RwLock::new(SidebarMode::Region));
pub static UNDOMANAGER: LazyLock<RwLock<UndoManager>> =
    LazyLock::new(|| RwLock::new(UndoManager::default()));
pub static TOOLLIST: LazyLock<RwLock<ToolList>> =
    LazyLock::new(|| RwLock::new(ToolList::default()));
pub static ACTIONLIST: LazyLock<RwLock<ActionList>> =
    LazyLock::new(|| RwLock::new(ActionList::default()));
// pub static PANELS: LazyLock<RwLock<Panels>> = LazyLock::new(|| RwLock::new(Panels::new()));
pub static PALETTE: LazyLock<RwLock<ThePalette>> =
    LazyLock::new(|| RwLock::new(ThePalette::default()));
pub static RUSTERIX: LazyLock<RwLock<Rusterix>> =
    LazyLock::new(|| RwLock::new(Rusterix::default()));
pub static CONFIGEDITOR: LazyLock<RwLock<ConfigEditor>> =
    LazyLock::new(|| RwLock::new(ConfigEditor::new()));
pub static CONFIG: LazyLock<RwLock<toml::Table>> =
    LazyLock::new(|| RwLock::new(toml::Table::default()));
pub static EDITCAMERA: LazyLock<RwLock<EditCamera>> =
    LazyLock::new(|| RwLock::new(EditCamera::new()));
pub static SCENEMANAGER: LazyLock<RwLock<SceneManager>> =
    LazyLock::new(|| RwLock::new(SceneManager::default()));
pub static DOCKMANAGER: LazyLock<RwLock<DockManager>> =
    LazyLock::new(|| RwLock::new(DockManager::default()));
pub static TEXTGAME: LazyLock<RwLock<TextGameState>> =
    LazyLock::new(|| RwLock::new(TextGameState::default()));

#[derive(Clone)]
struct ProjectSession {
    project: Project,
    project_path: Option<PathBuf>,
    undo: UndoManager,
    dirty: bool,
}

#[derive(Default)]
struct IsoPaintStrokeRenderCache {
    origin: [i32; 2],
    screen_anchor: Option<[i32; 2]>,
    world_anchor: Option<[f32; 3]>,
    camera_scale: Option<f32>,
    brush: String,
    clip: String,
    material_id: u8,
    color: [u8; 4],
    pattern_kind: String,
    pattern_scale: f32,
    pattern_mortar: f32,
    pattern_detail: f32,
    pattern_variation: f32,
    erase: bool,
    buffer: TheRGBABuffer,
}

#[derive(Default)]
struct IsoPaintChunkRenderCache {
    revision: u64,
    strokes: Vec<IsoPaintStrokeRenderCache>,
}

#[derive(Default)]
struct IsoPaintRenderCache {
    region_id: Option<Uuid>,
    chunks: HashMap<String, IsoPaintChunkRenderCache>,
    prepared_key: Option<IsoPaintPreparedOverlayKey>,
    prepared_overlay: Option<IsoPaintPreparedOverlay>,
    uploaded_key: Option<IsoPaintPreparedOverlayKey>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IsoPaintPreparedOverlayKey {
    region_id: Uuid,
    width: i32,
    height: i32,
    layer_key: u64,
    surface_key: u64,
    camera_scale_bits: u32,
}

#[derive(Clone)]
struct IsoPaintPreparedOverlay {
    width: u32,
    height: u32,
    color_rgba: Vec<u8>,
    material_rgba: Vec<u8>,
}

#[derive(Deserialize, Clone)]
struct StarterProjectManifest {
    #[serde(default)]
    starter: Vec<StarterProjectManifestEntry>,
}

#[derive(Deserialize, Clone)]
struct StarterProjectManifestEntry {
    id: String,
    title: String,
    description: String,
    project_path: String,
    image: String,
}

#[derive(Clone)]
struct StarterProjectEntry {
    id: Uuid,
    manifest_id: String,
    title: String,
    description: String,
    project_path: String,
    preview: Option<TheRGBATile>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct CreatorWindowState {
    x: Option<i32>,
    y: Option<i32>,
    width: Option<usize>,
    height: Option<usize>,
}

pub struct Editor {
    project: Project,
    project_path: Option<PathBuf>,
    sessions: Vec<ProjectSession>,
    active_session: usize,
    replace_next_project_load_in_active_tab: bool,
    last_active_dirty: bool,

    sidebar: Sidebar,
    mapeditor: MapEditor,

    server_ctx: ServerContext,

    update_tracker: UpdateTracker,
    event_receiver: Option<Receiver<TheEvent>>,
    #[cfg(not(target_arch = "wasm32"))]
    scepter_receiver: Option<Receiver<ScepterEvent>>,
    last_3d_hover_redraw_at: Option<std::time::Instant>,

    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    self_update_rx: Receiver<SelfUpdateEvent>,
    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    self_update_tx: Sender<SelfUpdateEvent>,
    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    self_updater: Arc<Mutex<SelfUpdater>>,

    update_counter: usize,
    last_processed_log_len: usize,
    pending_game_messages: Vec<rusterix::server::Message>,
    pending_game_says: Vec<TextGameSay>,
    pending_game_choices: Vec<rusterix::MultipleChoice>,
    pending_text_game_command: Option<(String, String)>,
    pending_text_game_runtime_flush: bool,

    build_values: ValueContainer,
    window_state: CreatorWindowState,
    starter_projects: Vec<StarterProjectEntry>,
    starter_project_cache: HashMap<String, Project>,
    starter_manifest_cache: Option<Vec<StarterProjectEntry>>,
    starter_loader_rx: Option<Receiver<Vec<StarterProjectEntry>>>,
    selected_starter_manifest_id: Option<String>,
    iso_paint_render_cache: IsoPaintRenderCache,
}

impl Editor {
    const PROJECT_EXTENSION: &'static str = "eldiron";
    const STARTER_REPO_RAW_BASE: &'static str =
        "https://raw.githubusercontent.com/markusmoenig/Eldiron/master/";
    const STARTER_LIST_ID: &'static str = "Starter Project List";
    const STARTER_PREVIEW_ID: &'static str = "Starter Project Preview";
    const STARTER_CREATE_ID: &'static str = "Starter Project Create";
    const STARTER_CANCEL_ID: &'static str = "Starter Project Cancel";

    fn iso_paint_color_with_opacity(mut color: [u8; 4], opacity: f32) -> [u8; 4] {
        color[3] = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        color
    }

    fn iso_paint_material_pixel(material_id: u8) -> [u8; 4] {
        [254, material_id, 0, 0]
    }

    fn iso_paint_set_material_pixel(
        material_pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        material_id: u8,
    ) {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        if index + 3 >= material_pixels.len() {
            return;
        }
        let material = Self::iso_paint_material_pixel(material_id);
        material_pixels[index..index + 4].copy_from_slice(&material);
    }

    fn iso_paint_clear_material_pixel(
        color_pixels: &[u8],
        material_pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
    ) {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        if index + 3 >= color_pixels.len() || index + 3 >= material_pixels.len() {
            return;
        }
        if color_pixels[index + 3] == 0 {
            material_pixels[index..index + 4].copy_from_slice(&Self::iso_paint_material_pixel(0));
        }
    }

    fn iso_paint_blend_pixel(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        x: i32,
        y: i32,
        color: [u8; 4],
    ) {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= height || color[3] == 0 {
            return;
        }
        let index = (y as usize * width + x as usize) * 4;
        if index + 3 >= pixels.len() {
            return;
        }

        let src_a = color[3] as u16;
        let inv_a = 255 - src_a;
        pixels[index] = ((color[0] as u16 * src_a + pixels[index] as u16 * inv_a) / 255) as u8;
        pixels[index + 1] =
            ((color[1] as u16 * src_a + pixels[index + 1] as u16 * inv_a) / 255) as u8;
        pixels[index + 2] =
            ((color[2] as u16 * src_a + pixels[index + 2] as u16 * inv_a) / 255) as u8;
        pixels[index + 3] = (src_a + (pixels[index + 3] as u16 * inv_a) / 255).min(255) as u8;
    }

    fn iso_paint_stamp(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        local_x: i32,
        local_y: i32,
        radius: i32,
        color: [u8; 4],
    ) {
        let radius = radius.max(1);
        let radius_sq = radius * radius;
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                if ox * ox + oy * oy > radius_sq {
                    continue;
                }
                Self::iso_paint_blend_pixel(
                    pixels,
                    width,
                    height,
                    local_x + ox,
                    local_y + oy,
                    color,
                );
            }
        }
    }

    fn iso_paint_draw_segment(
        pixels: &mut [u8],
        width: usize,
        height: usize,
        a: [i32; 2],
        b: [i32; 2],
        origin: [i32; 2],
        radius: i32,
        color: [u8; 4],
    ) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let steps = dx.abs().max(dy.abs()).max(1);
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let x = (a[0] as f32 + dx as f32 * t).round() as i32;
            let y = (a[1] as f32 + dy as f32 * t).round() as i32;
            Self::iso_paint_stamp(
                pixels,
                width,
                height,
                x - origin[0],
                y - origin[1],
                radius,
                color,
            );
        }
    }

    fn iso_paint_sample_brick_color(
        pattern_x: f32,
        pattern_y: f32,
        base: [u8; 4],
        pattern_kind: &str,
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
    ) -> [u8; 4] {
        let pattern_scale = pattern_scale.clamp(0.25, 4.0);
        let pattern_mortar = pattern_mortar.clamp(0.0, 0.4);
        let pattern_detail = pattern_detail.clamp(0.0, 1.0);
        let pattern_variation = pattern_variation.clamp(0.0, 1.0);
        let staggered = !matches!(pattern_kind, "tile" | "tiles");
        let brick_w = if staggered { 34.0 } else { 24.0 } * pattern_scale;
        let brick_h = if staggered { 17.0 } else { 24.0 } * pattern_scale;
        let mortar =
            (brick_w.min(brick_h) * pattern_mortar).clamp(0.0, brick_w.min(brick_h) * 0.45);

        let row = (pattern_y / brick_h).floor();
        let offset_x = if staggered && row as i32 & 1 != 0 {
            brick_w * 0.5
        } else {
            0.0
        };
        let local_x = (pattern_x + offset_x).rem_euclid(brick_w);
        let local_y = pattern_y.rem_euclid(brick_h);
        let col = ((pattern_x + offset_x) / brick_w).floor() as i32;
        let row_i = row as i32;

        let hash = |x: i32, y: i32, salt: i32| -> f32 {
            let mut n = x
                .wrapping_mul(374_761_393)
                .wrapping_add(y.wrapping_mul(668_265_263))
                .wrapping_add(salt.wrapping_mul(2_147_483_647));
            n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
            ((n ^ (n >> 16)) & 0xffff) as f32 / 65_535.0
        };

        if local_x < mortar || local_y < mortar {
            return [base[0], base[1], base[2], 0];
        }

        let edge_distance = local_x
            .min(local_y)
            .min(brick_w - local_x)
            .min(brick_h - local_y);
        let edge_wear = if edge_distance < mortar + 1.6 {
            1.0 - 0.12 * pattern_detail + hash(col, row_i, 31) * 0.06 * pattern_detail
        } else {
            1.0
        };
        let brick_variation = 1.0 + (hash(col, row_i, 11) - 0.5) * 0.44 * pattern_variation;
        let grain = 1.0
            + (hash(
                pattern_x.floor() as i32,
                pattern_y.floor() as i32,
                col.wrapping_mul(19) ^ row_i.wrapping_mul(23),
            ) - 0.5)
                * 0.20
                * pattern_detail;
        let hairline = if (local_y - mortar).abs() < 1.0 || (local_x - mortar).abs() < 0.8 {
            1.0 - 0.07 * pattern_detail
        } else {
            1.0
        };
        let shade = brick_variation * grain * edge_wear * hairline;
        [
            (base[0] as f32 * shade).clamp(0.0, 255.0) as u8,
            (base[1] as f32 * shade).clamp(0.0, 255.0) as u8,
            (base[2] as f32 * shade).clamp(0.0, 255.0) as u8,
            base[3],
        ]
    }

    fn iso_paint_sample_brick_surface_color(
        surface_uv: [f32; 2],
        base: [u8; 4],
        pattern_kind: &str,
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
    ) -> [u8; 4] {
        let pixels_per_world = 42.0;
        Self::iso_paint_sample_brick_color(
            surface_uv[0] * pixels_per_world,
            surface_uv[1] * pixels_per_world,
            base,
            pattern_kind,
            pattern_scale,
            pattern_mortar,
            pattern_detail,
            pattern_variation,
        )
    }

    fn iso_paint_geo_object_matches(a: scenevm::GeoId, b: scenevm::GeoId) -> bool {
        match (a, b) {
            (scenevm::GeoId::GeometryObject(a), scenevm::GeoId::GeometryObject(b)) => a == b,
            (scenevm::GeoId::Sector(a), scenevm::GeoId::Sector(b)) => a == b,
            (scenevm::GeoId::Terrain(..), scenevm::GeoId::Terrain(..)) => true,
            (scenevm::GeoId::Character(a), scenevm::GeoId::Character(b)) => a == b,
            (scenevm::GeoId::Item(a), scenevm::GeoId::Item(b)) => a == b,
            (scenevm::GeoId::Triangle(a), scenevm::GeoId::Triangle(b)) => a == b,
            _ => a == b,
        }
    }

    fn iso_paint_start_clip_pixel(
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        start_screen: Option<[i32; 2]>,
    ) -> Option<scenevm::PaintSurfacePixel> {
        if clip == "none" {
            return None;
        }
        let start_screen = start_screen?;
        surface_buffer?
            .pixel(start_screen[0], start_screen[1])
            .copied()
            .filter(|pixel| pixel.valid)
    }

    fn iso_paint_clip_allows(
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        start_pixel: Option<scenevm::PaintSurfacePixel>,
        x: i32,
        y: i32,
    ) -> bool {
        match clip {
            "none" => true,
            _ => {
                let Some(start) = start_pixel else {
                    return false;
                };
                surface_buffer
                    .and_then(|surface| surface.pixel(x, y))
                    .is_some_and(|pixel| {
                        pixel.valid
                            && Self::iso_paint_geo_object_matches(start.geo_id, pixel.geo_id)
                    })
            }
        }
    }

    fn iso_paint_composite_overlay_scaled_at(
        target: &mut TheRGBABuffer,
        material_pixels: &mut [u8],
        paint: &TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        material_id: u8,
        start_screen: Option<[i32; 2]>,
        x: i32,
        y: i32,
        scale: f32,
    ) {
        let target_dim = *target.dim();
        let paint_dim = *paint.dim();
        if target_dim.width <= 0
            || target_dim.height <= 0
            || paint_dim.width <= 0
            || paint_dim.height <= 0
        {
            return;
        }

        let scale = scale.clamp(0.05, 20.0);
        let target_w = target_dim.width as usize;
        let target_h = target_dim.height as usize;
        let paint_w = paint_dim.width as usize;
        let paint_h = paint_dim.height as usize;
        let draw_w = ((paint_dim.width as f32) * scale).round().max(1.0) as usize;
        let draw_h = ((paint_dim.height as f32) * scale).round().max(1.0) as usize;
        let target_pixels = target.pixels_mut();
        let paint_pixels = paint.pixels();
        let start_pixel = Self::iso_paint_start_clip_pixel(surface_buffer, clip, start_screen);

        for dy_local in 0..draw_h {
            let dy = y + dy_local as i32;
            if dy < 0 || dy >= target_dim.height {
                continue;
            }
            let sy = ((dy_local as f32) / scale).floor() as usize;
            if sy >= paint_h {
                continue;
            }
            for dx_local in 0..draw_w {
                let dx = x + dx_local as i32;
                if dx < 0 || dx >= target_dim.width {
                    continue;
                }
                let sx = ((dx_local as f32) / scale).floor() as usize;
                if sx >= paint_w {
                    continue;
                }
                if !Self::iso_paint_clip_allows(surface_buffer, clip, start_pixel, dx, dy) {
                    continue;
                }

                let src_index = (sy * paint_w + sx) * 4;
                if src_index + 3 >= paint_pixels.len() {
                    continue;
                }
                let src = [
                    paint_pixels[src_index],
                    paint_pixels[src_index + 1],
                    paint_pixels[src_index + 2],
                    paint_pixels[src_index + 3],
                ];
                if src[3] == 0 {
                    continue;
                }
                Self::iso_paint_blend_pixel(target_pixels, target_w, target_h, dx, dy, src);
                Self::iso_paint_set_material_pixel(
                    material_pixels,
                    target_w,
                    target_h,
                    dx,
                    dy,
                    material_id,
                );
            }
        }
    }

    fn iso_paint_composite_brick_overlay_scaled_at(
        target: &mut TheRGBABuffer,
        material_pixels: &mut [u8],
        mask: &TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        material_id: u8,
        start_screen: Option<[i32; 2]>,
        x: i32,
        y: i32,
        scale: f32,
        base: [u8; 4],
        pattern_kind: &str,
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
    ) {
        let target_dim = *target.dim();
        let mask_dim = *mask.dim();
        if target_dim.width <= 0
            || target_dim.height <= 0
            || mask_dim.width <= 0
            || mask_dim.height <= 0
        {
            return;
        }

        let Some(surface_buffer) = surface_buffer else {
            return;
        };

        let scale = scale.clamp(0.05, 20.0);
        let target_w = target_dim.width as usize;
        let target_h = target_dim.height as usize;
        let mask_w = mask_dim.width as usize;
        let mask_h = mask_dim.height as usize;
        let draw_w = ((mask_dim.width as f32) * scale).round().max(1.0) as usize;
        let draw_h = ((mask_dim.height as f32) * scale).round().max(1.0) as usize;
        let target_pixels = target.pixels_mut();
        let mask_pixels = mask.pixels();
        let start_pixel =
            Self::iso_paint_start_clip_pixel(Some(surface_buffer), clip, start_screen);

        for dy_local in 0..draw_h {
            let dy = y + dy_local as i32;
            if dy < 0 || dy >= target_dim.height {
                continue;
            }
            let sy = ((dy_local as f32) / scale).floor() as usize;
            if sy >= mask_h {
                continue;
            }
            for dx_local in 0..draw_w {
                let dx = x + dx_local as i32;
                if dx < 0 || dx >= target_dim.width {
                    continue;
                }
                let sx = ((dx_local as f32) / scale).floor() as usize;
                if sx >= mask_w {
                    continue;
                }
                if !Self::iso_paint_clip_allows(Some(surface_buffer), clip, start_pixel, dx, dy) {
                    continue;
                }

                let src_index = (sy * mask_w + sx) * 4;
                if src_index + 3 >= mask_pixels.len() {
                    continue;
                }
                let mask_alpha = mask_pixels[src_index + 3];
                if mask_alpha == 0 {
                    continue;
                }
                let Some(surface_pixel) = surface_buffer.pixel(dx, dy).filter(|pixel| pixel.valid)
                else {
                    continue;
                };
                let mut color = Self::iso_paint_sample_brick_surface_color(
                    surface_pixel.uv,
                    base,
                    pattern_kind,
                    pattern_scale,
                    pattern_mortar,
                    pattern_detail,
                    pattern_variation,
                );
                color[3] = ((color[3] as u16 * mask_alpha as u16) / 255) as u8;
                if color[3] == 0 {
                    continue;
                }
                Self::iso_paint_blend_pixel(target_pixels, target_w, target_h, dx, dy, color);
                Self::iso_paint_set_material_pixel(
                    material_pixels,
                    target_w,
                    target_h,
                    dx,
                    dy,
                    material_id,
                );
            }
        }
    }

    fn iso_paint_clear_overlay_scaled_at(
        target: &mut TheRGBABuffer,
        material_pixels: &mut [u8],
        mask: &TheRGBABuffer,
        surface_buffer: Option<&scenevm::PaintSurfaceBuffer>,
        clip: &str,
        start_screen: Option<[i32; 2]>,
        x: i32,
        y: i32,
        scale: f32,
    ) {
        let target_dim = *target.dim();
        let mask_dim = *mask.dim();
        if target_dim.width <= 0
            || target_dim.height <= 0
            || mask_dim.width <= 0
            || mask_dim.height <= 0
        {
            return;
        }

        let scale = scale.clamp(0.05, 20.0);
        let target_w = target_dim.width as usize;
        let target_h = target_dim.height as usize;
        let mask_w = mask_dim.width as usize;
        let mask_h = mask_dim.height as usize;
        let draw_w = ((mask_dim.width as f32) * scale).round().max(1.0) as usize;
        let draw_h = ((mask_dim.height as f32) * scale).round().max(1.0) as usize;
        let target_pixels = target.pixels_mut();
        let mask_pixels = mask.pixels();
        let start_pixel = Self::iso_paint_start_clip_pixel(surface_buffer, clip, start_screen);

        for dy_local in 0..draw_h {
            let dy = y + dy_local as i32;
            if dy < 0 || dy >= target_dim.height {
                continue;
            }
            let sy = ((dy_local as f32) / scale).floor() as usize;
            if sy >= mask_h {
                continue;
            }
            for dx_local in 0..draw_w {
                let dx = x + dx_local as i32;
                if dx < 0 || dx >= target_dim.width {
                    continue;
                }
                let sx = ((dx_local as f32) / scale).floor() as usize;
                if sx >= mask_w {
                    continue;
                }
                if !Self::iso_paint_clip_allows(surface_buffer, clip, start_pixel, dx, dy) {
                    continue;
                }

                let src_index = (sy * mask_w + sx) * 4;
                let dst_index = (dy as usize * target_w + dx as usize) * 4;
                if src_index + 3 >= mask_pixels.len() || dst_index + 3 >= target_pixels.len() {
                    continue;
                }
                let mask_a = mask_pixels[src_index + 3] as u16;
                if mask_a == 0 {
                    continue;
                }
                let keep = 255 - mask_a;
                target_pixels[dst_index + 3] =
                    ((target_pixels[dst_index + 3] as u16 * keep) / 255) as u8;
                Self::iso_paint_clear_material_pixel(
                    target_pixels,
                    material_pixels,
                    target_w,
                    target_h,
                    dx,
                    dy,
                );
            }
        }
    }

    fn iso_paint_preview_color(layer: &IsoPaintLayer) -> [u8; 4] {
        match layer.active_operation.as_str() {
            "erase" => [242, 92, 84, 230],
            "pick" => [87, 186, 255, 230],
            _ => {
                let mut color = layer.active_color;
                color[3] = 230;
                color
            }
        }
    }

    fn draw_iso_paint_preview(
        buffer: &mut TheRGBABuffer,
        layer: &IsoPaintLayer,
        hover: Option<Vec2<i32>>,
    ) {
        if !layer.visible || layer.active_operation == "pick" && hover.is_none() {
            return;
        }

        let Some(hover) = hover else {
            return;
        };
        let dim = *buffer.dim();
        if dim.width <= 0 || dim.height <= 0 {
            return;
        }

        let radius = (layer.active_size * 2.0).round().clamp(3.0, 96.0) as i32;
        let outer = radius + 2;
        let radius_sq = radius * radius;
        let inner_sq = (radius - 2).max(1).pow(2);
        let shadow_sq = outer * outer;
        let color = Self::iso_paint_preview_color(layer);
        let fill = [color[0], color[1], color[2], 38];
        let shadow = [8, 10, 12, 145];
        let pixels = buffer.pixels_mut();
        let width = dim.width as usize;
        let height = dim.height as usize;

        for oy in -outer..=outer {
            for ox in -outer..=outer {
                let d = ox * ox + oy * oy;
                let x = hover.x + ox;
                let y = hover.y + oy;
                if d <= shadow_sq && d > radius_sq {
                    Self::iso_paint_blend_pixel(pixels, width, height, x, y, shadow);
                } else if d <= radius_sq && d >= inner_sq {
                    Self::iso_paint_blend_pixel(pixels, width, height, x, y, color);
                } else if d < inner_sq && layer.active_operation != "pick" {
                    Self::iso_paint_blend_pixel(pixels, width, height, x, y, fill);
                }
            }
        }
    }

    fn iso_paint_current_camera_scale() -> Option<f32> {
        RUSTERIX
            .read()
            .ok()
            .map(|rusterix| rusterix.client.camera_d3.scale())
    }

    fn iso_paint_stroke_anchor(
        stroke: &IsoPaintStroke,
    ) -> (Option<[i32; 2]>, Option<[f32; 3]>, Option<f32>) {
        for point in &stroke.points {
            if let Some(world) = point.world {
                return (Some(point.screen), Some(world), point.camera_scale);
            }
        }
        (None, None, None)
    }

    fn iso_paint_stroke_bounds(stroke: &IsoPaintStroke) -> ([i32; 2], [i32; 2]) {
        let pad = (stroke.size * 2.0).round().max(1.0) as i32 + 1;
        let min = [stroke.screen_bounds[0] - pad, stroke.screen_bounds[1] - pad];
        let max = [stroke.screen_bounds[2] + pad, stroke.screen_bounds[3] + pad];
        (min, max)
    }

    fn build_iso_paint_stroke_caches(stroke: &IsoPaintStroke) -> Vec<IsoPaintStrokeRenderCache> {
        if stroke.points.is_empty() || stroke.operation == "pick" {
            return Vec::new();
        }

        let erase = stroke.operation == "erase";
        let (origin, max) = Self::iso_paint_stroke_bounds(stroke);
        let width = (max[0] - origin[0] + 1).max(1);
        let height = (max[1] - origin[1] + 1).max(1);
        let mut paint = TheRGBABuffer::new(TheDim::sized(width, height));
        let paint_w = width as usize;
        let paint_h = height as usize;

        let (screen_anchor, world_anchor, camera_scale) = Self::iso_paint_stroke_anchor(stroke);
        if !erase && stroke.brush == "brick" && world_anchor.is_none() {
            return Vec::new();
        }

        let color = if erase {
            [
                0,
                0,
                0,
                (stroke.opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
            ]
        } else if stroke.brush == "brick" {
            [
                255,
                255,
                255,
                (stroke.opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
            ]
        } else {
            Self::iso_paint_color_with_opacity(stroke.color, stroke.opacity)
        };
        let radius = (stroke.size * 2.0).round().max(1.0) as i32;
        let pixels = paint.pixels_mut();

        if stroke.points.len() == 1 {
            let point = &stroke.points[0];
            Self::iso_paint_stamp(
                pixels,
                paint_w,
                paint_h,
                point.screen[0] - origin[0],
                point.screen[1] - origin[1],
                radius,
                color,
            );
        } else {
            for pair in stroke.points.windows(2) {
                Self::iso_paint_stamp(
                    pixels,
                    paint_w,
                    paint_h,
                    pair[0].screen[0] - origin[0],
                    pair[0].screen[1] - origin[1],
                    radius,
                    color,
                );
                Self::iso_paint_draw_segment(
                    pixels,
                    paint_w,
                    paint_h,
                    pair[0].screen,
                    pair[1].screen,
                    origin,
                    radius,
                    color,
                );
            }
        }

        vec![IsoPaintStrokeRenderCache {
            origin,
            screen_anchor,
            world_anchor,
            camera_scale: camera_scale.or_else(Self::iso_paint_current_camera_scale),
            brush: stroke.brush.clone(),
            clip: stroke.clip.clone(),
            material_id: stroke.material_id,
            color: Self::iso_paint_color_with_opacity(stroke.color, 1.0),
            pattern_kind: stroke.pattern_kind.clone(),
            pattern_scale: stroke.pattern_scale,
            pattern_mortar: stroke.pattern_mortar,
            pattern_detail: stroke.pattern_detail,
            pattern_variation: stroke.pattern_variation,
            erase,
            buffer: paint,
        }]
    }

    fn build_iso_paint_chunk_cache(chunk: &IsoPaintChunk) -> IsoPaintChunkRenderCache {
        IsoPaintChunkRenderCache {
            revision: chunk.revision,
            strokes: chunk
                .strokes
                .iter()
                .flat_map(Self::build_iso_paint_stroke_caches)
                .collect(),
        }
    }

    fn iso_paint_layer_key(layer: &IsoPaintLayer) -> u64 {
        let mut hasher = DefaultHasher::new();
        layer.visible.hash(&mut hasher);
        layer.chunks.len().hash(&mut hasher);
        for (key, chunk) in &layer.chunks {
            key.hash(&mut hasher);
            chunk.origin.hash(&mut hasher);
            chunk.revision.hash(&mut hasher);
            chunk.strokes.len().hash(&mut hasher);
        }
        hasher.finish()
    }

    fn iso_paint_overlay_key(
        region_id: Uuid,
        layer: &IsoPaintLayer,
        target_dim: TheDim,
        paint_surface_key: u64,
        current_camera_scale: Option<f32>,
    ) -> IsoPaintPreparedOverlayKey {
        IsoPaintPreparedOverlayKey {
            region_id,
            width: target_dim.width,
            height: target_dim.height,
            layer_key: Self::iso_paint_layer_key(layer),
            surface_key: paint_surface_key,
            camera_scale_bits: current_camera_scale.unwrap_or(0.0).to_bits(),
        }
    }

    fn build_iso_paint_overlay_prepared(
        render_cache: &mut IsoPaintRenderCache,
        region_id: Uuid,
        layer: &IsoPaintLayer,
        paint_surface: Option<&scenevm::PaintSurfaceBuffer>,
        paint_surface_key: u64,
        current_camera_scale: Option<f32>,
        target_dim: TheDim,
        project_world_anchor: impl Fn([f32; 3], i32, i32) -> Option<[i32; 2]>,
    ) -> Option<(IsoPaintPreparedOverlayKey, IsoPaintPreparedOverlay, bool)> {
        if render_cache.region_id != Some(region_id) {
            render_cache.region_id = Some(region_id);
            render_cache.chunks.clear();
            render_cache.prepared_key = None;
            render_cache.prepared_overlay = None;
            render_cache.uploaded_key = None;
        }

        if !layer.visible
            || layer.chunks.is_empty()
            || target_dim.width <= 0
            || target_dim.height <= 0
        {
            return None;
        }

        let overlay_key = Self::iso_paint_overlay_key(
            region_id,
            layer,
            target_dim,
            paint_surface_key,
            current_camera_scale,
        );
        if render_cache.prepared_key == Some(overlay_key)
            && let Some(overlay) = render_cache.prepared_overlay.as_ref()
        {
            return Some((overlay_key, overlay.clone(), false));
        }

        let mut paint_overlay = TheRGBABuffer::new(target_dim);
        let mut material_overlay =
            vec![0_u8; target_dim.width as usize * target_dim.height as usize * 4];
        for pixel in material_overlay.chunks_exact_mut(4) {
            pixel.copy_from_slice(&Self::iso_paint_material_pixel(0));
        }

        render_cache
            .chunks
            .retain(|key, _| layer.chunks.contains_key(key));

        for (key, chunk) in &layer.chunks {
            let rebuild = render_cache
                .chunks
                .get(key)
                .map(|cached| cached.revision != chunk.revision)
                .unwrap_or(true);
            if rebuild {
                let cached = Self::build_iso_paint_chunk_cache(chunk);
                render_cache.chunks.insert(key.clone(), cached);
            }

            if let Some(cached) = render_cache.chunks.get(key) {
                for stroke in &cached.strokes {
                    let mut draw_origin = stroke.origin;
                    let mut draw_scale = 1.0;
                    let mut start_screen = stroke.screen_anchor;
                    if let (Some(screen_anchor), Some(world_anchor)) =
                        (stroke.screen_anchor, stroke.world_anchor)
                    {
                        if let Some(current_screen) =
                            project_world_anchor(world_anchor, target_dim.width, target_dim.height)
                        {
                            if let (Some(source_scale), Some(current_scale)) =
                                (stroke.camera_scale, current_camera_scale)
                            {
                                draw_scale =
                                    (source_scale / current_scale.max(0.001)).clamp(0.05, 20.0);
                            }
                            let anchor_local_x = screen_anchor[0] - stroke.origin[0];
                            let anchor_local_y = screen_anchor[1] - stroke.origin[1];
                            draw_origin[0] = (current_screen[0] as f32
                                - anchor_local_x as f32 * draw_scale)
                                .round() as i32;
                            draw_origin[1] = (current_screen[1] as f32
                                - anchor_local_y as f32 * draw_scale)
                                .round() as i32;
                            start_screen = Some(current_screen);
                        }
                    }

                    if stroke.erase {
                        Self::iso_paint_clear_overlay_scaled_at(
                            &mut paint_overlay,
                            &mut material_overlay,
                            &stroke.buffer,
                            paint_surface,
                            &stroke.clip,
                            start_screen,
                            draw_origin[0],
                            draw_origin[1],
                            draw_scale,
                        );
                    } else if stroke.brush == "brick" {
                        Self::iso_paint_composite_brick_overlay_scaled_at(
                            &mut paint_overlay,
                            &mut material_overlay,
                            &stroke.buffer,
                            paint_surface,
                            &stroke.clip,
                            stroke.material_id,
                            start_screen,
                            draw_origin[0],
                            draw_origin[1],
                            draw_scale,
                            stroke.color,
                            &stroke.pattern_kind,
                            stroke.pattern_scale,
                            stroke.pattern_mortar,
                            stroke.pattern_detail,
                            stroke.pattern_variation,
                        );
                    } else {
                        Self::iso_paint_composite_overlay_scaled_at(
                            &mut paint_overlay,
                            &mut material_overlay,
                            &stroke.buffer,
                            paint_surface,
                            &stroke.clip,
                            stroke.material_id,
                            start_screen,
                            draw_origin[0],
                            draw_origin[1],
                            draw_scale,
                        );
                    }
                }
            }
        }

        let overlay = IsoPaintPreparedOverlay {
            width: target_dim.width as u32,
            height: target_dim.height as u32,
            color_rgba: paint_overlay.pixels().to_vec(),
            material_rgba: material_overlay,
        };
        render_cache.prepared_key = Some(overlay_key);
        render_cache.prepared_overlay = Some(overlay.clone());
        Some((overlay_key, overlay, true))
    }

    fn ensure_project_extension(mut path: PathBuf) -> PathBuf {
        if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
            let file_name = file_name.to_string();
            if !file_name
                .to_ascii_lowercase()
                .ends_with(&format!(".{}", Self::PROJECT_EXTENSION))
            {
                path.set_file_name(format!("{file_name}.{}", Self::PROJECT_EXTENSION));
            }
        } else if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(Self::PROJECT_EXTENSION))
        {
            path.set_extension(Self::PROJECT_EXTENSION);
        }

        path
    }

    fn activate_edit_tile_meta_action(&mut self) {
        if self.server_ctx.curr_tile_id.is_none() {
            return;
        }

        if let Some(action) = ACTIONLIST
            .read()
            .unwrap()
            .actions
            .iter()
            .find(|action| action.id().name == fl!("action_edit_tile"))
        {
            self.server_ctx.curr_action_id = Some(action.id().uuid);
        }
    }

    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    fn set_update_button_text(ui: &mut TheUI, ctx: &mut TheContext, text: Option<String>) {
        if let Some(widget) = ui.get_widget("Update") {
            if let Some(text) = text {
                widget.set_value(TheValue::Text(text));
                widget.set_disabled(false);
                widget.limiter_mut().set_max_width(180);
            } else {
                widget.set_value(TheValue::Text(String::new()));
                widget.set_disabled(true);
                widget.limiter_mut().set_max_width(0);
            }

            ctx.ui.relayout = true;
        }
    }

    #[cfg(all(
        feature = "self-update",
        any(target_os = "windows", target_os = "linux", target_os = "macos")
    ))]
    fn set_update_button(ui: &mut TheUI, ctx: &mut TheContext, release: Option<&Release>) {
        Self::set_update_button_text(
            ui,
            ctx,
            release.map(|release| format!("Update to v{}", release.version)),
        );
    }

    fn log_segment_has_warning_or_error(segment: &str) -> bool {
        let segment = segment.to_ascii_lowercase();
        segment.contains("[error]") || segment.contains("[warning]") || segment.contains("[warn]")
    }

    fn starter_manifest_url() -> String {
        format!("{}starters/manifest.toml", Self::STARTER_REPO_RAW_BASE)
    }

    fn starter_repo_url(repo_path: &str) -> String {
        format!("{}{}", Self::STARTER_REPO_RAW_BASE, repo_path)
    }

    fn fetch_url_bytes(url: &str) -> Option<Vec<u8>> {
        if let Ok(response) = ureq::get(url)
            .set("Cache-Control", "no-cache")
            .set("Pragma", "no-cache")
            .call()
        {
            let mut reader = response.into_reader();
            let mut bytes = Vec::new();
            if reader.read_to_end(&mut bytes).is_ok() {
                return Some(bytes);
            }
        }
        None
    }

    fn fetch_url_text(url: &str) -> Option<String> {
        let bytes = Self::fetch_url_bytes(url)?;
        String::from_utf8(bytes).ok()
    }

    fn refresh_system_text_clipboard(ctx: &mut TheContext) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(mut clipboard) = arboard::Clipboard::new()
                && let Ok(text) = clipboard.get_text()
            {
                ctx.ui.clipboard = Some(TheValue::Text(text));
                ctx.ui.clipboard_app_type = Some("text/plain".to_string());
            }
        }
    }

    fn load_project_from_json_path(path: &std::path::Path) -> Option<Project> {
        let contents = std::fs::read_to_string(path).ok()?;
        let mut loaded = serde_json::from_str::<Project>(&contents).ok()?;
        loaded.migrate_default_ruleset();
        loaded.migrate_button_commands();
        let _ = loaded.sync_ruleset_items();
        loaded.art_palette.current_index = 0;
        Some(loaded)
    }

    fn load_empty_project_template() -> Project {
        let mut project = Project::new();
        if let Some(bytes) = crate::Embedded::get("toml/config.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.config = source.to_string();
        }
        if let Some(bytes) = crate::Embedded::get("toml/rules.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.rules = source.to_string();
        }
        if let Some(bytes) = crate::Embedded::get("toml/locales.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.locales = source.to_string();
        }
        if let Some(bytes) = crate::Embedded::get("toml/audio_fx.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.audio_fx = source.to_string();
        }
        if let Some(bytes) = crate::Embedded::get("toml/authoring.toml")
            && let Ok(source) = std::str::from_utf8(bytes.data.as_ref())
        {
            project.authoring = source.to_string();
        }
        let _ = project.sync_ruleset_items();
        project
    }

    fn load_starter_manifest() -> Vec<StarterProjectEntry> {
        let contents = match Self::fetch_url_text(&Self::starter_manifest_url()) {
            Some(contents) => contents,
            None => return Vec::new(),
        };
        let manifest = match toml::from_str::<StarterProjectManifest>(&contents) {
            Ok(manifest) => manifest,
            Err(_) => return Vec::new(),
        };

        manifest
            .starter
            .into_iter()
            .map(|entry| StarterProjectEntry {
                id: Uuid::new_v4(),
                preview: Self::load_starter_preview(&entry.image),
                manifest_id: entry.id,
                title: entry.title,
                description: entry.description,
                project_path: entry.project_path,
            })
            .collect()
    }

    fn load_starter_preview(repo_path: &str) -> Option<TheRGBATile> {
        let bytes = Self::fetch_url_bytes(&Self::starter_repo_url(repo_path))?;
        Self::decode_png_tile(bytes)
    }

    fn decode_png_tile(bytes: Vec<u8>) -> Option<TheRGBATile> {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().ok()?;
        let buffer_size = reader.output_buffer_size()?;
        let mut buf = vec![0; buffer_size];
        let info = reader.next_frame(&mut buf).ok()?;
        let bytes = &buf[..info.buffer_size()];
        Some(TheRGBATile::buffer(TheRGBABuffer::from(
            bytes.to_vec(),
            info.width,
            info.height,
        )))
    }

    fn load_named_starter_project(&mut self, manifest_id: &str) -> Option<Project> {
        if let Some(project) = self.starter_project_cache.get(manifest_id).cloned() {
            return Some(project);
        }

        let choice = self
            .starter_manifest_cache
            .clone()
            .unwrap_or_else(|| self.starter_projects.clone())
            .into_iter()
            .find(|choice| choice.manifest_id == manifest_id)?;
        let contents = Self::fetch_url_text(&Self::starter_repo_url(&choice.project_path))?;
        let mut loaded = serde_json::from_str::<Project>(&contents).ok()?;
        loaded.migrate_default_ruleset();
        loaded.migrate_button_commands();
        let _ = loaded.sync_ruleset_items();
        loaded.art_palette.current_index = 0;
        self.starter_project_cache
            .insert(manifest_id.to_string(), loaded.clone());
        Some(loaded)
    }

    fn window_state_file_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(
            PathBuf::from(home)
                .join(".eldiron")
                .join("creator_window_state.json"),
        )
    }

    fn load_window_state() -> CreatorWindowState {
        if let Some(path) = Self::window_state_file_path()
            && let Ok(data) = fs::read_to_string(path)
            && let Ok(state) = serde_json::from_str::<CreatorWindowState>(&data)
        {
            return state;
        }
        CreatorWindowState::default()
    }

    fn save_window_state(&self) {
        if let Some(path) = Self::window_state_file_path() {
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            if let Ok(json) = serde_json::to_string(&self.window_state) {
                let _ = fs::write(path, json);
            }
        }
    }

    fn persist_active_region_view_state(&mut self) {
        if let Some(region) = self.project.get_region_mut(&self.server_ctx.curr_region) {
            match self.server_ctx.editor_view_mode {
                EditorViewMode::Iso => {
                    region.editing_position_iso_3d = Some(region.editing_position_3d);
                    region.editing_look_at_iso_3d = Some(region.editing_look_at_3d);
                    region.editing_iso_scale = Some(EDITCAMERA.read().unwrap().iso_camera.scale);
                }
                EditorViewMode::Orbit => {
                    region.editing_position_orbit_3d = Some(region.editing_position_3d);
                    region.editing_look_at_orbit_3d = Some(region.editing_look_at_3d);
                    region.editing_orbit_distance =
                        Some(EDITCAMERA.read().unwrap().orbit_camera.distance);
                }
                EditorViewMode::FirstP => {
                    region.editing_position_firstp_3d = Some(region.editing_position_3d);
                    region.editing_look_at_firstp_3d = Some(region.editing_look_at_3d);
                }
                EditorViewMode::D2 => {}
            }
        }
    }

    fn project_tab_title_for(
        project: &Project,
        project_path: &Option<PathBuf>,
        fallback_index: usize,
        dirty: bool,
    ) -> String {
        let prefix = if dirty { "* " } else { "" };

        if let Some(path) = project_path
            && let Some(stem) = path.file_stem()
            && let Some(name) = stem.to_str()
            && !name.is_empty()
        {
            return format!("{}{}", prefix, name);
        }
        if !project.name.is_empty() {
            return format!("{}{}", prefix, project.name);
        }

        if project_path.is_none() {
            return format!("{}{}", prefix, fl!("new_project"));
        }

        format!("{}Project {}", prefix, fallback_index + 1)
    }

    fn sync_active_session_from_editor(&mut self) {
        if self.active_session >= self.sessions.len() {
            return;
        }
        self.persist_active_region_view_state();
        self.sessions[self.active_session].project = self.project.clone();
        self.sessions[self.active_session].project_path = self.project_path.clone();
        self.sessions[self.active_session].undo = UNDOMANAGER.read().unwrap().clone();
        self.sessions[self.active_session].dirty = self.active_session_has_changes();
    }

    fn sync_editor_from_active_session(&mut self) {
        if self.active_session >= self.sessions.len() {
            return;
        }
        let session = self.sessions[self.active_session].clone();
        self.project = session.project;
        self.project_path = session.project_path;
        *UNDOMANAGER.write().unwrap() = session.undo;
    }

    fn rebuild_project_tabs(&self, ui: &mut TheUI) {
        if let Some(widget) = ui.get_widget("Project Tabs")
            && let Some(tabbar) = widget.as_tabbar()
        {
            tabbar.clear();
            for (index, session) in self.sessions.iter().enumerate() {
                tabbar.add_tab(Self::project_tab_title_for(
                    &session.project,
                    &session.project_path,
                    index,
                    session.dirty,
                ));
            }
            tabbar.set_selection_index(self.active_session);
        }
    }

    fn open_starter_project_dialog(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        self.starter_loader_rx = None;
        self.selected_starter_manifest_id = None;

        let width = 980;
        let height = 340;
        let bottom_bar_height = 32;
        let preview_size = height;

        let mut dialog = TheCanvas::new();
        dialog.limiter_mut().set_max_size(Vec2::new(width, height));

        let mut left = TheCanvas::new();
        left.limiter_mut()
            .set_max_size(Vec2::new(preview_size, preview_size));
        let mut preview = TheIconView::new(TheId::named(Self::STARTER_PREVIEW_ID));
        preview
            .limiter_mut()
            .set_max_size(Vec2::new(preview_size, preview_size));
        preview.set_border_color(Some([120, 120, 120, 255]));
        preview.set_background_color(Some([218, 211, 177, 255]));
        preview.set_alpha_mode(true);
        if let Some(tile) = ctx.ui.icon("lord").cloned().map(TheRGBATile::buffer) {
            preview.set_rgba_tile(tile);
        }
        left.set_widget(preview);
        dialog.set_left(left);

        let mut center = TheCanvas::new();
        center
            .limiter_mut()
            .set_max_size(Vec2::new(width - (preview_size + 20), preview_size));
        let mut list = TheListLayout::new(TheId::named(Self::STARTER_LIST_ID));
        list.set_item_size(52);
        let mut item = TheListItem::new(TheId::named("Starter Project Loading"));
        item.set_text(fl!("starter_loading"));
        item.set_sub_text(fl!("starter_loading_sub"));
        item.set_size(52);
        item.set_text_color(WHITE);
        item.set_text_size(14.0);
        item.set_sub_text_size(12.0);
        list.add_item(item, ctx);
        center.set_layout(list);
        dialog.set_center(center);

        let mut bottom = TheCanvas::new();
        bottom
            .limiter_mut()
            .set_max_size(Vec2::new(width, bottom_bar_height));
        let mut actions = TheHLayout::new(TheId::named("Starter Project Actions"));
        actions
            .limiter_mut()
            .set_max_size(Vec2::new(width, bottom_bar_height));
        actions.set_background_color(Some(TheThemeColors::ListLayoutBackground));
        actions.set_margin(Vec4::new(10, 2, 10, 2));
        actions.set_padding(8);
        actions.set_reverse_index(Some(2));

        let mut create = TheTraybarButton::new(TheId::named(Self::STARTER_CREATE_ID));
        create.set_text(fl!("starter_choose"));
        actions.add_widget(Box::new(create));

        let mut cancel = TheTraybarButton::new(TheId::named(Self::STARTER_CANCEL_ID));
        cancel.set_text(fl!("starter_cancel"));
        actions.add_widget(Box::new(cancel));

        bottom.set_layout(actions);
        dialog.set_bottom(bottom);

        ui.show_dialog(&fl!("starter_dialog_title"), dialog, vec![], ctx);
        if let Some(starters) = self.starter_manifest_cache.clone() {
            self.starter_projects = starters;
            self.rebuild_starter_project_list(ui, ctx);
            if let Some(first) = self.starter_projects.first() {
                self.selected_starter_manifest_id = Some(first.manifest_id.clone());
                ctx.ui.send(TheEvent::StateChanged(
                    TheId::named_with_id("Starter Project List Item", first.id),
                    TheWidgetState::Selected,
                ));
                ui.set_enabled(Self::STARTER_CREATE_ID, ctx);
            } else {
                ui.set_disabled(Self::STARTER_CREATE_ID, ctx);
            }
        } else {
            self.starter_projects.clear();
            ui.set_disabled(Self::STARTER_CREATE_ID, ctx);
        }

        let (tx, rx) = std::sync::mpsc::channel();
        self.starter_loader_rx = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(Self::load_starter_manifest());
        });
    }

    fn rebuild_starter_project_list(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(list) = ui.get_list_layout(Self::STARTER_LIST_ID) {
            list.clear();
            list.set_item_size(52);
            for (index, entry) in self.starter_projects.iter().enumerate() {
                let mut item =
                    TheListItem::new(TheId::named_with_id("Starter Project List Item", entry.id));
                item.set_text(entry.title.clone());
                item.set_sub_text(entry.description.clone());
                item.set_size(52);
                item.set_text_color(WHITE);
                item.set_text_size(14.0);
                item.set_sub_text_size(12.0);
                if index == 0 {
                    item.set_state(TheWidgetState::Selected);
                }
                if let Some(preview) = &entry.preview
                    && let Some(first) = preview.buffer.first()
                {
                    item.set_icon(first.clone());
                }
                list.add_item(item, ctx);
            }
        }
    }

    fn open_project_as_session(
        &mut self,
        mut project: Project,
        project_path: Option<PathBuf>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        update_server_icons: &mut bool,
        redraw: &mut bool,
    ) {
        Self::sanitize_loaded_project(&mut project);

        self.sync_active_session_from_editor();
        let new_index = if self.replace_next_project_load_in_active_tab {
            self.sessions[self.active_session] = ProjectSession {
                project,
                project_path,
                undo: UndoManager::default(),
                dirty: false,
            };
            self.replace_next_project_load_in_active_tab = false;
            self.active_session
        } else {
            self.sessions.push(ProjectSession {
                project,
                project_path,
                undo: UndoManager::default(),
                dirty: false,
            });
            self.sessions.len() - 1
        };
        self.switch_to_session(new_index, ui, ctx, update_server_icons, redraw);
    }

    fn activate_loaded_project(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        update_server_icons: &mut bool,
        redraw: &mut bool,
    ) {
        self.update_counter = 0;
        self.sidebar.startup = true;

        if let Some(widget) = ui.get_widget("Server Time Slider") {
            widget.set_value(TheValue::Time(self.project.time));
        }

        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.assets.config = self.project.config.clone();
            rusterix
                .scene_handler
                .sync_base_render_settings(&self.project.config);
            rusterix.client.set_server_time(self.project.time);
            if rusterix.server.state == rusterix::ServerState::Running
                && let Some(map) = self.project.get_map(&self.server_ctx)
            {
                rusterix.server.set_time(&map.id, self.project.time);
            }
        }

        self.server_ctx.clear();
        self.server_ctx.text_game_mode = TOOLLIST.read().unwrap().text_game_mode;
        TEXTGAME.write().unwrap().reset();
        if let Some(first) = self.project.regions.first() {
            self.server_ctx.curr_region = first.id;
        }
        let restored_view_index = self
            .project
            .get_region(&self.server_ctx.curr_region)
            .map(|region| match region.map.camera {
                MapCamera::TwoD => 0,
                MapCamera::ThreeDIso => 2,
                MapCamera::ThreeDFirstPerson => 3,
            })
            .unwrap_or(0);
        self.server_ctx.editor_view_mode = EditorViewMode::from_index(restored_view_index);
        let restored_camera_action_name = match restored_view_index {
            2 => fl!("action_iso_camera"),
            3 => fl!("action_first_p_camera"),
            _ => fl!("action_editing_camera"),
        };

        self.sidebar
            .load_from_project(ui, ctx, &mut self.server_ctx, &mut self.project);
        self.mapeditor.load_from_project(ui, ctx, &self.project);
        if let Some(widget) = ui.get_widget("Editor View Switch")
            && let Some(group) = widget.as_group_button()
        {
            group.set_index(restored_view_index);
        }
        {
            let mut actions = ACTIONLIST.write().unwrap();
            if let Some(action) = actions
                .actions
                .iter_mut()
                .find(|action| action.id().name == restored_camera_action_name)
            {
                self.server_ctx.curr_action_id = Some(action.id().uuid);
                if let Some(map) = self.project.get_map_mut(&self.server_ctx) {
                    action.load_params(map);
                    let _ = action.apply(map, ui, ctx, &mut self.server_ctx);
                }
                action.load_params_project(&self.project, &mut self.server_ctx);
                action.apply_project(&mut self.project, ui, ctx, &mut self.server_ctx);
            }
        }
        *update_server_icons = true;
        *redraw = true;

        *PALETTE.write().unwrap() = self.project.art_palette.clone();
        {
            let mut rusterix = RUSTERIX.write().unwrap();
            rusterix.assets.palette = self.project.art_palette.clone();
            rusterix.assets.palette_materials = self
                .project
                .art_palette_materials
                .iter()
                .map(|m| m.rmoe_values())
                .collect();
            rusterix.assets.palette_material_ids =
                crate::undo::project_helper::palette_material_ids(&self.project);
            rusterix.set_tiles(self.project.tiles.clone(), true);
            rusterix.set_tile_groups(self.project.tile_groups.clone());
        }
        SCENEMANAGER.write().unwrap().set_palette(
            self.project.art_palette.clone(),
            self.project
                .art_palette_materials
                .iter()
                .map(|m| m.rmoe_values())
                .collect(),
            crate::undo::project_helper::palette_material_ids(&self.project),
        );

        UNDOMANAGER.read().unwrap().set_undo_state_to_ui(ctx);
    }

    fn switch_to_session(
        &mut self,
        index: usize,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        update_server_icons: &mut bool,
        redraw: &mut bool,
    ) {
        if index >= self.sessions.len() {
            self.rebuild_project_tabs(ui);
            return;
        }
        if index == self.active_session {
            self.sync_editor_from_active_session();
            self.activate_loaded_project(ui, ctx, update_server_icons, redraw);
            self.rebuild_project_tabs(ui);
            return;
        }
        self.sync_active_session_from_editor();
        self.active_session = index;
        self.sync_editor_from_active_session();
        self.activate_loaded_project(ui, ctx, update_server_icons, redraw);
        self.rebuild_project_tabs(ui);
    }

    fn sanitize_loaded_project(project: &mut Project) {
        project.migrate_button_commands();
        insert_content_into_maps(project);

        let mut char_names = FxHashMap::default();
        for c in &project.characters {
            char_names.insert(c.0, c.1.name.clone());
        }
        for r in &mut project.regions {
            for c in &mut r.characters {
                if let Some(n) = char_names.get(&c.1.character_id) {
                    c.1.name = n.clone();
                }
            }
        }

        let mut item_names = FxHashMap::default();
        for c in &project.items {
            item_names.insert(c.0, c.1.name.clone());
        }
        for r in &mut project.regions {
            for c in &mut r.items {
                if let Some(n) = item_names.get(&c.1.item_id) {
                    c.1.name = n.clone();
                }
            }
            for (_, p) in &mut r.map.profiles {
                p.sanitize();
            }
            r.map.sanitize();
        }

        for (_, screen) in &mut project.screens {
            screen.map.sanitize();
        }

        for (_, tile) in project.tiles.iter_mut() {
            for texture in &mut tile.textures {
                if texture.data_ext.is_none() {
                    texture.generate_normals(true);
                }
            }
        }
    }

    fn close_active_session(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        update_server_icons: &mut bool,
        redraw: &mut bool,
    ) {
        if self.sessions.is_empty() {
            return;
        }

        self.sync_active_session_from_editor();
        self.sessions.remove(self.active_session);

        if self.sessions.is_empty() {
            let project = Self::load_empty_project_template();
            self.sessions.push(ProjectSession {
                project,
                project_path: None,
                undo: UndoManager::default(),
                dirty: false,
            });
            self.active_session = 0;
        } else if self.active_session >= self.sessions.len() {
            self.active_session = self.sessions.len() - 1;
        }

        self.sync_editor_from_active_session();
        self.activate_loaded_project(ui, ctx, update_server_icons, redraw);
        self.rebuild_project_tabs(ui);
        if self.sessions.len() == 1 && self.project_path.is_none() {
            self.replace_next_project_load_in_active_tab = true;
            self.open_starter_project_dialog(ui, ctx);
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("status_starter_choose"),
            ));
            *redraw = true;
        }
    }

    fn active_session_has_changes(&self) -> bool {
        UNDOMANAGER.read().unwrap().has_unsaved() || DOCKMANAGER.read().unwrap().has_dock_changes()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_project_snapshot(&self) -> serde_json::Value {
        let current_region = self
            .project
            .regions
            .iter()
            .find(|region| region.id == self.server_ctx.curr_region)
            .map(|region| {
                serde_json::json!({
                    "id": region.id.to_string(),
                    "name": region.name,
                    "map_name": region.map.name,
                    "camera": format!("{:?}", region.map.camera),
                })
            });

        let regions = self
            .project
            .regions
            .iter()
            .map(|region| {
                serde_json::json!({
                    "id": region.id.to_string(),
                    "name": region.name,
                    "map_name": region.map.name,
                    "camera": format!("{:?}", region.map.camera),
                    "sectors": region.map.sectors.len(),
                    "items": region.items.len(),
                    "characters": region.characters.len(),
                })
            })
            .collect::<Vec<_>>();

        let characters = self
            .project
            .characters
            .values()
            .map(|character| {
                serde_json::json!({
                    "id": character.id.to_string(),
                    "name": character.name,
                    "source_len": character.source.len(),
                    "data_len": character.data.len(),
                    "has_authoring": !character.authoring.trim().is_empty(),
                    "has_preview_rigging": !character.preview_rigging.trim().is_empty(),
                })
            })
            .collect::<Vec<_>>();

        let items = self
            .project
            .items
            .values()
            .map(|item| {
                serde_json::json!({
                    "id": item.id.to_string(),
                    "name": item.name,
                    "source_len": item.source.len(),
                    "data_len": item.data.len(),
                    "has_authoring": !item.authoring.trim().is_empty(),
                })
            })
            .collect::<Vec<_>>();

        serde_json::json!({
            "name": self.project.name,
            "path": self.project_path.as_ref().map(|path| path.display().to_string()),
            "dirty": self.active_session_has_changes(),
            "active_session": self.active_session,
            "session_count": self.sessions.len(),
            "current_region": current_region,
            "regions": regions,
            "characters": characters,
            "items": items,
            "counts": {
                "regions": self.project.regions.len(),
                "tiles": self.project.tiles.len(),
                "tile_groups": self.project.tile_groups.len(),
                "tile_node_groups": self.project.tile_node_groups.len(),
                "characters": self.project.characters.len(),
                "items": self.project.items.len(),
                "screens": self.project.screens.len(),
                "assets": self.project.assets.len(),
            }
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_tiles_snapshot(&self) -> serde_json::Value {
        let mut tiles = self
            .project
            .tiles
            .values()
            .map(|tile| {
                let first_frame = tile.textures.first().map(|texture| {
                    serde_json::json!({
                        "width": texture.width,
                        "height": texture.height,
                    })
                });

                serde_json::json!({
                    "id": tile.id.to_string(),
                    "alias": tile.alias,
                    "role": tile.role.to_string(),
                    "blocking": tile.blocking,
                    "scale": tile.scale,
                    "frame_count": tile.textures.len(),
                    "first_frame": first_frame,
                    "procedural": {
                        "style": tile.procedural.style,
                        "kind": tile.procedural.kind,
                        "weight": tile.procedural.weight,
                    },
                    "has_module": tile.module.is_some(),
                    "has_particle_emitter": tile.particle_emitter.is_some(),
                    "has_light_emitter": tile.light_emitter.is_some(),
                })
            })
            .collect::<Vec<_>>();
        tiles.sort_by(|a, b| {
            let alias_a = a["alias"].as_str().unwrap_or_default();
            let alias_b = b["alias"].as_str().unwrap_or_default();
            alias_a.cmp(alias_b).then_with(|| {
                a["id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(b["id"].as_str().unwrap_or_default())
            })
        });

        let mut tile_groups = self
            .project
            .tile_groups
            .values()
            .map(|group| {
                let members = group
                    .members
                    .iter()
                    .map(|member| {
                        serde_json::json!({
                            "tile_id": member.tile_id.to_string(),
                            "x": member.x,
                            "y": member.y,
                        })
                    })
                    .collect::<Vec<_>>();

                serde_json::json!({
                    "id": group.id.to_string(),
                    "name": group.name,
                    "width": group.width,
                    "height": group.height,
                    "tags": group.tags,
                    "members": members,
                })
            })
            .collect::<Vec<_>>();
        tile_groups.sort_by(|a, b| {
            let name_a = a["name"].as_str().unwrap_or_default();
            let name_b = b["name"].as_str().unwrap_or_default();
            name_a.cmp(name_b).then_with(|| {
                a["id"]
                    .as_str()
                    .unwrap_or_default()
                    .cmp(b["id"].as_str().unwrap_or_default())
            })
        });

        let roles = rusterix::TileRole::iterator()
            .map(|role| role.to_string())
            .collect::<Vec<_>>();

        serde_json::json!({
            "roles": roles,
            "tiles": tiles,
            "tile_groups": tile_groups,
            "counts": {
                "tiles": self.project.tiles.len(),
                "tile_groups": self.project.tile_groups.len(),
            }
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_tile_summary(&self, tile_id: &Uuid) -> Option<serde_json::Value> {
        self.project.tiles.get(tile_id).map(|tile| {
            let first_frame = tile.textures.first().map(|texture| {
                serde_json::json!({
                    "width": texture.width,
                    "height": texture.height,
                })
            });

            serde_json::json!({
                "id": tile.id.to_string(),
                "alias": tile.alias,
                "role": tile.role.to_string(),
                "blocking": tile.blocking,
                "frame_count": tile.textures.len(),
                "first_frame": first_frame,
                "procedural": {
                    "style": tile.procedural.style,
                    "kind": tile.procedural.kind,
                    "weight": tile.procedural.weight,
                },
            })
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_source_summary(&self, source: &rusterix::PixelSource) -> serde_json::Value {
        use rusterix::PixelSource;

        let mut summary = match source {
            PixelSource::Off => serde_json::json!({ "kind": "off" }),
            PixelSource::TileId(tile_id) => serde_json::json!({
                "kind": "tile_id",
                "tile_id": tile_id.to_string(),
            }),
            PixelSource::TileGroup(group_id) => {
                let group = self.project.tile_groups.get(group_id);
                serde_json::json!({
                    "kind": "tile_group",
                    "group_id": group_id.to_string(),
                    "group": group.map(|group| serde_json::json!({
                        "id": group.id.to_string(),
                        "name": group.name,
                        "width": group.width,
                        "height": group.height,
                        "member_count": group.members.len(),
                        "tags": group.tags,
                    })),
                })
            }
            PixelSource::TileGroupMember {
                group_id,
                member_index,
            } => {
                let member = self
                    .project
                    .tile_groups
                    .get(group_id)
                    .and_then(|group| group.members.get(*member_index as usize));
                serde_json::json!({
                    "kind": "tile_group_member",
                    "group_id": group_id.to_string(),
                    "member_index": member_index,
                    "tile_id": member.map(|member| member.tile_id.to_string()),
                    "member": member.map(|member| serde_json::json!({
                        "x": member.x,
                        "y": member.y,
                    })),
                })
            }
            PixelSource::ProceduralTile(tile_id) => serde_json::json!({
                "kind": "procedural_tile",
                "tile_id": tile_id.to_string(),
            }),
            PixelSource::PaletteIndex(index) => serde_json::json!({
                "kind": "palette_index",
                "index": index,
            }),
            PixelSource::MaterialId(material_id) => serde_json::json!({
                "kind": "material_id",
                "material_id": material_id.to_string(),
            }),
            PixelSource::Sequence(name) => serde_json::json!({
                "kind": "sequence",
                "name": name,
            }),
            PixelSource::EntityTile(entity_id, tile_index) => serde_json::json!({
                "kind": "entity_tile",
                "entity_id": entity_id,
                "tile_index": tile_index,
            }),
            PixelSource::ItemTile(item_id, tile_index) => serde_json::json!({
                "kind": "item_tile",
                "item_id": item_id,
                "tile_index": tile_index,
            }),
            PixelSource::Color(color) => serde_json::json!({
                "kind": "color",
                "rgba": color.to_u8_array(),
            }),
            PixelSource::LegacyShapeFXGraphId(graph_id) => serde_json::json!({
                "kind": "legacy_shape_fx_graph_id",
                "graph_id": graph_id.to_string(),
            }),
            PixelSource::StaticTileIndex(index) => serde_json::json!({
                "kind": "static_tile_index",
                "index": index,
            }),
            PixelSource::DynamicTileIndex(index) => serde_json::json!({
                "kind": "dynamic_tile_index",
                "index": index,
            }),
            PixelSource::Pixel(_) => serde_json::json!({
                "kind": "pixel",
            }),
        };

        let resolved_tile_id = match source {
            PixelSource::TileId(tile_id)
            | PixelSource::ProceduralTile(tile_id)
            | PixelSource::MaterialId(tile_id) => Some(*tile_id),
            PixelSource::TileGroupMember {
                group_id,
                member_index,
            } => self
                .project
                .tile_groups
                .get(group_id)
                .and_then(|group| group.members.get(*member_index as usize))
                .map(|member| member.tile_id),
            _ => None,
        };

        if let Some(tile_id) = resolved_tile_id
            && let Some(tile) = self.scepter_tile_summary(&tile_id)
            && let Some(object) = summary.as_object_mut()
        {
            object.insert("resolved_tile".to_string(), tile);
        }

        summary
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_resolved_tile_id(&self, source: &rusterix::PixelSource) -> Option<Uuid> {
        use rusterix::PixelSource;

        match source {
            PixelSource::TileId(tile_id)
            | PixelSource::ProceduralTile(tile_id)
            | PixelSource::MaterialId(tile_id) => Some(*tile_id),
            PixelSource::TileGroupMember {
                group_id,
                member_index,
            } => self
                .project
                .tile_groups
                .get(group_id)
                .and_then(|group| group.members.get(*member_index as usize))
                .map(|member| member.tile_id),
            _ => None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_value_snapshot(&self, value: &Value) -> serde_json::Value {
        match value {
            Value::Source(source) => serde_json::json!({
                "type": "source",
                "source": self.scepter_source_summary(source),
            }),
            Value::TileOverrides(tiles) => {
                let mut entries = tiles
                    .iter()
                    .map(|((x, y), source)| {
                        serde_json::json!({
                            "cell": [x, y],
                            "source": self.scepter_source_summary(source),
                        })
                    })
                    .collect::<Vec<_>>();
                entries.sort_by_key(|entry| {
                    (
                        entry["cell"][0].as_i64().unwrap_or_default(),
                        entry["cell"][1].as_i64().unwrap_or_default(),
                    )
                });
                serde_json::json!({
                    "type": "tile_overrides",
                    "entries": entries,
                })
            }
            Value::BlendOverrides(blend_tiles) => {
                let mut entries = blend_tiles
                    .iter()
                    .map(|((x, y), (preset, source))| {
                        serde_json::json!({
                            "cell": [x, y],
                            "preset": serde_json::to_value(preset)
                                .unwrap_or_else(|_| serde_json::json!(format!("{preset:?}"))),
                            "source": self.scepter_source_summary(source),
                        })
                    })
                    .collect::<Vec<_>>();
                entries.sort_by_key(|entry| {
                    (
                        entry["cell"][0].as_i64().unwrap_or_default(),
                        entry["cell"][1].as_i64().unwrap_or_default(),
                    )
                });
                serde_json::json!({
                    "type": "blend_overrides",
                    "entries": entries,
                })
            }
            _ => {
                serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!(value.to_string()))
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_properties_snapshot(&self, properties: &ValueContainer) -> serde_json::Value {
        let mut values = serde_json::Map::new();
        for key in properties.keys_sorted() {
            if let Some(value) = properties.get(key) {
                values.insert(key.clone(), self.scepter_value_snapshot(value));
            }
        }
        serde_json::Value::Object(values)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_region_snapshot(&self, request: &ScepterRegionRequest) -> serde_json::Value {
        let region = if let Some(id) = &request.id {
            Uuid::from_str(id)
                .ok()
                .and_then(|id| self.project.regions.iter().find(|region| region.id == id))
        } else if let Some(name) = &request.name {
            self.project
                .regions
                .iter()
                .find(|region| region.name.eq_ignore_ascii_case(name))
        } else {
            self.project
                .regions
                .iter()
                .find(|region| region.id == self.server_ctx.curr_region)
                .or_else(|| self.project.regions.first())
        };

        let Some(region) = region else {
            return serde_json::json!({
                "error": "region not found",
                "request": {
                    "id": request.id,
                    "name": request.name,
                }
            });
        };

        let vertices = region
            .map
            .vertices
            .iter()
            .map(|vertex| {
                serde_json::json!({
                    "id": vertex.id,
                    "name": vertex.name,
                    "position": [vertex.x, vertex.y, vertex.z],
                    "properties": self.scepter_properties_snapshot(&vertex.properties),
                })
            })
            .collect::<Vec<_>>();

        let linedefs = region
            .map
            .linedefs
            .iter()
            .map(|linedef| {
                let start = region.map.find_vertex(linedef.start_vertex);
                let end = region.map.find_vertex(linedef.end_vertex);
                serde_json::json!({
                    "id": linedef.id,
                    "creator_id": linedef.creator_id.to_string(),
                    "name": linedef.name,
                    "start_vertex": linedef.start_vertex,
                    "end_vertex": linedef.end_vertex,
                    "start": start.map(|vertex| serde_json::json!([vertex.x, vertex.y, vertex.z])),
                    "end": end.map(|vertex| serde_json::json!([vertex.x, vertex.y, vertex.z])),
                    "sector_ids": linedef.sector_ids,
                    "length": linedef.length(&region.map),
                    "properties": self.scepter_properties_snapshot(&linedef.properties),
                })
            })
            .collect::<Vec<_>>();

        let sectors = region
            .map
            .sectors
            .iter()
            .map(|sector| {
                let polygon = sector
                    .linedefs
                    .iter()
                    .filter_map(|linedef_id| {
                        let linedef = region.map.find_linedef(*linedef_id)?;
                        let vertex = region.map.find_vertex(linedef.start_vertex)?;
                        Some(serde_json::json!([vertex.x, vertex.y, vertex.z]))
                    })
                    .collect::<Vec<_>>();
                let bbox = sector.bounding_box(&region.map);
                let center = sector.center(&region.map);

                serde_json::json!({
                    "id": sector.id,
                    "creator_id": sector.creator_id.to_string(),
                    "name": sector.name,
                    "layer": sector.layer,
                    "linedefs": sector.linedefs,
                    "polygon": polygon,
                    "bbox": {
                        "min": [bbox.min.x, bbox.min.y],
                        "max": [bbox.max.x, bbox.max.y],
                    },
                    "center": center.map(|center| serde_json::json!([center.x, center.y])),
                    "area": sector.area(&region.map),
                    "properties": self.scepter_properties_snapshot(&sector.properties),
                })
            })
            .collect::<Vec<_>>();

        let characters = region
            .characters
            .values()
            .map(|character| {
                serde_json::json!({
                    "id": character.id.to_string(),
                    "template_id": character.character_id.to_string(),
                    "name": character.name,
                    "position": [character.position.x, character.position.y, character.position.z],
                    "orientation": [character.orientation.x, character.orientation.y],
                    "source_len": character.source.len(),
                    "data_len": character.data.len(),
                })
            })
            .collect::<Vec<_>>();

        let items = region
            .items
            .values()
            .map(|item| {
                serde_json::json!({
                    "id": item.id.to_string(),
                    "template_id": item.item_id.to_string(),
                    "name": item.name,
                    "position": [item.position.x, item.position.y, item.position.z],
                    "source_len": item.source.len(),
                    "data_len": item.data.len(),
                })
            })
            .collect::<Vec<_>>();

        let mut body = serde_json::json!({
            "id": region.id.to_string(),
            "name": region.name,
            "map": {
                "id": region.map.id.to_string(),
                "name": region.map.name,
                "camera": format!("{:?}", region.map.camera),
                "grid_size": region.map.grid_size,
                "subdivisions": region.map.subdivisions,
                "authoring_notes": {
                    "primary_2d_surface_source_key": "source",
                    "coordinate_system": "2D origin uses x right, negative y up, positive y down",
                    "ceiling_source": "screen/button selected-state legacy usage; not current 2D map authoring",
                    "terrain": "deprecated in current form; defer Scepter terrain commands until the replacement terrain system exists",
                },
                "vertices": vertices,
                "linedefs": linedefs,
                "sectors": sectors,
                "characters": characters,
                "items": items,
                "counts": {
                    "vertices": region.map.vertices.len(),
                    "linedefs": region.map.linedefs.len(),
                    "sectors": region.map.sectors.len(),
                    "geometry_objects": region.map.geometry_objects.len(),
                    "lights": region.map.lights.len(),
                    "entities": region.map.entities.len(),
                    "items": region.map.items.len(),
                    "region_characters": region.characters.len(),
                    "region_items": region.items.len(),
                },
            },
        });

        if request.include_tiles
            && let Some(object) = body.as_object_mut()
        {
            object.insert("tile_lookup".to_string(), self.scepter_tiles_snapshot());
        }

        body
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_source_overview_char(&self, source: Option<&rusterix::PixelSource>) -> char {
        let Some(source) = source else {
            return ' ';
        };
        let Some(tile_id) = self.scepter_resolved_tile_id(source) else {
            return ' ';
        };
        let Some(tile) = self.project.tiles.get(&tile_id) else {
            return '?';
        };

        match tile.procedural.kind.as_str() {
            "entrance" => 'E',
            "exit" => 'X',
            "wall" => '#',
            "floor" => '.',
            "door" => 'D',
            _ => match tile.role {
                rusterix::TileRole::Water => '~',
                rusterix::TileRole::Mountain => '^',
                rusterix::TileRole::Road => '=',
                rusterix::TileRole::Nature => {
                    if tile.blocking {
                        'T'
                    } else {
                        ','
                    }
                }
                rusterix::TileRole::ManMade => {
                    if tile.blocking {
                        '#'
                    } else {
                        '.'
                    }
                }
                rusterix::TileRole::Dungeon => {
                    if tile.blocking {
                        '#'
                    } else {
                        '.'
                    }
                }
                _ => '?',
            },
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_base64_encode(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);

        for chunk in bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = chunk.get(1).copied().unwrap_or(0);
            let b2 = chunk.get(2).copied().unwrap_or(0);
            let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;

            encoded.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
            encoded.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
            if chunk.len() > 1 {
                encoded.push(TABLE[((triple >> 6) & 0x3f) as usize] as char);
            } else {
                encoded.push('=');
            }
            if chunk.len() > 2 {
                encoded.push(TABLE[(triple & 0x3f) as usize] as char);
            } else {
                encoded.push('=');
            }
        }

        encoded
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_region_render_preview(&self, command: &RegionRenderPreview) -> serde_json::Value {
        let region_index = match self.scepter_resolve_region_index(&command.region) {
            Ok(index) => index,
            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
        };
        let region = &self.project.regions[region_index];
        let map = &region.map;

        let (min_x, max_x, min_y, max_y) = if let Some([x, y, width, height]) = command.bounds {
            if width == 0 || height == 0 {
                return serde_json::json!({
                    "ok": false,
                    "error": "region.render_preview bounds require non-zero width and height",
                });
            }
            (
                x.min(x + width),
                x.max(x + width),
                y.min(y + height),
                y.max(y + height),
            )
        } else if map.vertices.is_empty() {
            return serde_json::json!({
                "ok": false,
                "error": "region has no geometry to render",
            });
        } else {
            let min_x = map
                .vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::INFINITY, f32::min)
                .floor() as i32;
            let max_x = map
                .vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil() as i32;
            let min_y = map
                .vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::INFINITY, f32::min)
                .floor() as i32;
            let max_y = map
                .vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil() as i32;
            (min_x, max_x, min_y, max_y)
        };

        let grid_width = (max_x - min_x).max(0) as usize;
        let grid_height = (max_y - min_y).max(0) as usize;
        if grid_width == 0 || grid_height == 0 {
            return serde_json::json!({
                "ok": false,
                "error": "region.render_preview resolved empty bounds",
            });
        }
        if grid_width > 128 || grid_height > 128 {
            return serde_json::json!({
                "ok": false,
                "error": "region.render_preview bounds exceed 128x128 cells",
                "bounds": {
                    "min": [min_x, min_y],
                    "max": [max_x, max_y],
                    "size": [grid_width, grid_height],
                }
            });
        }

        let zoom = command.zoom.unwrap_or(2).clamp(1, 8) as usize;
        let cell_pixels = 16usize * zoom;
        let width = grid_width * cell_pixels;
        let height = grid_height * cell_pixels;
        let mut rgb = vec![16u8; width * height * 3];

        for sector in &map.sectors {
            let Some(source) = sector
                .properties
                .get("source")
                .and_then(|value| value.to_source())
            else {
                continue;
            };
            let Some(tile_id) = self.scepter_resolved_tile_id(source) else {
                continue;
            };
            let Some(tile) = self.project.tiles.get(&tile_id) else {
                continue;
            };
            let Some(texture) = tile.textures.first() else {
                continue;
            };

            let bbox = sector.bounding_box(map);
            let sx0 = (bbox.min.x.floor() as i32).max(min_x);
            let sx1 = (bbox.max.x.ceil() as i32).min(max_x);
            let sy0 = (bbox.min.y.floor() as i32).max(min_y);
            let sy1 = (bbox.max.y.ceil() as i32).min(max_y);

            for cell_y in sy0..sy1 {
                for cell_x in sx0..sx1 {
                    let cell_origin_x = (cell_x - min_x) as usize * cell_pixels;
                    let cell_origin_y = (cell_y - min_y) as usize * cell_pixels;

                    for py in 0..cell_pixels {
                        let ty = py * texture.height / cell_pixels;
                        for px in 0..cell_pixels {
                            let tx = px * texture.width / cell_pixels;
                            let source_index = (ty * texture.width + tx) * 4;
                            if source_index + 3 >= texture.data.len() {
                                continue;
                            }

                            let alpha = texture.data[source_index + 3] as u16;
                            if alpha == 0 {
                                continue;
                            }
                            let image_x = cell_origin_x + px;
                            let image_y = cell_origin_y + py;
                            let target_index = (image_y * width + image_x) * 3;
                            for channel in 0..3 {
                                let src = texture.data[source_index + channel] as u16;
                                let dst = rgb[target_index + channel] as u16;
                                rgb[target_index + channel] =
                                    ((src * alpha + dst * (255 - alpha)) / 255) as u8;
                            }
                        }
                    }
                }
            }
        }

        let mut png_data = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_data, width as u32, height as u32);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            match encoder.write_header() {
                Ok(mut writer) => {
                    if let Err(error) = writer.write_image_data(&rgb) {
                        return serde_json::json!({
                            "ok": false,
                            "error": format!("could not encode preview PNG: {error}"),
                        });
                    }
                }
                Err(error) => {
                    return serde_json::json!({
                        "ok": false,
                        "error": format!("could not write preview PNG header: {error}"),
                    });
                }
            }
        }

        serde_json::json!({
            "ok": true,
            "region": {
                "id": region.id.to_string(),
                "name": region.name,
            },
            "bounds": {
                "min": [min_x, min_y],
                "max": [max_x, max_y],
                "size": [grid_width, grid_height],
                "coordinate_system": "x right, negative y up; first image row is min_y/up",
            },
            "image": {
                "mime": "image/png",
                "encoding": "base64",
                "data": Self::scepter_base64_encode(&png_data),
                "width": width,
                "height": height,
                "grid_width": grid_width,
                "grid_height": grid_height,
                "cell_pixels": cell_pixels,
                "bytes": png_data.len(),
            }
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_region_summary(&self, request: &ScepterRegionRequest) -> serde_json::Value {
        let region = if let Some(id) = &request.id {
            Uuid::from_str(id)
                .ok()
                .and_then(|id| self.project.regions.iter().find(|region| region.id == id))
        } else if let Some(name) = &request.name {
            self.project
                .regions
                .iter()
                .find(|region| region.name.eq_ignore_ascii_case(name))
        } else {
            self.project
                .regions
                .iter()
                .find(|region| region.id == self.server_ctx.curr_region)
                .or_else(|| self.project.regions.first())
        };

        let Some(region) = region else {
            return serde_json::json!({
                "error": "region not found",
                "request": {
                    "id": request.id,
                    "name": request.name,
                }
            });
        };

        let bounds = if region.map.vertices.is_empty() {
            None
        } else {
            let min_x = region
                .map
                .vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::INFINITY, f32::min);
            let max_x = region
                .map
                .vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::NEG_INFINITY, f32::max);
            let min_y = region
                .map
                .vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::INFINITY, f32::min);
            let max_y = region
                .map
                .vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::NEG_INFINITY, f32::max);
            Some((min_x, max_x, min_y, max_y))
        };

        let mut layer_counts: HashMap<String, usize> = HashMap::new();
        let mut sector_source_counts: HashMap<Uuid, usize> = HashMap::new();
        let mut linedef_source_counts: HashMap<Uuid, usize> = HashMap::new();
        let mut role_counts: HashMap<String, (usize, usize)> = HashMap::new();
        let mut kind_counts: HashMap<String, usize> = HashMap::new();
        let mut off_sector_count = 0usize;
        let mut named_sectors = Vec::new();
        let mut procedural_sectors: HashMap<String, usize> = HashMap::new();

        for sector in &region.map.sectors {
            *layer_counts
                .entry(
                    sector
                        .layer
                        .map(|layer| layer.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                )
                .or_default() += 1;

            if let Some(Value::Source(source)) = sector.properties.get("source") {
                if let Some(tile_id) = self.scepter_resolved_tile_id(source) {
                    *sector_source_counts.entry(tile_id).or_default() += 1;
                    if let Some(tile) = self.project.tiles.get(&tile_id) {
                        let role = tile.role.to_string().to_string();
                        let entry = role_counts.entry(role).or_default();
                        entry.0 += 1;
                        if tile.blocking {
                            entry.1 += 1;
                        }
                        if !tile.procedural.kind.is_empty() {
                            *kind_counts.entry(tile.procedural.kind.clone()).or_default() += 1;
                        }
                    }
                } else if matches!(source, rusterix::PixelSource::Off) {
                    off_sector_count += 1;
                }
            }

            if sector
                .properties
                .get_bool_default("procedural_generated", false)
            {
                let kind = sector
                    .properties
                    .get_str("procedural_kind")
                    .unwrap_or("unknown")
                    .to_string();
                *procedural_sectors.entry(kind).or_default() += 1;
            }

            if !sector.name.is_empty() {
                let bbox = sector.bounding_box(&region.map);
                let center = sector.center(&region.map);
                named_sectors.push(serde_json::json!({
                    "id": sector.id,
                    "name": sector.name,
                    "layer": sector.layer,
                    "bbox": {
                        "min": [bbox.min.x, bbox.min.y],
                        "max": [bbox.max.x, bbox.max.y],
                    },
                    "center": center.map(|center| serde_json::json!([center.x, center.y])),
                    "source": sector
                        .properties
                        .get("source")
                        .and_then(|value| value.to_source())
                        .map(|source| self.scepter_source_summary(source)),
                    "data": sector.properties.get_str("data"),
                }));
            }
        }

        for linedef in &region.map.linedefs {
            for key in [
                "source",
                "row1_source",
                "row2_source",
                "row3_source",
                "row4_source",
            ] {
                if let Some(Value::Source(source)) = linedef.properties.get(key)
                    && let Some(tile_id) = self.scepter_resolved_tile_id(source)
                {
                    *linedef_source_counts.entry(tile_id).or_default() += 1;
                }
            }
        }

        let source_usage = |counts: HashMap<Uuid, usize>| {
            let mut usage = counts
                .into_iter()
                .map(|(tile_id, count)| {
                    serde_json::json!({
                        "tile_id": tile_id.to_string(),
                        "count": count,
                        "tile": self.scepter_tile_summary(&tile_id),
                    })
                })
                .collect::<Vec<_>>();
            usage.sort_by(|a, b| {
                b["count"]
                    .as_u64()
                    .unwrap_or_default()
                    .cmp(&a["count"].as_u64().unwrap_or_default())
            });
            usage.truncate(30);
            usage
        };

        let mut layers = layer_counts
            .into_iter()
            .map(|(layer, count)| serde_json::json!({ "layer": layer, "count": count }))
            .collect::<Vec<_>>();
        layers.sort_by(|a, b| {
            a["layer"]
                .as_str()
                .unwrap_or_default()
                .cmp(b["layer"].as_str().unwrap_or_default())
        });

        let mut roles = role_counts
            .into_iter()
            .map(|(role, (count, blocking_count))| {
                serde_json::json!({
                    "role": role,
                    "count": count,
                    "blocking": blocking_count,
                    "walkable": count.saturating_sub(blocking_count),
                })
            })
            .collect::<Vec<_>>();
        roles.sort_by(|a, b| {
            b["count"]
                .as_u64()
                .unwrap_or_default()
                .cmp(&a["count"].as_u64().unwrap_or_default())
        });

        let mut kinds = kind_counts
            .into_iter()
            .map(|(kind, count)| serde_json::json!({ "kind": kind, "count": count }))
            .collect::<Vec<_>>();
        kinds.sort_by(|a, b| {
            b["count"]
                .as_u64()
                .unwrap_or_default()
                .cmp(&a["count"].as_u64().unwrap_or_default())
        });

        let mut procedural = procedural_sectors
            .into_iter()
            .map(|(kind, count)| serde_json::json!({ "kind": kind, "count": count }))
            .collect::<Vec<_>>();
        procedural.sort_by(|a, b| {
            b["count"]
                .as_u64()
                .unwrap_or_default()
                .cmp(&a["count"].as_u64().unwrap_or_default())
        });

        let characters = region
            .characters
            .values()
            .map(|character| {
                serde_json::json!({
                    "id": character.id.to_string(),
                    "name": character.name,
                    "position": [character.position.x, character.position.y, character.position.z],
                    "orientation": [character.orientation.x, character.orientation.y],
                })
            })
            .collect::<Vec<_>>();

        let items = region
            .items
            .values()
            .map(|item| {
                serde_json::json!({
                    "id": item.id.to_string(),
                    "name": item.name,
                    "position": [item.position.x, item.position.y, item.position.z],
                })
            })
            .collect::<Vec<_>>();

        let overview = if request.include_ascii {
            bounds.map(|(min_x, max_x, min_y, max_y)| {
                let min_x_i = min_x.floor() as i32;
                let max_x_i = max_x.ceil() as i32;
                let min_y_i = min_y.floor() as i32;
                let max_y_i = max_y.ceil() as i32;
                let width = (max_x_i - min_x_i).max(0) as usize;
                let height = (max_y_i - min_y_i).max(0) as usize;
                let mut grid = vec![vec![' '; width]; height];

                if width <= 100 && height <= 100 {
                    for sector in &region.map.sectors {
                        let bbox = sector.bounding_box(&region.map);
                        let ch = self.scepter_source_overview_char(
                            sector
                                .properties
                                .get("source")
                                .and_then(|value| value.to_source()),
                        );
                        let sx0 = (bbox.min.x.floor() as i32).max(min_x_i);
                        let sx1 = (bbox.max.x.ceil() as i32).min(max_x_i);
                        let sy0 = (bbox.min.y.floor() as i32).max(min_y_i);
                        let sy1 = (bbox.max.y.ceil() as i32).min(max_y_i);
                        for y in sy0..sy1 {
                            for x in sx0..sx1 {
                                let gx = (x - min_x_i) as usize;
                                let gy = (y - min_y_i) as usize;
                                if let Some(row) = grid.get_mut(gy)
                                    && let Some(cell) = row.get_mut(gx)
                                {
                                    *cell = ch;
                                }
                            }
                        }
                    }

                    for character in region.characters.values() {
                        let x = character.position.x.floor() as i32 - min_x_i;
                        let y = character.position.z.floor() as i32 - min_y_i;
                        if let Some(row) = grid.get_mut(y as usize)
                            && let Some(cell) = row.get_mut(x as usize)
                        {
                            *cell = if character.name == "Player" { 'P' } else { 'C' };
                        }
                    }

                    for item in region.items.values() {
                        let x = item.position.x.floor() as i32 - min_x_i;
                        let y = item.position.z.floor() as i32 - min_y_i;
                        if let Some(row) = grid.get_mut(y as usize)
                            && let Some(cell) = row.get_mut(x as usize)
                        {
                            *cell = if item.name == "Door" { 'D' } else { 'i' };
                        }
                    }
                }

                let rows = if width <= 100 && height <= 100 {
                    grid.into_iter()
                        .map(|row| row.into_iter().collect::<String>())
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                serde_json::json!({
                    "bounds": {
                        "min": [min_x_i, min_y_i],
                        "max": [max_x_i, max_y_i],
                        "width": width,
                        "height": height,
                        "orientation": "first row is most negative y/up; later rows move downward toward positive y",
                    },
                    "legend": {
                        "~": "water",
                        "^": "mountain/blocking terrain",
                        "#": "blocking wall or manmade blocker",
                        ".": "floor/manmade walkable",
                        ",": "nature walkable",
                        "T": "blocking nature",
                        "=": "road/path",
                        "E": "entrance",
                        "X": "exit",
                        "P": "player",
                        "C": "character",
                        "D": "door item or door tile",
                        "i": "item"
                    },
                    "rows": rows,
                    "omitted": width > 100 || height > 100,
                })
            })
        } else {
            None
        };

        serde_json::json!({
            "id": region.id.to_string(),
            "name": region.name,
            "map": {
                "id": region.map.id.to_string(),
                "name": region.map.name,
                "camera": format!("{:?}", region.map.camera),
                "grid_size": region.map.grid_size,
                "subdivisions": region.map.subdivisions,
                "bounds": bounds.map(|(min_x, max_x, min_y, max_y)| serde_json::json!({
                    "min": [min_x, min_y],
                    "max": [max_x, max_y],
                    "size": [max_x - min_x, max_y - min_y],
                })),
                "counts": {
                    "vertices": region.map.vertices.len(),
                    "linedefs": region.map.linedefs.len(),
                    "sectors": region.map.sectors.len(),
                    "geometry_objects": region.map.geometry_objects.len(),
                    "characters": region.characters.len(),
                    "items": region.items.len(),
                    "off_sectors": off_sector_count,
                },
                "layers": layers,
                "tile_roles": roles,
                "procedural_kinds": kinds,
                "procedural_sectors": procedural,
                "sector_source_usage": source_usage(sector_source_counts),
                "linedef_source_usage": source_usage(linedef_source_counts),
                "named_sectors": named_sectors,
                "characters": characters,
                "items": items,
                "overview": overview,
                "authoring_notes": {
                    "primary_2d_surface_source_key": "source",
                    "coordinate_system": "2D origin uses x right, negative y up, positive y down",
                    "ceiling_source": "screen/button selected-state legacy usage; not current 2D map authoring",
                    "terrain": "deprecated in current form; defer Scepter terrain commands until the replacement terrain system exists",
                },
            },
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_resolve_region_index(&self, region: &RegionRef) -> Result<usize, String> {
        match region {
            RegionRef::Id { id } => {
                let id = Uuid::from_str(id).map_err(|err| format!("invalid region id: {err}"))?;
                self.project
                    .regions
                    .iter()
                    .position(|region| region.id == id)
                    .ok_or_else(|| format!("region id not found: {id}"))
            }
            RegionRef::Name { name } => self
                .project
                .regions
                .iter()
                .position(|region| region.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("region name not found: {name}")),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_normalize_match_text(value: &str) -> String {
        value
            .chars()
            .filter(|ch| !ch.is_whitespace() && *ch != '_' && *ch != '-')
            .flat_map(char::to_lowercase)
            .collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_resolve_tile_selector(&self, selector: &TileSelector) -> Result<Uuid, String> {
        if let Some(id) = &selector.id {
            let id = Uuid::from_str(id).map_err(|err| format!("invalid tile id: {err}"))?;
            if self.project.tiles.contains_key(&id) {
                return Ok(id);
            }
            return Err(format!("tile id not found: {id}"));
        }

        if let Some(alias) = &selector.alias
            && let Some(tile) = self
                .project
                .tiles
                .values()
                .find(|tile| tile.alias.eq_ignore_ascii_case(alias))
        {
            return Ok(tile.id);
        }

        let role = selector
            .role
            .as_deref()
            .map(Self::scepter_normalize_match_text);
        let kind = selector
            .kind
            .as_deref()
            .map(Self::scepter_normalize_match_text);
        let style = selector
            .style
            .as_deref()
            .map(Self::scepter_normalize_match_text);
        let tags = selector
            .tags
            .iter()
            .map(|tag| Self::scepter_normalize_match_text(tag))
            .collect::<Vec<_>>();

        self.project
            .tiles
            .values()
            .find(|tile| {
                role.as_ref().is_none_or(|role| {
                    Self::scepter_normalize_match_text(tile.role.to_string()) == *role
                }) && kind.as_ref().is_none_or(|kind| {
                    Self::scepter_normalize_match_text(&tile.procedural.kind) == *kind
                }) && style.as_ref().is_none_or(|style| {
                    Self::scepter_normalize_match_text(&tile.procedural.style) == *style
                }) && tags.iter().all(|tag| {
                    let alias = Self::scepter_normalize_match_text(&tile.alias);
                    let kind = Self::scepter_normalize_match_text(&tile.procedural.kind);
                    let style = Self::scepter_normalize_match_text(&tile.procedural.style);
                    alias.contains(tag) || kind.contains(tag) || style.contains(tag)
                })
            })
            .map(|tile| tile.id)
            .ok_or_else(|| format!("no tile matched selector: {selector:?}"))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_find_rect_sector(map: &rusterix::Map, expected: &[(f32, f32); 4]) -> Option<u32> {
        Self::scepter_find_rect_sectors(map, expected)
            .into_iter()
            .next()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_find_rect_sectors(map: &rusterix::Map, expected: &[(f32, f32); 4]) -> Vec<u32> {
        map.sectors
            .iter()
            .filter_map(|sector| {
                let mut points = sector
                    .linedefs
                    .iter()
                    .filter_map(|linedef_id| {
                        let linedef = map.find_linedef(*linedef_id)?;
                        let vertex = map.find_vertex(linedef.start_vertex)?;
                        Some((vertex.x, vertex.y))
                    })
                    .collect::<Vec<_>>();
                points.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
                points.dedup();

                let mut expected = expected.to_vec();
                expected.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
                expected.dedup();

                if points == expected {
                    Some(sector.id)
                } else {
                    None
                }
            })
            .collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_find_cell_replacement_sectors(map: &rusterix::Map, x: i32, y: i32) -> Vec<u32> {
        let x0 = x as f32;
        let y0 = y as f32;
        let x1 = x0 + 1.0;
        let y1 = y0 + 1.0;
        let epsilon = 0.0001;

        map.sectors
            .iter()
            .filter_map(|sector| {
                let source = sector.properties.get_default_source()?;
                if matches!(source, rusterix::PixelSource::Off) {
                    return None;
                }

                let mut min_x = f32::MAX;
                let mut min_y = f32::MAX;
                let mut max_x = f32::MIN;
                let mut max_y = f32::MIN;
                for linedef_id in &sector.linedefs {
                    let linedef = map.find_linedef(*linedef_id)?;
                    let vertex = map.find_vertex(linedef.start_vertex)?;
                    min_x = min_x.min(vertex.x);
                    min_y = min_y.min(vertex.y);
                    max_x = max_x.max(vertex.x);
                    max_y = max_y.max(vertex.y);
                }

                let overlaps = min_x < x1 - epsilon
                    && max_x > x0 + epsilon
                    && min_y < y1 - epsilon
                    && max_y > y0 + epsilon;

                overlaps.then_some(sector.id)
            })
            .collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_create_cell_sector(
        map: &mut rusterix::Map,
        x: i32,
        y: i32,
        source: rusterix::PixelSource,
        layer: u8,
    ) -> Option<u32> {
        let x0 = x as f32;
        let y0 = y as f32;
        let x1 = x0 + 1.0;
        let y1 = y0 + 1.0;
        let expected = [(x0, y0), (x0, y1), (x1, y1), (x1, y0)];

        let v0 = map.add_vertex_at(x0, y0);
        let v1 = map.add_vertex_at(x0, y1);
        let v2 = map.add_vertex_at(x1, y1);
        let v3 = map.add_vertex_at(x1, y0);

        map.possible_polygon.clear();
        let _ = map.create_linedef_manual(v0, v1);
        let _ = map.create_linedef_manual(v1, v2);
        let _ = map.create_linedef_manual(v2, v3);
        let _ = map.create_linedef_manual(v3, v0);

        let sector_id = map
            .close_polygon_manual()
            .or_else(|| Self::scepter_find_rect_sector(map, &expected))?;

        if let Some(sector) = map.find_sector_mut(sector_id) {
            sector.properties.set("rect", Value::Bool(true));
            sector.properties.set("source", Value::Source(source));
            sector.layer = Some(layer);
        }

        Some(sector_id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_region_paint_cells_batch(
        &mut self,
        region: &RegionRef,
        tile: &TileSelector,
        cells: &[GridPoint],
        layer: Option<&str>,
        select: bool,
        replace_existing: bool,
        command_name: &str,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        let region_index = match self.scepter_resolve_region_index(region) {
            Ok(index) => index,
            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
        };
        let tile_id = match self.scepter_resolve_tile_selector(tile) {
            Ok(tile_id) => tile_id,
            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
        };

        if cells.is_empty() {
            return serde_json::json!({
                "ok": false,
                "error": format!("{command_name} requires at least one cell"),
            });
        }

        let layer = layer
            .and_then(|layer| layer.parse::<u8>().ok())
            .unwrap_or(1);
        let source = rusterix::PixelSource::TileId(tile_id);
        let region_id = self.project.regions[region_index].id;
        let old_map = self.project.regions[region_index].map.clone();

        let mut created_sector_ids = Vec::new();
        let mut replaced_sector_ids = Vec::new();
        {
            let map = &mut self.project.regions[region_index].map;

            for [x, y] in cells {
                if replace_existing {
                    let sectors = Self::scepter_find_cell_replacement_sectors(map, *x, *y);
                    if !sectors.is_empty() {
                        let linedefs = sectors
                            .iter()
                            .filter_map(|sector_id| map.find_sector(*sector_id))
                            .flat_map(|sector| sector.linedefs.clone())
                            .collect::<Vec<_>>();
                        map.delete_elements(&[], &linedefs, &sectors);
                        replaced_sector_ids.extend(sectors);
                    }
                }

                match Self::scepter_create_cell_sector(map, *x, *y, source.clone(), layer) {
                    Some(sector_id) => created_sector_ids.push(sector_id),
                    None => {
                        return serde_json::json!({
                            "ok": false,
                            "error": format!("could not create cell sector at [{x}, {y}]"),
                            "created_sector_ids": created_sector_ids,
                            "replaced_sector_ids": replaced_sector_ids,
                        });
                    }
                }
            }

            if select {
                map.selected_vertices.clear();
                map.selected_linedefs.clear();
                map.selected_sectors = created_sector_ids.clone();
            }
            map.changed = map.changed.saturating_add(created_sector_ids.len() as u32);
        }

        let new_map = self.project.regions[region_index].map.clone();
        let undo_atom = ProjectUndoAtom::MapEdit(
            ProjectContext::Region(region_id),
            Box::new(old_map.clone()),
            Box::new(new_map.clone()),
        );
        editor_scene_apply_map_edit(&self.project, &self.server_ctx, &old_map, &new_map);
        update_region(ctx);
        UNDOMANAGER.write().unwrap().add_undo(undo_atom, ctx);

        serde_json::json!({
            "ok": true,
            "command": command_name,
            "region_id": region_id.to_string(),
            "sector_ids": created_sector_ids,
            "replaced_sector_ids": replaced_sector_ids,
            "tile_id": tile_id.to_string(),
            "cell_count": cells.len(),
            "layer": layer,
            "replace_existing": replace_existing,
            "tile": self.scepter_tile_summary(&tile_id),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_region_paint_rect(
        &mut self,
        command: RegionPaintRect,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        let [x, y, width, height] = command.rect;
        if width == 0 || height == 0 {
            return serde_json::json!({
                "ok": false,
                "error": "region.paint_rect requires non-zero width and height",
            });
        }

        let x0 = x.min(x + width);
        let x1 = x.max(x + width);
        let y0 = y.min(y + height);
        let y1 = y.max(y + height);
        let mut cells = Vec::new();
        for y in y0..y1 {
            for x in x0..x1 {
                cells.push([x, y]);
            }
        }

        self.scepter_apply_region_paint_cells_batch(
            &command.region,
            &command.tile,
            &cells,
            command.layer.as_deref(),
            command.select.unwrap_or(false),
            command.replace_existing.unwrap_or(true),
            "region.paint_rect",
            ctx,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_region_paint_cells(
        &mut self,
        command: RegionPaintCells,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        self.scepter_apply_region_paint_cells_batch(
            &command.region,
            &command.tile,
            &command.cells,
            command.layer.as_deref(),
            command.select.unwrap_or(false),
            command.replace_existing.unwrap_or(true),
            "region.paint_cells",
            ctx,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_script_target_region_index(
        &self,
        target: &ScriptTarget,
    ) -> Result<Option<usize>, String> {
        if let Some(region) = &target.region {
            return self.scepter_resolve_region_index(region).map(Some);
        }

        if target.kind == ScriptTargetKind::Region {
            if let Some(id) = &target.id {
                return self
                    .scepter_resolve_region_index(&RegionRef::Id { id: id.clone() })
                    .map(Some);
            }
            if let Some(name) = &target.name {
                return self
                    .scepter_resolve_region_index(&RegionRef::Name { name: name.clone() })
                    .map(Some);
            }
            return self
                .project
                .regions
                .iter()
                .position(|region| region.id == self.server_ctx.curr_region)
                .or(Some(0))
                .ok_or_else(|| "project has no regions".to_string())
                .map(Some);
        }

        Ok(None)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_target_match(id: &Uuid, name: &str, target: &ScriptTarget) -> Result<bool, String> {
        if let Some(target_id) = &target.id {
            let target_id =
                Uuid::from_str(target_id).map_err(|err| format!("invalid target id: {err}"))?;
            return Ok(*id == target_id);
        }

        if let Some(target_name) = &target.name {
            return Ok(name.eq_ignore_ascii_case(target_name));
        }

        Ok(false)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_target_missing_error(target: &ScriptTarget) -> String {
        match target.kind {
            ScriptTargetKind::World => "world target not found".to_string(),
            ScriptTargetKind::Region => "region target not found".to_string(),
            ScriptTargetKind::Character => {
                "character target requires an id or name and must exist".to_string()
            }
            ScriptTargetKind::Item => {
                "item target requires an id or name and must exist".to_string()
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_script_payload(
        kind: &str,
        scope: &str,
        id: Option<Uuid>,
        name: &str,
        source: &str,
        source_debug: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "ok": true,
            "kind": kind,
            "scope": scope,
            "id": id.map(|id| id.to_string()),
            "name": name,
            "source": source,
            "source_debug": source_debug,
            "source_len": source.len(),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_get_script(&self, command: &ScriptGet) -> serde_json::Value {
        let target = &command.target;
        match target.kind {
            ScriptTargetKind::World => Self::scepter_script_payload(
                "world",
                "project",
                None,
                "World",
                &self.project.world_source,
                &self.project.world_source_debug,
            ),
            ScriptTargetKind::Region => {
                let region_index = match self.scepter_script_target_region_index(target) {
                    Ok(Some(index)) => index,
                    Ok(None) => {
                        return serde_json::json!({ "ok": false, "error": "region not found" });
                    }
                    Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                };
                let region = &self.project.regions[region_index];
                Self::scepter_script_payload(
                    "region",
                    "project",
                    Some(region.id),
                    &region.name,
                    &region.source,
                    &region.source_debug,
                )
            }
            ScriptTargetKind::Character => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(target) {
                    let region = &self.project.regions[region_index];
                    if let Some(character) = region.characters.values().find(|character| {
                        Self::scepter_target_match(&character.id, &character.name, target)
                            .unwrap_or(false)
                    }) {
                        return Self::scepter_script_payload(
                            "character",
                            "region_instance",
                            Some(character.id),
                            &character.name,
                            &character.source,
                            &character.source_debug,
                        );
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if let Some(character) = self.project.characters.values().find(|character| {
                    Self::scepter_target_match(&character.id, &character.name, target)
                        .unwrap_or(false)
                }) {
                    return Self::scepter_script_payload(
                        "character",
                        "template",
                        Some(character.id),
                        &character.name,
                        &character.source,
                        &character.source_debug,
                    );
                }

                serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(target) })
            }
            ScriptTargetKind::Item => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(target) {
                    let region = &self.project.regions[region_index];
                    if let Some(item) = region.items.values().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, target).unwrap_or(false)
                    }) {
                        return Self::scepter_script_payload(
                            "item",
                            "region_instance",
                            Some(item.id),
                            &item.name,
                            &item.source,
                            &item.source_debug,
                        );
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if let Some(item) = self.project.items.values().find(|item| {
                    Self::scepter_target_match(&item.id, &item.name, target).unwrap_or(false)
                }) {
                    return Self::scepter_script_payload(
                        "item",
                        "template",
                        Some(item.id),
                        &item.name,
                        &item.source,
                        &item.source_debug,
                    );
                }

                serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(target) })
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_validate_eldrin_source(
        &self,
        target: &ScriptTarget,
        source: &str,
    ) -> serde_json::Value {
        serde_json::json!({
            "ok": true,
            "valid": true,
            "target": {
                "kind": format!("{:?}", target.kind).to_lowercase(),
                "id": target.id,
                "name": target.name,
                "region": target.region,
            },
            "source_len": source.len(),
            "diagnostics": [],
            "note": "Scepter currently stores Eldrin source and reports structural command validity; parser-backed diagnostics can be wired in a later pass.",
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_script_patch(
        &mut self,
        command: ScriptPatch,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        if command.validate {
            let validation = self.scepter_validate_eldrin_source(&command.target, &command.patch);
            if !validation
                .get("valid")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
            {
                return validation;
            }
        }

        let old_project = self.project.clone();
        let target = command.target;
        let new_source = command.patch;
        let mut changed = None::<(String, String, String)>;

        match target.kind {
            ScriptTargetKind::World => {
                self.project.world_source = new_source.clone();
                self.project.world_source_debug = new_source.clone();
                changed = Some((
                    "world".to_string(),
                    "project".to_string(),
                    "World".to_string(),
                ));
            }
            ScriptTargetKind::Region => {
                let region_index = match self.scepter_script_target_region_index(&target) {
                    Ok(Some(index)) => index,
                    Ok(None) => {
                        return serde_json::json!({ "ok": false, "error": "region not found" });
                    }
                    Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                };
                let region = &mut self.project.regions[region_index];
                region.source = new_source.clone();
                region.source_debug = new_source.clone();
                changed = Some((
                    "region".to_string(),
                    "project".to_string(),
                    region.name.clone(),
                ));
            }
            ScriptTargetKind::Character => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(&target) {
                    let region = &mut self.project.regions[region_index];
                    if let Some(character) = region.characters.values_mut().find(|character| {
                        Self::scepter_target_match(&character.id, &character.name, &target)
                            .unwrap_or(false)
                    }) {
                        character.source = new_source.clone();
                        character.source_debug = new_source.clone();
                        changed = Some((
                            "character".to_string(),
                            "region_instance".to_string(),
                            character.name.clone(),
                        ));
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(&target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if changed.is_none()
                    && let Some(character) =
                        self.project.characters.values_mut().find(|character| {
                            Self::scepter_target_match(&character.id, &character.name, &target)
                                .unwrap_or(false)
                        })
                {
                    character.source = new_source.clone();
                    character.source_debug = new_source.clone();
                    changed = Some((
                        "character".to_string(),
                        "template".to_string(),
                        character.name.clone(),
                    ));
                }
            }
            ScriptTargetKind::Item => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(&target) {
                    let region = &mut self.project.regions[region_index];
                    if let Some(item) = region.items.values_mut().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, &target).unwrap_or(false)
                    }) {
                        item.source = new_source.clone();
                        item.source_debug = new_source.clone();
                        changed = Some((
                            "item".to_string(),
                            "region_instance".to_string(),
                            item.name.clone(),
                        ));
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(&target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if changed.is_none()
                    && let Some(item) = self.project.items.values_mut().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, &target).unwrap_or(false)
                    })
                {
                    item.source = new_source.clone();
                    item.source_debug = new_source.clone();
                    changed = Some((
                        "item".to_string(),
                        "template".to_string(),
                        item.name.clone(),
                    ));
                }
            }
        }

        let Some((kind, scope, name)) = changed else {
            self.project = old_project;
            return serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(&target) });
        };

        let new_project = self.project.clone();
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                format!("Scepter Script Edit: {kind} {name}"),
                Box::new(old_project),
                Box::new(new_project),
            ),
            ctx,
        );
        shared::rusterix_utils::insert_content_into_maps(&mut self.project);
        update_region(ctx);

        serde_json::json!({
            "ok": true,
            "command": "script.patch",
            "mode": "replace_source",
            "kind": kind,
            "scope": scope,
            "name": name,
            "source_len": new_source.len(),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_json_to_toml(value: serde_json::Value) -> Result<toml::Value, String> {
        match value {
            serde_json::Value::Null => {
                Err("null is not a TOML value; use remove for deletion".to_string())
            }
            serde_json::Value::Bool(value) => Ok(toml::Value::Boolean(value)),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Ok(toml::Value::Integer(value))
                } else if let Some(value) = value.as_f64() {
                    Ok(toml::Value::Float(value))
                } else {
                    Err("number is outside TOML's supported range".to_string())
                }
            }
            serde_json::Value::String(value) => Ok(toml::Value::String(value)),
            serde_json::Value::Array(values) => values
                .into_iter()
                .map(Self::scepter_json_to_toml)
                .collect::<Result<Vec<_>, _>>()
                .map(toml::Value::Array),
            serde_json::Value::Object(values) => {
                let mut table = toml::Table::new();
                for (key, value) in values {
                    table.insert(key, Self::scepter_json_to_toml(value)?);
                }
                Ok(toml::Value::Table(table))
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_attributes_payload(
        kind: &str,
        scope: &str,
        id: Uuid,
        name: &str,
        data: &str,
    ) -> serde_json::Value {
        let parsed = data.parse::<toml::Table>();
        let (attributes, parse_error) = match parsed {
            Ok(table) => (
                table
                    .get("attributes")
                    .and_then(toml::Value::as_table)
                    .cloned()
                    .unwrap_or_default(),
                None,
            ),
            Err(error) => (toml::Table::new(), Some(error.to_string())),
        };

        serde_json::json!({
            "ok": parse_error.is_none(),
            "kind": kind,
            "scope": scope,
            "id": id.to_string(),
            "name": name,
            "data": data,
            "attributes": attributes,
            "parse_error": parse_error,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_get_attributes(&self, command: &AttributesGet) -> serde_json::Value {
        let target = &command.target;
        match target.kind {
            ScriptTargetKind::Character => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(target) {
                    let region = &self.project.regions[region_index];
                    if let Some(character) = region.characters.values().find(|character| {
                        Self::scepter_target_match(&character.id, &character.name, target)
                            .unwrap_or(false)
                    }) {
                        return Self::scepter_attributes_payload(
                            "character",
                            "region_instance",
                            character.id,
                            &character.name,
                            &character.data,
                        );
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if let Some(character) = self.project.characters.values().find(|character| {
                    Self::scepter_target_match(&character.id, &character.name, target)
                        .unwrap_or(false)
                }) {
                    return Self::scepter_attributes_payload(
                        "character",
                        "template",
                        character.id,
                        &character.name,
                        &character.data,
                    );
                }
            }
            ScriptTargetKind::Item => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(target) {
                    let region = &self.project.regions[region_index];
                    if let Some(item) = region.items.values().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, target).unwrap_or(false)
                    }) {
                        return Self::scepter_attributes_payload(
                            "item",
                            "region_instance",
                            item.id,
                            &item.name,
                            &item.data,
                        );
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if let Some(item) = self.project.items.values().find(|item| {
                    Self::scepter_target_match(&item.id, &item.name, target).unwrap_or(false)
                }) {
                    return Self::scepter_attributes_payload(
                        "item", "template", item.id, &item.name, &item.data,
                    );
                }
            }
            ScriptTargetKind::World | ScriptTargetKind::Region => {
                return serde_json::json!({
                    "ok": false,
                    "error": "attributes.get currently supports character and item targets",
                });
            }
        }

        serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(target) })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_patch_data_source(
        data: &str,
        values: serde_json::Map<String, serde_json::Value>,
        remove: &[String],
    ) -> Result<(String, Vec<String>, Vec<String>), String> {
        let mut table = if data.trim().is_empty() {
            toml::Table::new()
        } else {
            data.parse::<toml::Table>()
                .map_err(|err| format!("existing TOML data is invalid: {err}"))?
        };

        let attributes_value = table
            .entry("attributes".to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if !attributes_value.is_table() {
            return Err("[attributes] exists but is not a TOML table".to_string());
        }
        let attributes = attributes_value
            .as_table_mut()
            .ok_or_else(|| "could not access [attributes] table".to_string())?;

        let mut changed = Vec::new();
        for (key, value) in values {
            attributes.insert(key.clone(), Self::scepter_json_to_toml(value)?);
            changed.push(key);
        }

        let mut removed = Vec::new();
        for key in remove {
            if attributes.remove(key).is_some() {
                removed.push(key.clone());
            }
        }

        let source = toml::to_string_pretty(&table)
            .map_err(|err| format!("could not serialize TOML data: {err}"))?;
        Ok((source, changed, removed))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scepter_apply_attributes_patch(
        &mut self,
        command: AttributesPatch,
        ctx: &mut TheContext,
    ) -> serde_json::Value {
        let old_project = self.project.clone();
        let target = command.target;
        let values = command.values;
        let remove = command.remove;
        let mut changed = None::<(String, String, String, Vec<String>, Vec<String>)>;

        match target.kind {
            ScriptTargetKind::Character => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(&target) {
                    let region = &mut self.project.regions[region_index];
                    if let Some(character) = region.characters.values_mut().find(|character| {
                        Self::scepter_target_match(&character.id, &character.name, &target)
                            .unwrap_or(false)
                    }) {
                        match Self::scepter_patch_data_source(
                            &character.data,
                            values.clone(),
                            &remove,
                        ) {
                            Ok((data, keys, removed)) => {
                                if command.validate
                                    && let Err(err) = data.parse::<toml::Table>()
                                {
                                    return serde_json::json!({ "ok": false, "error": format!("patched TOML is invalid: {err}") });
                                }
                                character.data = data;
                                changed = Some((
                                    "character".to_string(),
                                    "region_instance".to_string(),
                                    character.name.clone(),
                                    keys,
                                    removed,
                                ));
                            }
                            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                        }
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(&target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if changed.is_none()
                    && let Some(character) =
                        self.project.characters.values_mut().find(|character| {
                            Self::scepter_target_match(&character.id, &character.name, &target)
                                .unwrap_or(false)
                        })
                {
                    match Self::scepter_patch_data_source(&character.data, values, &remove) {
                        Ok((data, keys, removed)) => {
                            if command.validate
                                && let Err(err) = data.parse::<toml::Table>()
                            {
                                return serde_json::json!({ "ok": false, "error": format!("patched TOML is invalid: {err}") });
                            }
                            character.data = data;
                            changed = Some((
                                "character".to_string(),
                                "template".to_string(),
                                character.name.clone(),
                                keys,
                                removed,
                            ));
                        }
                        Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                    }
                }
            }
            ScriptTargetKind::Item => {
                if let Ok(Some(region_index)) = self.scepter_script_target_region_index(&target) {
                    let region = &mut self.project.regions[region_index];
                    if let Some(item) = region.items.values_mut().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, &target).unwrap_or(false)
                    }) {
                        match Self::scepter_patch_data_source(&item.data, values.clone(), &remove) {
                            Ok((data, keys, removed)) => {
                                if command.validate
                                    && let Err(err) = data.parse::<toml::Table>()
                                {
                                    return serde_json::json!({ "ok": false, "error": format!("patched TOML is invalid: {err}") });
                                }
                                item.data = data;
                                changed = Some((
                                    "item".to_string(),
                                    "region_instance".to_string(),
                                    item.name.clone(),
                                    keys,
                                    removed,
                                ));
                            }
                            Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                        }
                    }
                } else if let Err(error) = self.scepter_script_target_region_index(&target) {
                    return serde_json::json!({ "ok": false, "error": error });
                }

                if changed.is_none()
                    && let Some(item) = self.project.items.values_mut().find(|item| {
                        Self::scepter_target_match(&item.id, &item.name, &target).unwrap_or(false)
                    })
                {
                    match Self::scepter_patch_data_source(&item.data, values, &remove) {
                        Ok((data, keys, removed)) => {
                            if command.validate
                                && let Err(err) = data.parse::<toml::Table>()
                            {
                                return serde_json::json!({ "ok": false, "error": format!("patched TOML is invalid: {err}") });
                            }
                            item.data = data;
                            changed = Some((
                                "item".to_string(),
                                "template".to_string(),
                                item.name.clone(),
                                keys,
                                removed,
                            ));
                        }
                        Err(error) => return serde_json::json!({ "ok": false, "error": error }),
                    }
                }
            }
            ScriptTargetKind::World | ScriptTargetKind::Region => {
                return serde_json::json!({
                    "ok": false,
                    "error": "attributes.patch currently supports character and item targets",
                });
            }
        }

        let Some((kind, scope, name, changed_keys, removed_keys)) = changed else {
            self.project = old_project;
            return serde_json::json!({ "ok": false, "error": Self::scepter_target_missing_error(&target) });
        };

        let new_project = self.project.clone();
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                format!("Scepter Attribute Edit: {kind} {name}"),
                Box::new(old_project),
                Box::new(new_project),
            ),
            ctx,
        );
        shared::rusterix_utils::insert_content_into_maps(&mut self.project);
        update_region(ctx);

        serde_json::json!({
            "ok": true,
            "command": "attributes.patch",
            "kind": kind,
            "scope": scope,
            "name": name,
            "changed": changed_keys,
            "removed": removed_keys,
        })
    }

    fn is_realtime_mode(&self) -> bool {
        self.server_ctx.game_mode
            || RUSTERIX.read().unwrap().server.state == rusterix::ServerState::Running
    }

    fn firstp_editor_camera_moving(&self) -> bool {
        self.server_ctx.editor_view_mode == EditorViewMode::FirstP
            && (self.server_ctx.editor_fly_nav_active
                || EDITCAMERA.read().unwrap().move_action.is_some())
    }

    fn redraw_interval_ms(&self) -> u64 {
        let config = CONFIGEDITOR.read().unwrap();
        if self.is_realtime_mode() || self.firstp_editor_camera_moving() {
            (1000 / config.target_fps.clamp(1, 60)) as u64
        } else {
            config.game_tick_ms.max(1) as u64
        }
    }

    fn help_url_for_data_context(&self) -> String {
        match self.server_ctx.pc {
            ProjectContext::ProjectSettings => "docs/configuration/game".to_string(),
            ProjectContext::GameRules | ProjectContext::GameLocales => "docs/rules".to_string(),
            ProjectContext::GameAudioFx => "docs/audio".to_string(),
            ProjectContext::GameAuthoring | ProjectContext::GameShortcuts => {
                "docs/creator/tools/overview".to_string()
            }
            ProjectContext::RegionSettings(_) => "docs/building_maps/region_settings".to_string(),
            ProjectContext::CharacterPreviewRigging(_) => "docs/characters_items/rigging".into(),
            ProjectContext::Character(_)
            | ProjectContext::CharacterData(_)
            | ProjectContext::Item(_)
            | ProjectContext::ItemData(_) => "docs/characters_items/attributes".to_string(),
            ProjectContext::Screen(_)
            | ProjectContext::ScreenWidget(_, _)
            | ProjectContext::RegionCharacterInstance(_, _)
            | ProjectContext::RegionItemInstance(_, _) => "docs/screens/widgets".to_string(),
            _ => "docs/creator/docks/attribute_editor".to_string(),
        }
    }

    fn help_url_for_widget_name(&self, widget_name: &str) -> Option<String> {
        match widget_name {
            "Tiles" | "Tilemap" | "Tile Editor Dock RGBA Layout View" | "Tile Editor Tree" => {
                Some("docs/creator/docks/tile_picker_editor".into())
            }
            "Builder" => Some("docs/creator/tools/builder".into()),
            "Palette" => Some("docs/creator/tools/palette".into()),
            "Iso Paint" => Some("docs/creator/tools/iso-paint".into()),
            "Iso Paint Tool" => Some("docs/creator/tools/iso-paint".into()),
            "Object Tool" => Some("docs/creator/tools/object".into()),
            "Vertex Tool" => Some("docs/creator/tools/vertex".into()),
            "Linedef Tool" | "Linedef / Edge Tool" => Some("docs/creator/tools/linedef".into()),
            "Sector Tool" | "Sector / Face Tool" => Some("docs/creator/tools/sector".into()),
            "Rect Tool" => Some("docs/creator/tools/rect".into()),
            "Entity Tool" => Some("docs/creator/tools/entity".into()),
            "DockDataEditor" | "DockDataEditorMax" | "Data" => {
                Some(self.help_url_for_data_context())
            }
            "DockCodeEditor" | "Code" => Some("docs/creator/docks/eldrin_script_editor".into()),
            "PolyView" => {
                if self.server_ctx.editor_view_mode == EditorViewMode::D2 {
                    Some("docs/building_maps/creating_2d".into())
                } else {
                    Some("docs/building_maps/creating_3d_maps".into())
                }
            }
            name if name.starts_with("Tile Editor ") => {
                Some("docs/creator/docks/tile_picker_editor".into())
            }
            _ => None,
        }
    }

    fn help_url_for_editor_event(&self, event: &TheEvent, ui: &mut TheUI) -> Option<String> {
        let mut clicked = false;
        let widget_name = match event {
            TheEvent::StateChanged(id, state) if *state == TheWidgetState::Clicked => {
                clicked = true;
                Some(id.name.clone())
            }
            TheEvent::RenderViewClicked(id, _) => {
                clicked = true;
                Some(id.name.clone())
            }
            TheEvent::TilePicked(id, _) => {
                clicked = true;
                Some(id.name.clone())
            }
            TheEvent::TileEditorClicked(id, _) => {
                clicked = true;
                Some(id.name.clone())
            }
            TheEvent::MouseDown(coord) => {
                clicked = true;
                ui.get_widget_at_coord(*coord).map(|w| w.id().name.clone())
            }
            _ => None,
        };

        if let Some(widget_name) = widget_name
            && let Some(url) = self.help_url_for_widget_name(&widget_name)
        {
            return Some(url);
        }

        if clicked {
            let dm = DOCKMANAGER.read().unwrap();
            if dm.state != DockManagerState::Minimized {
                return match dm.dock.as_str() {
                    "Tiles" => Some("docs/creator/docks/tile_picker_editor".into()),
                    "Builder" => Some("docs/creator/tools/builder".into()),
                    "Palette" => Some("docs/creator/tools/palette".into()),
                    "Iso Paint" => Some("docs/creator/tools/iso-paint".into()),
                    "Iso Paint Tool" => Some("docs/creator/tools/iso-paint".into()),
                    "Data" => Some(self.help_url_for_data_context()),
                    "Code" => Some("docs/creator/docks/eldrin_script_editor".into()),
                    _ => TOOLLIST
                        .read()
                        .unwrap()
                        .game_tools
                        .get(TOOLLIST.read().unwrap().curr_game_tool)
                        .and_then(|tool| tool.help_url()),
                };
            }
        }
        None
    }
}

impl TheTrait for Editor {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut project = Project::new();
        if let Some(bytes) = crate::Embedded::get("toml/config.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.config = source.to_string();
            }
        }
        if let Some(bytes) = crate::Embedded::get("toml/rules.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.rules = source.to_string();
            }
        }
        if let Some(bytes) = crate::Embedded::get("toml/locales.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.locales = source.to_string();
            }
        }
        if let Some(bytes) = crate::Embedded::get("toml/audio_fx.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.audio_fx = source.to_string();
            }
        }
        if let Some(bytes) = crate::Embedded::get("toml/authoring.toml") {
            if let Ok(source) = std::str::from_utf8(bytes.data.as_ref()) {
                project.authoring = source.to_string();
            }
        }
        let _ = project.sync_ruleset_items();

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        let (self_update_tx, self_update_rx) = channel();

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        let self_updater = SelfUpdater::github_creator();

        let initial_session = ProjectSession {
            project: project.clone(),
            project_path: None,
            undo: UndoManager::default(),
            dirty: false,
        };

        Self {
            project,
            project_path: None,
            sessions: vec![initial_session],
            active_session: 0,
            replace_next_project_load_in_active_tab: false,
            last_active_dirty: false,

            sidebar: Sidebar::new(),
            mapeditor: MapEditor::new(),

            server_ctx: ServerContext::default(),

            update_tracker: UpdateTracker::new(),
            event_receiver: None,
            #[cfg(not(target_arch = "wasm32"))]
            scepter_receiver: None,
            last_3d_hover_redraw_at: None,

            #[cfg(all(
                feature = "self-update",
                any(target_os = "windows", target_os = "linux", target_os = "macos")
            ))]
            self_update_rx,
            #[cfg(all(
                feature = "self-update",
                any(target_os = "windows", target_os = "linux", target_os = "macos")
            ))]
            self_update_tx,
            #[cfg(all(
                feature = "self-update",
                any(target_os = "windows", target_os = "linux", target_os = "macos")
            ))]
            self_updater: Arc::new(Mutex::new(self_updater)),

            update_counter: 0,
            last_processed_log_len: 0,
            pending_game_messages: Vec::new(),
            pending_game_says: Vec::new(),
            pending_game_choices: Vec::new(),
            pending_text_game_command: None,
            pending_text_game_runtime_flush: false,

            build_values: ValueContainer::default(),
            window_state: Self::load_window_state(),
            starter_projects: Vec::new(),
            starter_project_cache: HashMap::new(),
            starter_manifest_cache: None,
            starter_loader_rx: None,
            selected_starter_manifest_id: None,
            iso_paint_render_cache: IsoPaintRenderCache::default(),
        }
    }

    fn init(&mut self, _ctx: &mut TheContext) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (service, receiver) = ScepterService::start();
            self.scepter_receiver = Some(receiver);
            println!("{}", service.status_line());
        }

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        {
            let updater = Arc::clone(&self.self_updater);
            let tx = self.self_update_tx.clone();

            thread::spawn(move || {
                let mut updater = updater.lock().unwrap();

                if let Err(err) = updater.fetch_release_list() {
                    tx.send(SelfUpdateEvent::UpdateError(err.to_string()))
                        .unwrap();
                } else if updater.has_newer_release() {
                    tx.send(SelfUpdateEvent::UpdateAvailable(
                        updater.latest_release().cloned().unwrap(),
                    ))
                    .unwrap();
                };
            });
        }
    }

    fn window_title(&self) -> String {
        format!("Eldiron Creator v{}", env!("CARGO_PKG_VERSION"))
    }

    fn target_fps(&self) -> f64 {
        1000.0 / self.redraw_interval_ms() as f64
    }

    fn fonts_to_load(&self) -> Vec<TheFontScript> {
        vec![TheFontScript::Han]
    }

    fn default_window_size(&self) -> (usize, usize) {
        (
            self.window_state.width.unwrap_or(1200),
            self.window_state.height.unwrap_or(720),
        )
    }

    fn min_window_size(&self) -> (usize, usize) {
        (1200, 720)
    }

    fn default_window_position(&self) -> Option<(i32, i32)> {
        Some((self.window_state.x?, self.window_state.y?))
    }

    fn window_icon(&self) -> Option<(Vec<u8>, u32, u32)> {
        if let Some(file) = Embedded::get("window_logo.png") {
            let data = std::io::Cursor::new(file.data);

            let decoder = png::Decoder::new(data);
            if let Ok(mut reader) = decoder.read_info() {
                if let Some(buffer_size) = reader.output_buffer_size() {
                    let mut buf = vec![0; buffer_size];
                    let info = reader.next_frame(&mut buf).unwrap();
                    let bytes = &buf[..info.buffer_size()];

                    Some((bytes.to_vec(), info.width, info.height))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn init_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        RUSTERIX.write().unwrap().client.messages_font = ctx.ui.font.clone();

        // Embedded Icons
        for file in Embedded::iter() {
            let name = file.as_ref();

            if name.ends_with(".png") {
                if let Some(file) = Embedded::get(name) {
                    let data = std::io::Cursor::new(file.data);

                    let decoder = png::Decoder::new(data);
                    if let Ok(mut reader) = decoder.read_info() {
                        if let Some(buffer_size) = reader.output_buffer_size() {
                            let mut buf = vec![0; buffer_size];
                            let info = reader.next_frame(&mut buf).unwrap();
                            let bytes = &buf[..info.buffer_size()];

                            let mut cut_name = name.replace("icons/", "");
                            cut_name = cut_name.replace(".png", "");

                            ctx.ui.add_icon(
                                cut_name.to_string(),
                                TheRGBABuffer::from(bytes.to_vec(), info.width, info.height),
                            );
                        }
                    }
                }
            }
        }

        // ---

        ui.set_statusbar_name("Statusbar".to_string());

        let mut top_canvas = TheCanvas::new();
        // Internal file/edit/game menu is hidden for the Xcode staticlib wrapper
        // where native menu handling is expected.
        #[cfg(not(feature = "staticlib"))]
        {
            let mut menu_canvas = TheCanvas::new();
            let mut menu = TheMenu::new(TheId::named("Menu"));

            let mut file_menu = TheContextMenu::named(fl!("menu_file"));
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_new"),
                TheId::named("New"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'n'),
            ));
            file_menu.add_separator();
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_open"),
                TheId::named("Open"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'o'),
            ));
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_close"),
                TheId::named("Close"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'w'),
            ));
            file_menu.add_separator();
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_save"),
                TheId::named("Save"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 's'),
            ));
            file_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_save_as"),
                TheId::named("Save As"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'a'),
            ));
            let mut edit_menu = TheContextMenu::named(fl!("menu_edit"));
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_undo"),
                TheId::named("Undo"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'z'),
            ));
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_redo"),
                TheId::named("Redo"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT, 'z'),
            ));
            edit_menu.add_separator();
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_cut"),
                TheId::named("Cut"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'x'),
            ));
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_copy"),
                TheId::named("Copy"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'c'),
            ));
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_paste"),
                TheId::named("Paste"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'v'),
            ));
            edit_menu.add_separator();
            edit_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_apply_action"),
                TheId::named("Action Apply"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'p'),
            ));

            let mut game_menu = TheContextMenu::named(fl!("game"));
            game_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_play"),
                TheId::named("Play"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'p'),
            ));
            game_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_pause"),
                TheId::named("Pause"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'o'),
            ));
            game_menu.add(TheContextMenuItem::new_with_accel(
                fl!("menu_stop"),
                TheId::named("Stop"),
                TheAccelerator::new(TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT, 'p'),
            ));
            game_menu.add_separator();
            let mut show_menu = TheContextMenu::named("Show".to_string());
            show_menu.add(TheContextMenuItem::new(
                "Settings".to_string(),
                TheId::named("Show Settings"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Rules".to_string(),
                TheId::named("Show Rules"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Locales".to_string(),
                TheId::named("Show Locales"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Audio FX".to_string(),
                TheId::named("Show Audio FX"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Authoring".to_string(),
                TheId::named("Show Authoring"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Debug Log".to_string(),
                TheId::named("Show Debug Log"),
            ));
            show_menu.add(TheContextMenuItem::new(
                "Console".to_string(),
                TheId::named("Show Console"),
            ));
            game_menu.add(TheContextMenuItem::new_submenu(
                "Show".to_string(),
                TheId::named("Show"),
                show_menu,
            ));

            file_menu.register_accel(ctx);
            edit_menu.register_accel(ctx);
            game_menu.register_accel(ctx);

            menu.add_context_menu(file_menu);
            menu.add_context_menu(edit_menu);
            menu.add_context_menu(game_menu);
            menu_canvas.set_widget(menu);
            top_canvas.set_top(menu_canvas);
        }

        let mut menubar = TheMenubar::new(TheId::named("Menubar"));
        #[cfg(feature = "staticlib")]
        menubar.limiter_mut().set_max_height(43);
        #[cfg(not(feature = "staticlib"))]
        menubar.limiter_mut().set_max_height(43 + 22);

        let mut logo_button = TheMenubarButton::new(TheId::named("Logo"));
        logo_button.set_icon_name("logo".to_string());
        logo_button.set_status_text(&fl!("status_logo_button"));

        let mut open_button = TheMenubarButton::new(TheId::named("Open"));
        open_button.set_icon_name("icon_role_load".to_string());
        open_button.set_status_text(&fl!("status_open_button"));

        let mut save_button = TheMenubarButton::new(TheId::named("Save"));
        save_button.set_status_text(&fl!("status_save_button"));
        save_button.set_icon_name("icon_role_save".to_string());

        let mut save_as_button = TheMenubarButton::new(TheId::named("Save As"));
        save_as_button.set_icon_name("icon_role_save_as".to_string());
        save_as_button.set_status_text(&fl!("status_save_as_button"));
        save_as_button.set_icon_offset(Vec2::new(2, -5));

        let mut undo_button = TheMenubarButton::new(TheId::named("Undo"));
        undo_button.set_status_text(&fl!("status_undo_button"));
        undo_button.set_icon_name("icon_role_undo".to_string());

        let mut redo_button = TheMenubarButton::new(TheId::named("Redo"));
        redo_button.set_status_text(&fl!("status_redo_button"));
        redo_button.set_icon_name("icon_role_redo".to_string());

        let mut play_button = TheMenubarButton::new(TheId::named("Play"));
        play_button.set_status_text(&fl!("status_play_button"));
        play_button.set_icon_name("play".to_string());
        //play_button.set_fixed_size(vec2i(28, 28));

        let mut pause_button = TheMenubarButton::new(TheId::named("Pause"));
        pause_button.set_status_text(&fl!("status_pause_button"));
        pause_button.set_icon_name("play-pause".to_string());

        let mut stop_button = TheMenubarButton::new(TheId::named("Stop"));
        stop_button.set_status_text(&fl!("status_stop_button"));
        stop_button.set_icon_name("stop-fill".to_string());

        let mut input_button = TheMenubarButton::new(TheId::named("GameInput"));
        input_button.set_status_text(&fl!("status_game_input_button"));
        input_button.set_icon_name("keyboard".to_string());
        input_button.set_has_state(true);

        let mut time_slider = TheTimeSlider::new(TheId::named("Server Time Slider"));
        time_slider.set_status_text(&fl!("status_time_slider"));
        time_slider.set_continuous(true);
        time_slider.limiter_mut().set_max_width(400);
        time_slider.set_value(TheValue::Time(TheTime::default()));

        let mut patreon_button = TheMenubarButton::new(TheId::named("Patreon"));
        patreon_button.set_status_text(&fl!("status_patreon_button"));
        patreon_button.set_icon_name("patreon".to_string());
        // patreon_button.set_fixed_size(vec2i(36, 36));
        patreon_button.set_icon_offset(Vec2::new(-4, -2));

        let mut help_button = TheMenubarButton::new(TheId::named("Help"));
        help_button.set_status_text(&fl!("status_help_button"));
        help_button.set_icon_name("question-mark".to_string());
        help_button.set_has_state(true);
        // patreon_button.set_fixed_size(vec2i(36, 36));
        help_button.set_icon_offset(Vec2::new(-2, -2));

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        let update_button = {
            let mut button = TheTraybarButton::new(TheId::named("Update"));
            button.set_status_text(&fl!("status_update_button"));
            button.set_text(String::new());
            button.set_disabled(true);
            button.limiter_mut().set_max_width(0);
            button
        };

        let mut hlayout = TheHLayout::new(TheId::named("Menu Layout"));
        hlayout.set_background_color(None);
        hlayout.set_margin(Vec4::new(10, 2, 10, 1));
        hlayout.add_widget(Box::new(logo_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(open_button));
        hlayout.add_widget(Box::new(save_button));
        hlayout.add_widget(Box::new(save_as_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(undo_button));
        hlayout.add_widget(Box::new(redo_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(play_button));
        hlayout.add_widget(Box::new(pause_button));
        hlayout.add_widget(Box::new(stop_button));
        hlayout.add_widget(Box::new(input_button));
        hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));
        hlayout.add_widget(Box::new(time_slider));
        //hlayout.add_widget(Box::new(TheMenubarSeparator::new(TheId::empty())));

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        {
            hlayout.add_widget(Box::new(update_button));
            hlayout.add_widget(Box::new(patreon_button));
            hlayout.add_widget(Box::new(help_button));
            hlayout.set_reverse_index(Some(3));
        }

        #[cfg(not(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        )))]
        {
            hlayout.add_widget(Box::new(patreon_button));
            hlayout.add_widget(Box::new(help_button));
            hlayout.set_reverse_index(Some(2));
        }

        top_canvas.set_widget(menubar);
        top_canvas.set_layout(hlayout);
        ui.canvas.set_top(top_canvas);

        // Sidebar
        self.sidebar.init_ui(ui, ctx, &mut self.server_ctx);

        // Docks
        let bottom_panels = DOCKMANAGER.write().unwrap().init(ctx);

        let mut editor_canvas: TheCanvas = TheCanvas::new();

        let mut editor_stack = TheStackLayout::new(TheId::named("Editor Stack"));
        let poly_canvas = self.mapeditor.init_ui(ui, ctx, &mut self.project);
        editor_stack.add_canvas(poly_canvas);

        // Add Dock Editors
        DOCKMANAGER
            .write()
            .unwrap()
            .add_editors_to_stack(&mut editor_stack, ctx);

        editor_canvas.set_layout(editor_stack);

        // Main V Layout
        let mut vsplitlayout = TheSharedVLayout::new(TheId::named("Shared VLayout"));
        vsplitlayout.add_canvas(editor_canvas);
        vsplitlayout.add_canvas(bottom_panels);
        vsplitlayout.set_shared_ratio(crate::DEFAULT_VLAYOUT_RATIO);
        vsplitlayout.set_mode(TheSharedVLayoutMode::Shared);

        let mut shared_canvas = TheCanvas::new();
        shared_canvas.set_layout(vsplitlayout);

        let mut tabs_canvas = TheCanvas::new();
        let mut tabs = TheTabbar::new(TheId::named("Project Tabs"));
        tabs.limiter_mut().set_max_height(22);
        tabs_canvas.set_widget(tabs);
        shared_canvas.set_top(tabs_canvas);

        // Tool List
        let mut tool_list_canvas: TheCanvas = TheCanvas::new();

        let mut tool_list_bar_canvas = TheCanvas::new();
        tool_list_bar_canvas.set_widget(TheToolListBar::new(TheId::empty()));
        tool_list_canvas.set_top(tool_list_bar_canvas);

        let mut v_tool_list_layout = TheVLayout::new(TheId::named("Tool List Layout"));
        v_tool_list_layout.limiter_mut().set_max_width(51);
        v_tool_list_layout.set_margin(Vec4::new(2, 2, 2, 2));
        v_tool_list_layout.set_padding(1);

        TOOLLIST
            .write()
            .unwrap()
            .set_active_editor(&mut v_tool_list_layout, ctx);

        tool_list_canvas.set_layout(v_tool_list_layout);

        let mut tool_list_border_canvas = TheCanvas::new();
        let mut border_widget = TheIconView::new(TheId::empty());
        border_widget.set_border_color(Some([82, 82, 82, 255]));
        border_widget.limiter_mut().set_max_width(1);
        border_widget.limiter_mut().set_max_height(i32::MAX);
        tool_list_border_canvas.set_widget(border_widget);

        tool_list_canvas.set_right(tool_list_border_canvas);
        shared_canvas.set_left(tool_list_canvas);

        ui.canvas.set_center(shared_canvas);

        let mut status_canvas = TheCanvas::new();
        let mut statusbar = TheStatusbar::new(TheId::named("Statusbar"));
        statusbar.set_text(fl!("info_welcome"));
        status_canvas.set_widget(statusbar);

        ui.canvas.set_bottom(status_canvas);

        // -

        // ctx.ui.set_disabled("Save");
        // ctx.ui.set_disabled("Save As");
        ctx.ui.set_disabled("Undo");
        ctx.ui.set_disabled("Redo");

        // Init Rusterix

        if let Some(icon) = ctx.ui.icon("light_on") {
            let texture = Texture::from_rgbabuffer(icon);
            self.build_values.set("light_on", Value::Texture(texture));
        }
        if let Some(icon) = ctx.ui.icon("light_off") {
            let texture = Texture::from_rgbabuffer(icon);
            self.build_values.set("light_off", Value::Texture(texture));
        }
        if let Some(icon) = ctx.ui.icon("character_on") {
            let texture = Texture::from_rgbabuffer(icon);
            self.build_values
                .set("character_on", Value::Texture(texture));
        }
        if let Some(icon) = ctx.ui.icon("character_off") {
            let texture = Texture::from_rgbabuffer(icon);
            self.build_values
                .set("character_off", Value::Texture(texture));
        }
        RUSTERIX
            .write()
            .unwrap()
            .client
            .builder_d2
            .set_properties(&self.build_values);
        RUSTERIX.write().unwrap().set_d2();
        SCENEMANAGER
            .write()
            .unwrap()
            .set_apply_preview_filters(true);
        SCENEMANAGER.write().unwrap().startup();

        self.event_receiver = Some(ui.add_state_listener("Main Receiver".into()));
        self.rebuild_project_tabs(ui);
    }

    /// Set the command line arguments
    fn set_cmd_line_args(&mut self, args: Vec<String>, ctx: &mut TheContext) {
        if args.len() > 1 {
            let mut queued_any = false;
            for arg in args.iter().skip(1) {
                #[allow(irrefutable_let_patterns)]
                if let Ok(path) = PathBuf::from_str(arg) {
                    if !queued_any {
                        self.replace_next_project_load_in_active_tab = true;
                    }
                    ctx.ui.send(TheEvent::FileRequesterResult(
                        TheId::named("Open"),
                        vec![path],
                    ));
                    queued_any = true;
                }
            }
            if queued_any {
                return;
            }
        }

        self.replace_next_project_load_in_active_tab = true;
        ctx.ui.send(TheEvent::StateChanged(
            TheId::named("New"),
            TheWidgetState::Clicked,
        ));
    }

    /// Handle UI events and UI state
    fn update_ui(&mut self, ui: &mut TheUI, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        let mut update_server_icons = false;

        #[cfg(not(target_arch = "wasm32"))]
        let mut scepter_events = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(receiver) = &mut self.scepter_receiver {
            while let Ok(event) = receiver.try_recv() {
                scepter_events.push(event);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        for event in scepter_events {
            match event {
                ScepterEvent::Ping { message, peer } => {
                    let status = if message.trim().is_empty() {
                        format!("Scepter ping received from {peer}.")
                    } else {
                        format!("Scepter ping received from {peer}: {message}")
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    redraw = true;
                }
                ScepterEvent::ServiceError(message) => {
                    let status = format!("Scepter service: {message}");
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    redraw = true;
                }
                ScepterEvent::ProjectSnapshot { reply } => {
                    let _ = reply.send(self.scepter_project_snapshot());
                }
                ScepterEvent::ProjectUndo { reply } => {
                    let had_undo = UNDOMANAGER.read().unwrap().has_undo();
                    if had_undo {
                        UNDOMANAGER.write().unwrap().undo(
                            &mut self.server_ctx,
                            &mut self.project,
                            ui,
                            ctx,
                        );
                    }
                    let result = serde_json::json!({
                        "ok": had_undo,
                        "command": "project.undo",
                        "dirty": self.active_session_has_changes(),
                        "message": if had_undo { "undo applied" } else { "nothing to undo" },
                    });
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Scepter undo.".into(),
                    ));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::ProjectRedo { reply } => {
                    let had_redo = UNDOMANAGER.read().unwrap().has_redo();
                    if had_redo {
                        UNDOMANAGER.write().unwrap().redo(
                            &mut self.server_ctx,
                            &mut self.project,
                            ui,
                            ctx,
                        );
                    }
                    let result = serde_json::json!({
                        "ok": had_redo,
                        "command": "project.redo",
                        "dirty": self.active_session_has_changes(),
                        "message": if had_redo { "redo applied" } else { "nothing to redo" },
                    });
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        "Scepter redo.".into(),
                    ));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::RegionSnapshot { request, reply } => {
                    let _ = reply.send(self.scepter_region_snapshot(&request));
                }
                ScepterEvent::RegionSummary { request, reply } => {
                    let _ = reply.send(self.scepter_region_summary(&request));
                }
                ScepterEvent::RegionRenderPreview { command, reply } => {
                    let _ = reply.send(self.scepter_region_render_preview(&command));
                }
                ScepterEvent::RegionPaintRect { command, reply } => {
                    let result = self.scepter_apply_region_paint_rect(command, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter painted {} region cells.",
                            result
                                .get("cell_count")
                                .and_then(|value| value.as_u64())
                                .unwrap_or_default()
                        )
                    } else {
                        format!(
                            "Scepter paint failed: {}",
                            result
                                .get("error")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown error")
                        )
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::RegionPaintCells { command, reply } => {
                    let result = self.scepter_apply_region_paint_cells(command, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter painted {} region cells.",
                            result
                                .get("cell_count")
                                .and_then(|value| value.as_u64())
                                .unwrap_or_default()
                        )
                    } else {
                        format!(
                            "Scepter paint failed: {}",
                            result
                                .get("error")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown error")
                        )
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::ScriptGet { command, reply } => {
                    let _ = reply.send(self.scepter_get_script(&command));
                }
                ScepterEvent::ScriptValidate { command, reply } => {
                    let _ = reply.send(
                        self.scepter_validate_eldrin_source(&command.target, &command.source),
                    );
                }
                ScepterEvent::ScriptPatch { command, reply } => {
                    let result = self.scepter_apply_script_patch(command, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter edited {} script: {}.",
                            result
                                .get("kind")
                                .and_then(|value| value.as_str())
                                .unwrap_or("target"),
                            result
                                .get("name")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown")
                        )
                    } else {
                        format!(
                            "Scepter script edit failed: {}",
                            result
                                .get("error")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown error")
                        )
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::AttributesGet { command, reply } => {
                    let _ = reply.send(self.scepter_get_attributes(&command));
                }
                ScepterEvent::AttributesPatch { command, reply } => {
                    let result = self.scepter_apply_attributes_patch(command, ctx);
                    let ok = result
                        .get("ok")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    let status = if ok {
                        format!(
                            "Scepter edited {} attributes: {}.",
                            result
                                .get("kind")
                                .and_then(|value| value.as_str())
                                .unwrap_or("target"),
                            result
                                .get("name")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown")
                        )
                    } else {
                        format!(
                            "Scepter attribute edit failed: {}",
                            result
                                .get("error")
                                .and_then(|value| value.as_str())
                                .unwrap_or("unknown error")
                        )
                    };
                    println!("{status}");
                    ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), status));
                    let _ = reply.send(result);
                    redraw = true;
                }
                ScepterEvent::TilesSnapshot { reply } => {
                    let _ = reply.send(self.scepter_tiles_snapshot());
                }
            }
        }

        if let Some((input_id, command)) = self.pending_text_game_command.take() {
            TEXTGAME.write().unwrap().handle_input(
                &input_id,
                &command,
                &mut self.project,
                &self.server_ctx,
                ui,
                ctx,
            );
            self.pending_text_game_runtime_flush = !command.trim().is_empty();
            redraw = true;
        }

        if self.pending_text_game_runtime_flush {
            let is_running =
                RUSTERIX.read().unwrap().server.state == rusterix::ServerState::Running;
            if is_running && self.server_ctx.text_game_mode {
                warmup_runtime(&mut RUSTERIX.write().unwrap(), &mut self.project, 1);

                if let Some(region) = self.project.get_region_ctx(&self.server_ctx) {
                    let region_id = region.map.id;
                    let mut messages = RUSTERIX.write().unwrap().server.get_messages(&region_id);
                    let mut says = RUSTERIX.write().unwrap().server.get_says(&region_id);

                    TEXTGAME.write().unwrap().update(
                        &self.project,
                        &self.server_ctx,
                        &mut messages,
                        &mut says,
                        ui,
                        ctx,
                    );
                }
            }
            self.pending_text_game_runtime_flush = false;
            redraw = true;
        }

        // Make sure on first startup the active tool is properly selected
        if self.update_counter == 0 {
            let mut toollist = TOOLLIST.write().unwrap();
            let id = toollist.get_current_tool().id().uuid;

            toollist.set_tool(id, ui, ctx, &mut self.project, &mut self.server_ctx);
        }

        // Get build results from the scene manager if any
        while let Some(result) = SCENEMANAGER.write().unwrap().receive() {
            match result {
                SceneManagerResult::Startup => {
                    println!("Scene manager has started up.");
                }
                SceneManagerResult::Chunk(chunk, togo, total, billboards) => {
                    if togo == 0 {
                        self.server_ctx.background_progress = None;
                    } else {
                        self.server_ctx.background_progress = Some(format!("{togo}/{total}"));
                    }

                    let mut rusterix = RUSTERIX.write().unwrap();

                    rusterix
                        .scene_handler
                        .build_index
                        .remove_chunk_origin((chunk.origin.x, chunk.origin.y));
                    rusterix
                        .scene_handler
                        .vm
                        .execute(scenevm::Atom::RemoveChunkAt {
                            origin: chunk.origin,
                        });

                    rusterix.scene_handler.build_index.index_chunk(&chunk);
                    rusterix.scene_handler.vm.execute(scenevm::Atom::AddChunk {
                        id: Uuid::new_v4(),
                        chunk: chunk,
                    });

                    // Add billboards to scene_handler (indexed by GeoId)
                    for billboard in billboards {
                        rusterix
                            .scene_handler
                            .billboards
                            .insert(billboard.geo_id, billboard);
                    }

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Update Minimap"),
                        TheValue::Empty,
                    ));
                }
                SceneManagerResult::Clear => {
                    let mut rusterix = RUSTERIX.write().unwrap();
                    rusterix
                        .scene_handler
                        .vm
                        .execute(scenevm::Atom::ClearGeometry);

                    rusterix.scene_handler.billboards.clear();
                    rusterix.scene_handler.build_index.clear();
                }
                SceneManagerResult::Quit => {
                    println!("Scene manager has shutdown.");
                }
            }
        }

        // Check for redraw (30fps) and tick updates
        let redraw_ms = self.redraw_interval_ms();
        let tick_ms = CONFIGEDITOR.read().unwrap().game_tick_ms.max(1) as u64;
        let (mut redraw_update, tick_update) = self.update_tracker.update(redraw_ms, tick_ms);

        // Handle queued UI events in the same update pass so input can trigger immediate redraw work.
        let mut pending_events = Vec::new();
        if let Some(receiver) = &mut self.event_receiver {
            while let Ok(event) = receiver.try_recv() {
                pending_events.push(event);
            }
        }
        if !pending_events.is_empty() {
            let only_3d_polyview_hover = self.server_ctx.editor_view_mode != EditorViewMode::D2
                && pending_events.iter().all(|event| {
                    matches!(
                        event,
                        TheEvent::RenderViewHoverChanged(id, _)
                            | TheEvent::RenderViewLostHover(id) if id.name == "PolyView"
                    )
                });
            let only_3d_geometry_drag = self.server_ctx.editor_view_mode != EditorViewMode::D2
                && matches!(
                    self.server_ctx.curr_map_tool_type,
                    MapToolType::Vertex | MapToolType::Linedef | MapToolType::Sector
                )
                && self.server_ctx.geometry_edit_mode != GeometryEditMode::Detail
                && pending_events.iter().all(|event| {
                    matches!(
                        event,
                        TheEvent::RenderViewDragged(id, _) if id.name == "PolyView"
                    )
                });
            if only_3d_polyview_hover || only_3d_geometry_drag {
                if let Some(last_event) = pending_events.pop() {
                    pending_events.clear();
                    pending_events.push(last_event);
                }
            }

            if only_3d_polyview_hover {
                self.last_3d_hover_redraw_at = Some(std::time::Instant::now());
                redraw_update = true;
            } else {
                redraw_update = true;
            }
        }

        if let Some(receiver) = &mut self.starter_loader_rx
            && let Ok(starters) = receiver.try_recv()
        {
            self.starter_manifest_cache = Some(starters.clone());
            self.starter_projects = starters;
            self.rebuild_starter_project_list(ui, ctx);
            if let Some(first) = self.starter_projects.first() {
                self.selected_starter_manifest_id = Some(first.manifest_id.clone());
                ctx.ui.send(TheEvent::StateChanged(
                    TheId::named_with_id("Starter Project List Item", first.id),
                    TheWidgetState::Selected,
                ));
                ui.set_enabled(Self::STARTER_CREATE_ID, ctx);
            } else if let Some(list) = ui.get_list_layout(Self::STARTER_LIST_ID) {
                list.clear();
                let mut item = TheListItem::new(TheId::named("Starter Project Empty"));
                item.set_text(fl!("starter_empty"));
                item.set_sub_text(fl!("starter_empty_sub"));
                item.set_size(52);
                item.set_text_color(WHITE);
                item.set_text_size(14.0);
                item.set_sub_text_size(12.0);
                list.add_item(item, ctx);
            }
            self.starter_loader_rx = None;
            ctx.ui.relayout = true;
            ctx.ui.redraw_all = true;
            redraw_update = true;
        }

        if tick_update {
            RUSTERIX.write().unwrap().client.inc_animation_frame();
            RUSTERIX
                .write()
                .unwrap()
                .scene_handler
                .tick_particle_clocks();

            self.server_ctx.animation_counter = self.server_ctx.animation_counter.wrapping_add(1);
            // To update animated minimaps (only for docks that need it)
            if DOCKMANAGER
                .read()
                .unwrap()
                .current_dock_supports_minimap_animation()
            {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Soft Update Minimap"),
                    TheValue::Empty,
                ));
            }
        }

        if redraw_update && !self.project.regions.is_empty() {
            // SCENEMANAGER.write().unwrap().tick();
            SCENEMANAGER.write().unwrap().tick_batch(8);

            self.build_values.set(
                "no_rect_geo",
                Value::Bool(self.server_ctx.no_rect_geo_on_map),
            );

            extract_build_values_from_config(&mut self.build_values);

            let mut messages = Vec::new();
            let mut says = Vec::new();
            let mut choices = Vec::new();

            // Update entities when the server is running
            {
                let rusterix = &mut RUSTERIX.write().unwrap();
                if rusterix.server.state == rusterix::ServerState::Running {
                    // Send a game tick to all servers
                    if tick_update {
                        rusterix.server.system_tick();
                    }

                    // Send a redraw tick to all servers
                    if redraw_update {
                        rusterix.server.redraw_tick();
                    }

                    if let Some(new_region_name) = rusterix.update_server() {
                        rusterix.client.current_map = new_region_name;
                    }
                    if rusterix.server.log_changed {
                        let log_text = rusterix.server.get_log();
                        ui.set_widget_value("LogEdit", ctx, TheValue::Text(log_text.clone()));

                        // Auto-open Debug Log only when new log content contains warning/error.
                        let mut start = if log_text.len() < self.last_processed_log_len {
                            0
                        } else {
                            self.last_processed_log_len
                        };
                        while start < log_text.len() && !log_text.is_char_boundary(start) {
                            start += 1;
                        }
                        let new_segment = &log_text[start..];
                        if Self::log_segment_has_warning_or_error(new_segment) {
                            ctx.ui.send(TheEvent::StateChanged(
                                TheId::named("Debug Log"),
                                TheWidgetState::Clicked,
                            ));
                        }
                        self.last_processed_log_len = log_text.len();
                    }
                    let active_game_map = rusterix.client.current_map.clone();
                    let mut refresh_visual_debug = false;
                    for r in &mut self.project.regions {
                        rusterix.server.apply_entities_items(&mut r.map);

                        let is_active_region = if self.server_ctx.game_mode {
                            r.map.name == active_game_map
                        } else {
                            r.id == self.server_ctx.curr_region
                        };

                        if is_active_region {
                            refresh_visual_debug = true;
                            if let Some(time) = rusterix.server.get_time(&r.map.id) {
                                rusterix.client.set_server_time(time);
                                if let Some(widget) = ui.get_widget("Server Time Slider") {
                                    widget.set_value(TheValue::Time(rusterix.client.server_time));
                                }
                            }
                            messages = rusterix.server.get_messages(&r.map.id);
                            says = rusterix.server.get_says(&r.map.id);
                            choices = rusterix.server.get_choices(&r.map.id);

                            if !self.server_ctx.game_mode {
                                self.pending_game_messages.append(&mut messages);
                                self.pending_game_says.append(&mut says);
                                self.pending_game_choices.append(&mut choices);
                            } else {
                                if !self.pending_game_messages.is_empty() {
                                    let mut pending =
                                        std::mem::take(&mut self.pending_game_messages);
                                    pending.append(&mut messages);
                                    messages = pending;
                                }
                                if !self.pending_game_says.is_empty() {
                                    let mut pending = std::mem::take(&mut self.pending_game_says);
                                    pending.append(&mut says);
                                    says = pending;
                                }
                                if !self.pending_game_choices.is_empty() {
                                    let mut pending =
                                        std::mem::take(&mut self.pending_game_choices);
                                    pending.append(&mut choices);
                                    choices = pending;
                                }
                            }
                            for cmd in rusterix.server.get_audio_commands(&r.map.id) {
                                match cmd {
                                    AudioCommand::Play {
                                        name,
                                        bus,
                                        gain,
                                        looping,
                                    } => {
                                        rusterix.play_audio_on_bus(&name, &bus, gain, looping);
                                    }
                                    AudioCommand::ClearBus { bus } => {
                                        rusterix.clear_audio_bus(&bus);
                                    }
                                    AudioCommand::ClearAll => {
                                        rusterix.clear_all_audio();
                                    }
                                    AudioCommand::SetBusVolume { bus, volume } => {
                                        rusterix.set_audio_bus_volume(&bus, volume);
                                    }
                                }
                            }
                        }
                    }
                    if refresh_visual_debug {
                        DOCKMANAGER.write().unwrap().apply_eldrin_debug_data(
                            ui,
                            ctx,
                            &self.project,
                            &self.server_ctx,
                            &rusterix.server.eldrin_debug,
                        );
                    }
                }
            }
            let is_running =
                RUSTERIX.read().unwrap().server.state == rusterix::ServerState::Running;

            DOCKMANAGER.write().unwrap().sync_text_play_dock(
                ui,
                ctx,
                &self.project,
                &mut self.server_ctx,
                is_running,
            );

            if is_running && self.server_ctx.text_game_mode {
                if !self.server_ctx.game_mode {
                    if !self.pending_game_messages.is_empty() {
                        messages = std::mem::take(&mut self.pending_game_messages);
                    }
                    if !self.pending_game_says.is_empty() {
                        says = std::mem::take(&mut self.pending_game_says);
                    }
                }
                TEXTGAME.write().unwrap().update(
                    &self.project,
                    &self.server_ctx,
                    &mut messages,
                    &mut says,
                    ui,
                    ctx,
                );
            }

            // Draw Map
            if let Some(render_view) = ui.get_render_view("PolyView") {
                let dim = *render_view.dim();

                let buffer = render_view.render_buffer_mut();
                buffer.resize(dim.width, dim.height);

                {
                    // If we are drawing billboard vertices in the geometry overlay, update them.
                    if !self.server_ctx.game_mode
                        && self.server_ctx.editor_view_mode != EditorViewMode::D2
                        && self.server_ctx.curr_map_tool_type == MapToolType::Vertex
                    {
                        TOOLLIST
                            .write()
                            .unwrap()
                            .update_geometry_overlay_3d(&mut self.project, &mut self.server_ctx);
                    }

                    let rusterix = &mut RUSTERIX.write().unwrap();

                    if is_running && self.server_ctx.game_mode {
                        let game_messages = if self.server_ctx.text_game_mode {
                            Vec::new()
                        } else {
                            messages
                        };
                        let game_says = if self.server_ctx.text_game_mode {
                            Vec::new()
                        } else {
                            says
                        };
                        let game_choices = if self.server_ctx.text_game_mode {
                            Vec::new()
                        } else {
                            choices
                        };
                        for r in &mut self.project.regions {
                            if r.map.name == rusterix.client.current_map {
                                let region_id = r.id;
                                let iso_paint = r.iso_paint.clone();
                                let has_iso_paint =
                                    iso_paint.visible && !iso_paint.chunks.is_empty();
                                if !has_iso_paint {
                                    let active_vm = rusterix.scene_handler.vm.active_vm_index();
                                    rusterix.scene_handler.vm.set_active_vm(0);
                                    rusterix
                                        .scene_handler
                                        .vm
                                        .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                                    rusterix.scene_handler.vm.set_active_vm(active_vm);
                                    self.iso_paint_render_cache.uploaded_key = None;
                                }
                                rusterix.draw_game_with_widget_overlay(
                                    &r.map,
                                    game_messages,
                                    game_says,
                                    game_choices,
                                    |widget, scene_handler| {
                                        if !has_iso_paint {
                                            return false;
                                        }
                                        let dim = *widget.buffer.dim();
                                        if dim.width <= 0 || dim.height <= 0 {
                                            scene_handler
                                                .vm
                                                .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                                            return true;
                                        }
                                        let active_vm = scene_handler.vm.active_vm_index();
                                        scene_handler.vm.set_active_vm(0);
                                        let paint_surface = scene_handler.vm.paint_surface_buffer(
                                            dim.width as u32,
                                            dim.height as u32,
                                        );
                                        let paint_surface_key = scene_handler
                                            .vm
                                            .paint_surface_key(dim.width as u32, dim.height as u32);
                                        let view = widget.camera_d3.view_matrix();
                                        let proj = widget
                                            .camera_d3
                                            .projection_matrix(dim.width as f32, dim.height as f32);
                                        let camera_scale = Some(widget.camera_d3.scale());
                                        let overlay = Self::build_iso_paint_overlay_prepared(
                                            &mut self.iso_paint_render_cache,
                                            region_id,
                                            &iso_paint,
                                            Some(&paint_surface),
                                            paint_surface_key,
                                            camera_scale,
                                            TheDim::sized(dim.width, dim.height),
                                            |point, width, height| {
                                                if width <= 0 || height <= 0 {
                                                    return None;
                                                }
                                                let clip = (proj * view)
                                                    * Vec4::new(point[0], point[1], point[2], 1.0);
                                                if clip.w.abs() <= f32::EPSILON {
                                                    return None;
                                                }
                                                let ndc = Vec3::new(
                                                    clip.x / clip.w,
                                                    clip.y / clip.w,
                                                    clip.z / clip.w,
                                                );
                                                Some([
                                                    ((ndc.x * 0.5 + 0.5) * width as f32).round()
                                                        as i32,
                                                    ((1.0 - (ndc.y * 0.5 + 0.5)) * height as f32)
                                                        .round()
                                                        as i32,
                                                ])
                                            },
                                        );
                                        let should_redraw =
                                            if let Some((key, overlay, changed)) = overlay {
                                                let needs_upload = changed
                                                    || self.iso_paint_render_cache.uploaded_key
                                                        != Some(key);
                                                if needs_upload {
                                                    scene_handler.vm.execute(
                                                        scenevm::Atom::SetRaster3DPaintOverlay {
                                                            width: overlay.width,
                                                            height: overlay.height,
                                                            color_rgba: overlay.color_rgba,
                                                            material_rgba: overlay.material_rgba,
                                                        },
                                                    );
                                                    self.iso_paint_render_cache.uploaded_key =
                                                        Some(key);
                                                }
                                                needs_upload
                                            } else {
                                                scene_handler.vm.execute(
                                                    scenevm::Atom::ClearRaster3DPaintOverlay,
                                                );
                                                self.iso_paint_render_cache.uploaded_key = None;
                                                true
                                            };
                                        scene_handler.vm.set_active_vm(active_vm);
                                        should_redraw
                                    },
                                );
                                break;
                            }
                        }

                        rusterix
                            .client
                            .insert_game_buffer(render_view.render_buffer_mut());
                    } else {
                        if self.server_ctx.editor_view_mode != EditorViewMode::D2
                            && self.server_ctx.get_map_context() == MapContext::Region
                        {
                            if let Some(region) =
                                self.project.get_region_ctx_mut(&mut self.server_ctx)
                            {
                                let follow_player_firstp = is_running
                                    && self.server_ctx.editor_view_mode == EditorViewMode::FirstP;

                                if follow_player_firstp
                                    && let Some(player) =
                                        region.map.entities.iter().find(|e| e.is_player())
                                {
                                    let orientation =
                                        if player.orientation.magnitude_squared() > f32::EPSILON {
                                            player.orientation.normalized()
                                        } else {
                                            Vec2::new(1.0, 0.0)
                                        };

                                    region.editing_position_3d = Vec3::new(
                                        player.position.x,
                                        player.position.y,
                                        player.position.z,
                                    );
                                    region.editing_look_at_3d = Vec3::new(
                                        player.position.x + orientation.x,
                                        player.position.y,
                                        player.position.z + orientation.y,
                                    );
                                } else {
                                    EDITCAMERA.write().unwrap().update_action(
                                        region,
                                        &mut self.server_ctx,
                                        ctx.get_time(),
                                    );
                                }
                                EDITCAMERA.write().unwrap().update_camera(
                                    region,
                                    &mut self.server_ctx,
                                    rusterix,
                                );
                                if self.server_ctx.editor_view_mode == EditorViewMode::FirstP
                                    && (self.server_ctx.editor_fly_nav_active
                                        || EDITCAMERA.read().unwrap().move_action.is_some())
                                {
                                    ctx.ui.redraw_all = true;
                                }

                                // Keep editor 3D running mode in sync with runtime dynamic
                                // overlays (characters/items/lights).
                                let animation_frame = rusterix.client.animation_frame;
                                rusterix.scene_handler.mark_dynamics_dirty();
                                rusterix.build_dynamics_3d(&region.map, animation_frame);
                                let editor_neutral_background =
                                    !is_running && !self.server_ctx.game_mode;
                                let iso_paint = region.iso_paint.clone();
                                let has_iso_paint =
                                    iso_paint.visible && !iso_paint.chunks.is_empty();
                                if self.server_ctx.editor_view_mode == EditorViewMode::Iso
                                    && has_iso_paint
                                {
                                    let view = rusterix.client.camera_d3.view_matrix();
                                    let proj = rusterix
                                        .client
                                        .camera_d3
                                        .projection_matrix(dim.width as f32, dim.height as f32);
                                    let camera_scale = Some(rusterix.client.camera_d3.scale());
                                    let active_vm = rusterix.scene_handler.vm.active_vm_index();
                                    rusterix.scene_handler.vm.set_active_vm(0);
                                    let surface_key_before = rusterix
                                        .scene_handler
                                        .vm
                                        .paint_surface_key(dim.width as u32, dim.height as u32);
                                    let expected_key = Self::iso_paint_overlay_key(
                                        region.id,
                                        &iso_paint,
                                        TheDim::sized(dim.width, dim.height),
                                        surface_key_before,
                                        camera_scale,
                                    );
                                    let overlay_ready = self.iso_paint_render_cache.prepared_key
                                        == Some(expected_key)
                                        && self.iso_paint_render_cache.uploaded_key
                                            == Some(expected_key);
                                    rusterix.scene_handler.vm.set_active_vm(active_vm);

                                    if !overlay_ready {
                                        rusterix.draw_d3_with_editor_background(
                                            &region.map,
                                            render_view.render_buffer_mut().pixels_mut(),
                                            dim.width as usize,
                                            dim.height as usize,
                                            editor_neutral_background,
                                        );
                                    }

                                    if !overlay_ready {
                                        let active_vm = rusterix.scene_handler.vm.active_vm_index();
                                        rusterix.scene_handler.vm.set_active_vm(0);
                                        let paint_surface_key = rusterix
                                            .scene_handler
                                            .vm
                                            .paint_surface_key(dim.width as u32, dim.height as u32);
                                        let paint_surface =
                                            rusterix.scene_handler.vm.paint_surface_buffer(
                                                dim.width as u32,
                                                dim.height as u32,
                                            );
                                        let overlay = Self::build_iso_paint_overlay_prepared(
                                            &mut self.iso_paint_render_cache,
                                            region.id,
                                            &iso_paint,
                                            Some(&paint_surface),
                                            paint_surface_key,
                                            camera_scale,
                                            TheDim::sized(dim.width, dim.height),
                                            |point, width, height| {
                                                if width <= 0 || height <= 0 {
                                                    return None;
                                                }
                                                let clip = (proj * view)
                                                    * Vec4::new(point[0], point[1], point[2], 1.0);
                                                if clip.w.abs() <= f32::EPSILON {
                                                    return None;
                                                }
                                                let ndc = Vec3::new(
                                                    clip.x / clip.w,
                                                    clip.y / clip.w,
                                                    clip.z / clip.w,
                                                );
                                                Some([
                                                    ((ndc.x * 0.5 + 0.5) * width as f32).round()
                                                        as i32,
                                                    ((1.0 - (ndc.y * 0.5 + 0.5)) * height as f32)
                                                        .round()
                                                        as i32,
                                                ])
                                            },
                                        );
                                        if let Some((key, overlay, changed)) = overlay {
                                            let needs_upload = changed
                                                || self.iso_paint_render_cache.uploaded_key
                                                    != Some(key);
                                            if needs_upload {
                                                rusterix.scene_handler.vm.execute(
                                                    scenevm::Atom::SetRaster3DPaintOverlay {
                                                        width: overlay.width,
                                                        height: overlay.height,
                                                        color_rgba: overlay.color_rgba,
                                                        material_rgba: overlay.material_rgba,
                                                    },
                                                );
                                                self.iso_paint_render_cache.uploaded_key =
                                                    Some(key);
                                            }
                                        } else {
                                            rusterix
                                                .scene_handler
                                                .vm
                                                .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                                            self.iso_paint_render_cache.uploaded_key = None;
                                        }
                                        rusterix.scene_handler.vm.set_active_vm(active_vm);
                                    }
                                } else {
                                    let active_vm = rusterix.scene_handler.vm.active_vm_index();
                                    rusterix.scene_handler.vm.set_active_vm(0);
                                    rusterix
                                        .scene_handler
                                        .vm
                                        .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
                                    rusterix.scene_handler.vm.set_active_vm(active_vm);
                                    self.iso_paint_render_cache.uploaded_key = None;
                                }
                                rusterix.draw_d3_with_editor_background(
                                    &region.map,
                                    render_view.render_buffer_mut().pixels_mut(),
                                    dim.width as usize,
                                    dim.height as usize,
                                    editor_neutral_background,
                                );
                            }
                        } else
                        // Draw the region map
                        if self.server_ctx.get_map_context() == MapContext::Region
                            && self.server_ctx.editing_surface.is_none()
                        {
                            if let Some(region) =
                                self.project.get_region(&self.server_ctx.curr_region)
                            {
                                rusterix.client.set_clip_rect_d2(None);
                                rusterix
                                    .client
                                    .set_map_tool_type_d2(self.server_ctx.curr_map_tool_type);
                                if let Some(hover_cursor) = self.server_ctx.hover_cursor {
                                    rusterix.client.set_map_hover_info_d2(
                                        self.server_ctx.hover,
                                        Some(vek::Vec2::new(hover_cursor.x, hover_cursor.y)),
                                    );
                                } else {
                                    rusterix
                                        .client
                                        .set_map_hover_info_d2(self.server_ctx.hover, None);
                                }

                                if let Some(camera_pos) = region.map.camera_xz {
                                    rusterix.client.set_camera_info_d2(
                                        Some(Vec3::new(camera_pos.x, 0.0, camera_pos.y)),
                                        None,
                                    );
                                }

                                // let start_time = ctx.get_time();

                                if let Some(clipboard) = &self.server_ctx.paste_clipboard {
                                    // During a paste operation we use a merged map

                                    let mut map = region.map.clone();
                                    if let Some(hover) = self.server_ctx.hover_cursor {
                                        map.paste_at_position(clipboard, hover);
                                    }

                                    rusterix.set_dirty();
                                    rusterix.apply_entities_items(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        &map,
                                        &self.server_ctx.editing_surface,
                                        false,
                                    );
                                } else if let Some(map) = self.project.get_map(&self.server_ctx) {
                                    rusterix.apply_entities_items(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        map,
                                        &self.server_ctx.editing_surface,
                                        false,
                                    );
                                }

                                // Prepare the messages for the region for drawing
                                rusterix.process_messages(&region.map, says);

                                // let stop_time = ctx.get_time();
                                //println!("{} ms", stop_time - start_time);
                            }

                            if let Some(map) = self.project.get_map_mut(&self.server_ctx) {
                                if self.server_ctx.editor_view_mode == EditorViewMode::D2 {
                                    rusterix.scene_handler.settings.backend_2d =
                                        RendererBackend::Raster;
                                    rusterix.set_d2();
                                }
                                if is_running
                                    && self.server_ctx.editor_view_mode == EditorViewMode::D2
                                {
                                    let animation_frame = rusterix.client.animation_frame;
                                    rusterix.build_dynamics_2d(map, animation_frame);
                                }
                                if self.server_ctx.editor_view_mode == EditorViewMode::D2
                                    && rusterix.scene_handler.vm.vm_layer_count() > 1
                                {
                                    rusterix.scene_handler.vm.set_layer_enabled(
                                        1,
                                        self.server_ctx.show_editing_geometry,
                                    );
                                }
                                rusterix.draw_scene(
                                    map,
                                    render_view.render_buffer_mut().pixels_mut(),
                                    dim.width as usize,
                                    dim.height as usize,
                                );
                            }
                        } else if self.server_ctx.get_map_context() == MapContext::Region
                            && self.server_ctx.editing_surface.is_some()
                        {
                            rusterix
                                .client
                                .set_map_tool_type_d2(self.server_ctx.curr_map_tool_type);
                            if let Some(profile) = self.project.get_map_mut(&self.server_ctx) {
                                if rusterix.scene_handler.vm.vm_layer_count() > 1 {
                                    // Profile editor relies on 2D overlay guides.
                                    rusterix.scene_handler.vm.set_layer_enabled(1, true);
                                }
                                if let Some(hover_cursor) = self.server_ctx.hover_cursor {
                                    rusterix.client.set_map_hover_info_d2(
                                        self.server_ctx.hover,
                                        Some(vek::Vec2::new(hover_cursor.x, hover_cursor.y)),
                                    );
                                } else {
                                    rusterix
                                        .client
                                        .set_map_hover_info_d2(self.server_ctx.hover, None);
                                }

                                if let Some(clipboard) = &self.server_ctx.paste_clipboard {
                                    // During a paste operation we use a merged map
                                    let mut map = profile.clone();
                                    if let Some(hover) = self.server_ctx.hover_cursor {
                                        map.paste_at_position(clipboard, hover);
                                    }
                                    rusterix.set_dirty();
                                    rusterix.build_custom_scene_d2(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        &map,
                                        &self.build_values,
                                        &self.server_ctx.editing_surface,
                                        true,
                                    );
                                    rusterix.draw_custom_d2(
                                        &map,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                } else {
                                    rusterix.build_custom_scene_d2(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        profile,
                                        &self.build_values,
                                        &self.server_ctx.editing_surface,
                                        true,
                                    );
                                    rusterix.draw_custom_d2(
                                        profile,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                }
                            }
                        } else
                        // Draw the screen / character / item map
                        if self.server_ctx.get_map_context() == MapContext::Character
                            || self.server_ctx.get_map_context() == MapContext::Item
                            || self.server_ctx.get_map_context() == MapContext::Screen
                        {
                            rusterix
                                .client
                                .set_map_tool_type_d2(self.server_ctx.curr_map_tool_type);
                            if let Some(map) = self.project.get_map_mut(&self.server_ctx) {
                                if rusterix.scene_handler.vm.vm_layer_count() > 1 {
                                    // Screen/character/item overlays should respect toggle.
                                    rusterix.scene_handler.vm.set_layer_enabled(
                                        1,
                                        self.server_ctx.show_editing_geometry,
                                    );
                                }
                                if let Some(hover_cursor) = self.server_ctx.hover_cursor {
                                    rusterix.client.set_map_hover_info_d2(
                                        self.server_ctx.hover,
                                        Some(vek::Vec2::new(hover_cursor.x, hover_cursor.y)),
                                    );
                                } else {
                                    rusterix
                                        .client
                                        .set_map_hover_info_d2(self.server_ctx.hover, None);
                                }

                                if self.server_ctx.get_map_context() != MapContext::Screen {
                                    rusterix.client.builder_d2.set_clip_rect(Some(
                                        rusterix::Rect {
                                            x: -5.0,
                                            y: -5.0,
                                            width: 10.0,
                                            height: 10.0,
                                        },
                                    ));
                                } else {
                                    let viewport = CONFIGEDITOR.read().unwrap().viewport;
                                    let grid_size = CONFIGEDITOR.read().unwrap().grid_size as f32;
                                    let w = viewport.x as f32 / grid_size;
                                    let h = viewport.y as f32 / grid_size;
                                    rusterix.client.builder_d2.set_clip_rect(Some(
                                        rusterix::Rect {
                                            x: -w / 2.0,
                                            y: -h / 2.0,
                                            width: w,
                                            height: h,
                                        },
                                    ));
                                }

                                if let Some(clipboard) = &self.server_ctx.paste_clipboard {
                                    // During a paste operation we use a merged map
                                    let mut map = map.clone();
                                    if let Some(hover) = self.server_ctx.hover_cursor {
                                        map.paste_at_position(clipboard, hover);
                                    }
                                    rusterix.set_dirty();
                                    rusterix.build_custom_scene_d2(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        &map,
                                        &self.build_values,
                                        &self.server_ctx.editing_surface,
                                        true,
                                    );
                                    rusterix.draw_custom_d2(
                                        &map,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                } else {
                                    rusterix.build_custom_scene_d2(
                                        Vec2::new(dim.width as f32, dim.height as f32),
                                        map,
                                        &self.build_values,
                                        &None,
                                        true,
                                    );
                                    rusterix.draw_custom_d2(
                                        map,
                                        render_view.render_buffer_mut().pixels_mut(),
                                        dim.width as usize,
                                        dim.height as usize,
                                    );
                                }
                            }
                        }
                    }
                }
                if !self.server_ctx.game_mode
                    && self.server_ctx.get_map_context() == MapContext::Region
                    && self.server_ctx.editor_view_mode == EditorViewMode::Iso
                {
                    let iso_paint = self
                        .project
                        .get_region(&self.server_ctx.curr_region)
                        .map(|region| region.iso_paint.clone());
                    if let Some(iso_paint) = iso_paint {
                        let buffer = render_view.render_buffer_mut();
                        if self.server_ctx.curr_map_tool_type == MapToolType::IsoPaint {
                            Self::draw_iso_paint_preview(
                                buffer,
                                &iso_paint,
                                self.server_ctx.iso_paint_hover_screen,
                            );
                        }
                    }
                }
                if !self.server_ctx.game_mode {
                    let map_for_hud = if self.server_ctx.get_map_context() == MapContext::Region
                        && self.server_ctx.editor_view_mode != EditorViewMode::D2
                        && self.server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                    {
                        self.project
                            .get_region_mut(&self.server_ctx.curr_region)
                            .map(|region| &mut region.map)
                    } else {
                        self.project.get_map_mut(&self.server_ctx)
                    };
                    if let Some(map) = map_for_hud {
                        TOOLLIST.write().unwrap().draw_hud(
                            render_view.render_buffer_mut(),
                            map,
                            ctx,
                            &mut self.server_ctx,
                            &RUSTERIX.read().unwrap().assets,
                        );
                    }
                }
            }

            // Draw the 3D Preview if active.
            // if !self.server_ctx.game_mode
            //     && self.server_ctx.curr_map_tool_helper == MapToolHelper::Preview
            // {
            //     if let Some(region) = self.project.get_region_ctx(&self.server_ctx) {
            //         PREVIEWVIEW
            //             .write()
            //             .unwrap()
            //             .draw(region, ui, ctx, &mut self.server_ctx);
            //     }
            // }

            redraw = true;
        }

        for event in pending_events {
            if self.server_ctx.help_mode
                && let Some(url) = self.help_url_for_editor_event(&event, ui)
            {
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Show Help"),
                    TheValue::Text(url),
                ));
                redraw = true;
                continue;
            }

            if self.server_ctx.game_input_mode && !self.server_ctx.game_mode {
                // In game input mode send events to the game tool
                if let Some(game_tool) =
                    TOOLLIST.write().unwrap().get_game_tool_of_name("Game Tool")
                {
                    redraw = game_tool.handle_event(
                        &event,
                        ui,
                        ctx,
                        &mut self.project,
                        &mut self.server_ctx,
                    );
                }
            }
            if self
                .sidebar
                .handle_event(&event, ui, ctx, &mut self.project, &mut self.server_ctx)
            {
                redraw = true;
            }
            if TOOLLIST.write().unwrap().handle_event(
                &event,
                ui,
                ctx,
                &mut self.project,
                &mut self.server_ctx,
            ) {
                redraw = true;
            }
            if DOCKMANAGER.write().unwrap().handle_event(
                &event,
                ui,
                ctx,
                &mut self.project,
                &mut self.server_ctx,
            ) {
                redraw = true;
            }
            if self
                .mapeditor
                .handle_event(&event, ui, ctx, &mut self.project, &mut self.server_ctx)
            {
                redraw = true;
            }
            match event {
                TheEvent::IndexChanged(id, index) => {
                    if id.name == "Project Tabs" {
                        self.switch_to_session(
                            index,
                            ui,
                            ctx,
                            &mut update_server_icons,
                            &mut redraw,
                        );
                    }
                }
                TheEvent::CustomUndo(id, p, n) => {
                    if id.name == "ModuleUndo" {
                        let _ = (&p, &n);
                    }
                }
                TheEvent::Custom(id, value) => {
                    if id.name == "Show Help" {
                        if let TheValue::Text(url) = value {
                            _ = open::that(format!("https://www.eldiron.com/{}", url));
                            ctx.ui
                                .set_widget_state("Help".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            self.server_ctx.help_mode = false;
                            redraw = true;
                        }
                    } else if id.name == "Set Project Undo State" {
                        UNDOMANAGER.read().unwrap().set_undo_state_to_ui(ctx);
                    } else if id.name == "Pick Tile Source" {
                        if let TheValue::List(values) = value {
                            let picked = match values.as_slice() {
                                [TheValue::Text(kind), TheValue::Id(id)] if kind == "single" => {
                                    Some(rusterix::TileSource::SingleTile(*id))
                                }
                                [TheValue::Text(kind), TheValue::Id(id)] if kind == "group" => {
                                    Some(rusterix::TileSource::TileGroup(*id))
                                }
                                [TheValue::Text(kind), TheValue::Id(id), TheValue::Int(index)]
                                    if kind == "group_member" =>
                                {
                                    Some(rusterix::TileSource::TileGroupMember {
                                        group_id: *id,
                                        member_index: (*index).max(0) as u16,
                                    })
                                }
                                [TheValue::Text(kind), TheValue::Id(id)]
                                    if kind == "procedural" =>
                                {
                                    Some(rusterix::TileSource::Procedural(*id))
                                }
                                _ => None,
                            };

                            if let Some(source) = picked {
                                self.server_ctx.curr_tile_source = Some(source.clone());
                                self.server_ctx.curr_tile_id = match source {
                                    rusterix::TileSource::SingleTile(tile_id) => Some(tile_id),
                                    rusterix::TileSource::TileGroupMember {
                                        group_id,
                                        member_index,
                                    } => self
                                        .project
                                        .tile_groups
                                        .get(&group_id)
                                        .and_then(|group| group.members.get(member_index as usize))
                                        .map(|member| member.tile_id),
                                    rusterix::TileSource::TileGroup(group_id) => self
                                        .project
                                        .tile_groups
                                        .get(&group_id)
                                        .and_then(|group| group.members.first())
                                        .map(|member| member.tile_id),
                                    rusterix::TileSource::Procedural(_) => None,
                                };

                                if let Some(tile_id) = self.server_ctx.curr_tile_id {
                                    ctx.ui.send(TheEvent::Custom(
                                        TheId::named("Tile Picked"),
                                        TheValue::Id(tile_id),
                                    ));
                                }
                                self.activate_edit_tile_meta_action();
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Tilepicker"),
                                    TheValue::Empty,
                                ));
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Reveal Tilepicker Source"),
                                    TheValue::Empty,
                                ));
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Action List"),
                                    TheValue::Empty,
                                ));
                                redraw = true;
                            }
                        }
                    } else if id.name == "Open Tile Node Group Workflow" {
                        self.server_ctx.tile_node_group_id = if let TheValue::Id(group_id) = value {
                            Some(group_id)
                        } else {
                            None
                        };
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Open Tile Node Editor Skeleton"),
                            value.clone(),
                        ));
                        let mut dm = DOCKMANAGER.write().unwrap();
                        dm.set_dock("Tiles".into(), ui, ctx, &self.project, &mut self.server_ctx);
                        dm.edit_maximize(ui, ctx, &mut self.project, &mut self.server_ctx);
                        redraw = true;
                    } else if id.name == "Open Builder Graph Workflow" {
                        if let TheValue::Id(builder_id) = value {
                            self.server_ctx.curr_builder_graph_id = Some(builder_id);
                        }
                        let mut dm = DOCKMANAGER.write().unwrap();
                        dm.set_dock(
                            "Builder".into(),
                            ui,
                            ctx,
                            &self.project,
                            &mut self.server_ctx,
                        );
                        dm.edit_maximize(ui, ctx, &mut self.project, &mut self.server_ctx);
                        redraw = true;
                    } else if id.name == "Close Tile Node Editor Skeleton" {
                        self.server_ctx.tile_node_group_id = None;
                        DOCKMANAGER.write().unwrap().minimize(ui, ctx);
                        redraw = true;
                    } else if id.name == "Minimize Dock" {
                        DOCKMANAGER.write().unwrap().minimize(ui, ctx);
                        ctx.ui.relayout = true;
                        ctx.ui.redraw_all = true;
                        redraw = true;
                    } else if id.name == "Mark Rusterix Dirty" {
                        RUSTERIX.write().unwrap().set_dirty();
                        redraw = true;
                    } else if id.name == "Render SceneManager Map" {
                        if self.server_ctx.pc.is_region() {
                            if self.server_ctx.editor_view_mode == EditorViewMode::D2
                                && self.server_ctx.profile_view.is_some()
                            {
                            } else {
                                crate::utils::editor_scene_full_rebuild(
                                    &self.project,
                                    &self.server_ctx,
                                );
                                if self.server_ctx.editor_view_mode != EditorViewMode::D2 {
                                    TOOLLIST.write().unwrap().update_geometry_overlay_3d(
                                        &mut self.project,
                                        &mut self.server_ctx,
                                    );
                                }
                            }
                        }
                    } else if id.name == "Tool Changed" {
                        TOOLLIST
                            .write()
                            .unwrap()
                            .update_geometry_overlay_3d(&mut self.project, &mut self.server_ctx);
                    } else if id.name == "Update Client Properties" {
                        let mut rusterix = RUSTERIX.write().unwrap();
                        self.build_values.set(
                            "no_rect_geo",
                            rusterix::Value::Bool(self.server_ctx.no_rect_geo_on_map),
                        );
                        self.build_values.set(
                            "editing_slice",
                            rusterix::Value::Float(self.server_ctx.editing_slice),
                        );
                        self.build_values.set(
                            "editing_slice_height",
                            rusterix::Value::Float(self.server_ctx.editing_slice_height),
                        );
                        rusterix
                            .client
                            .builder_d2
                            .set_properties(&self.build_values);
                        rusterix.set_dirty();
                    }
                }

                TheEvent::DialogValueOnClose(role, name, uuid, _value) => {
                    if name == "Delete Character Instance ?" {
                        if role == TheDialogButtonRole::Delete {
                            if let Some(region) =
                                self.project.get_region_mut(&self.server_ctx.curr_region)
                            {
                                let character_id = uuid;
                                if region.characters.shift_remove(&character_id).is_some() {
                                    self.server_ctx.curr_region_content = ContentContext::Unknown;
                                    region.map.selected_entity_item = None;
                                    redraw = true;

                                    // Remove from the content list
                                    if let Some(list) = ui.get_list_layout("Region Content List") {
                                        list.remove(TheId::named_with_id(
                                            "Region Content List Item",
                                            character_id,
                                        ));
                                        ui.select_first_list_item("Region Content List", ctx);
                                        ctx.ui.relayout = true;
                                    }
                                    insert_content_into_maps(&mut self.project);
                                    RUSTERIX.write().unwrap().set_dirty();
                                }
                            }
                        }
                    } else if name == "Delete Item Instance ?" {
                        if role == TheDialogButtonRole::Delete {
                            if let Some(region) =
                                self.project.get_region_mut(&self.server_ctx.curr_region)
                            {
                                let item_id = uuid;
                                if region.items.shift_remove(&item_id).is_some() {
                                    self.server_ctx.curr_region_content = ContentContext::Unknown;
                                    redraw = true;

                                    // Remove from the content list
                                    if let Some(list) = ui.get_list_layout("Region Content List") {
                                        list.remove(TheId::named_with_id(
                                            "Region Content List Item",
                                            item_id,
                                        ));
                                        ui.select_first_list_item("Region Content List", ctx);
                                        ctx.ui.relayout = true;
                                    }
                                    insert_content_into_maps(&mut self.project);
                                    RUSTERIX.write().unwrap().set_dirty();
                                }
                            }
                        }
                    } else if name == "Close Project Tab" && role == TheDialogButtonRole::Accept {
                        self.close_active_session(ui, ctx, &mut update_server_icons, &mut redraw);
                    } else if name == "Update Eldiron" && role == TheDialogButtonRole::Accept {
                        #[cfg(all(
                            feature = "self-update",
                            any(target_os = "windows", target_os = "linux")
                        ))]
                        {
                            let updater = self.self_updater.lock().unwrap();

                            if updater.has_newer_release() {
                                let release = updater.latest_release().cloned().unwrap();

                                let updater = Arc::clone(&self.self_updater);
                                let tx = self.self_update_tx.clone();

                                self.self_update_tx
                                    .send(SelfUpdateEvent::UpdateStart(release.clone()))
                                    .unwrap();

                                thread::spawn(move || {
                                    match updater.lock().unwrap().update_latest() {
                                        Ok(status) => match status {
                                            self_update::Status::UpToDate(_) => {
                                                tx.send(SelfUpdateEvent::AlreadyUpToDate).unwrap();
                                            }
                                            self_update::Status::Updated(_) => {
                                                tx.send(SelfUpdateEvent::UpdateCompleted(release))
                                                    .unwrap();
                                            }
                                        },
                                        Err(err) => {
                                            tx.send(SelfUpdateEvent::UpdateError(err.to_string()))
                                                .unwrap();
                                        }
                                    }
                                });
                            } else {
                                self.self_update_tx
                                    .send(SelfUpdateEvent::AlreadyUpToDate)
                                    .unwrap();
                            }
                        }
                    }
                }
                TheEvent::RenderViewDrop(_id, location, drop) => {
                    if drop.id.name.starts_with("Shader") {
                        return true;
                    }

                    let mut grid_pos = Vec2::zero();
                    let mut spawn_y = 0.0;
                    let mut placement_reference_y: Option<f32> = None;
                    let use_3d_hit = self.server_ctx.editor_view_mode != EditorViewMode::D2;
                    let placement_clearance = if drop.id.name.starts_with("Character") {
                        2.0
                    } else {
                        1.0
                    };

                    if let Some(map) = self.project.get_map(&self.server_ctx) {
                        if use_3d_hit && let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            let screen_uv = [
                                location.x as f32 / dim.width as f32,
                                location.y as f32 / dim.height as f32,
                            ];
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.scene_handler.vm.set_active_vm(0);
                            let ray = rusterix.scene_handler.vm.ray_from_uv_with_size(
                                dim.width as u32,
                                dim.height as u32,
                                screen_uv,
                            );

                            if let Some(raw) = rusterix.scene_handler.vm.pick_geo_id_at_uv(
                                dim.width as u32,
                                dim.height as u32,
                                screen_uv,
                                false,
                                false,
                            ) {
                                let floor_candidates = map
                                    .geometry_floor_candidates_at(Vec2::new(raw.1.x, raw.1.z))
                                    .into_iter()
                                    .map(|floor| format!("{:.3}", floor.height))
                                    .collect::<Vec<_>>()
                                    .join(",");
                                if let Some((ray_origin, ray_dir)) = ray
                                    && let Some((floor_hit, reference_y)) = map
                                        .geometry_floor_hit_from_ray_for_placement(
                                            ray_origin,
                                            ray_dir,
                                            raw.1,
                                            placement_clearance,
                                        )
                                {
                                    grid_pos = Vec2::new(floor_hit.x, floor_hit.z);
                                    spawn_y = floor_hit.y;
                                    placement_reference_y = Some(reference_y);
                                    eprintln!(
                                        "[EntityPlacementDebug] viewport drop raw=({:.3},{:.3},{:.3}) raw_floors=[{}] resolved=({:.3},{:.3},{:.3}) reference_y={:.3} clearance={:.3}",
                                        raw.1.x,
                                        raw.1.y,
                                        raw.1.z,
                                        floor_candidates,
                                        floor_hit.x,
                                        floor_hit.y,
                                        floor_hit.z,
                                        reference_y,
                                        placement_clearance
                                    );
                                } else {
                                    grid_pos = Vec2::new(raw.1.x, raw.1.z);
                                    spawn_y = raw.1.y;
                                    placement_reference_y = Some(raw.1.y);
                                    eprintln!(
                                        "[EntityPlacementDebug] viewport drop raw fallback raw=({:.3},{:.3},{:.3}) raw_floors=[{}] clearance={:.3}",
                                        raw.1.x,
                                        raw.1.y,
                                        raw.1.z,
                                        floor_candidates,
                                        placement_clearance
                                    );
                                }
                            } else {
                                grid_pos = self.server_ctx.local_to_map_cell(
                                    Vec2::new(dim.width as f32, dim.height as f32),
                                    Vec2::new(location.x as f32, location.y as f32),
                                    map,
                                    map.subdivisions,
                                );
                                grid_pos += 0.5;
                            }
                        } else if let Some(render_view) = ui.get_render_view("PolyView") {
                            let dim = *render_view.dim();
                            grid_pos = self.server_ctx.local_to_map_cell(
                                Vec2::new(dim.width as f32, dim.height as f32),
                                Vec2::new(location.x as f32, location.y as f32),
                                map,
                                map.subdivisions,
                            );
                            grid_pos += 0.5;
                            let mut best_height: Option<f32> = None;
                            for sector in map
                                .sectors
                                .iter()
                                .filter(|s| s.layer.is_none() && s.is_inside(map, grid_pos))
                            {
                                let mut vertex_ids: Vec<u32> = Vec::new();
                                let mut sum_y = 0.0f32;
                                let mut count = 0usize;
                                for linedef_id in &sector.linedefs {
                                    if let Some(ld) = map.find_linedef(*linedef_id) {
                                        if !vertex_ids.contains(&ld.start_vertex) {
                                            vertex_ids.push(ld.start_vertex);
                                            if let Some(v) = map.get_vertex_3d(ld.start_vertex) {
                                                sum_y += v.y;
                                                count += 1;
                                            }
                                        }
                                        if !vertex_ids.contains(&ld.end_vertex) {
                                            vertex_ids.push(ld.end_vertex);
                                            if let Some(v) = map.get_vertex_3d(ld.end_vertex) {
                                                sum_y += v.y;
                                                count += 1;
                                            }
                                        }
                                    }
                                }
                                if count > 0 {
                                    let h = sum_y / count as f32;
                                    best_height = Some(best_height.map_or(h, |prev| prev.max(h)));
                                }
                            }
                            if let Some(h) = best_height {
                                spawn_y = h;
                            }
                        }

                        if use_3d_hit {
                            let floor_height = if let Some(reference_y) = placement_reference_y {
                                map.geometry_floor_height_nearest(grid_pos, reference_y)
                            } else {
                                map.geometry_floor_height_at(grid_pos)
                            };
                            if let Some(height) = floor_height {
                                eprintln!(
                                    "[EntityPlacementDebug] viewport drop height recheck grid=({:.3},{:.3}) reference_y={:?} before={:.3} after={:.3}",
                                    grid_pos.x, grid_pos.y, placement_reference_y, spawn_y, height
                                );
                                spawn_y = height;
                            }
                        }
                    }

                    if drop.id.name.starts_with("Character") {
                        let mut instance = Character {
                            character_id: drop.id.references,
                            position: Vec3::new(grid_pos.x, spawn_y, grid_pos.y),
                            ..Default::default()
                        };

                        let mut name = "Character".to_string();
                        if let Some(character) = self.project.characters.get(&drop.id.references) {
                            name.clone_from(&character.name);
                        }
                        instance.name = name.clone();

                        let atom = ProjectUndoAtom::AddRegionCharacterInstance(
                            self.server_ctx.curr_region,
                            instance,
                        );
                        atom.redo(&mut self.project, ui, ctx, &mut self.server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    } else if drop.id.name.starts_with("Item") {
                        let mut instance = Item {
                            item_id: drop.id.references,
                            position: Vec3::new(grid_pos.x, spawn_y, grid_pos.y),
                            ..Default::default()
                        };

                        let mut name = "Item".to_string();
                        if let Some(item) = self.project.items.get(&drop.id.references) {
                            name.clone_from(&item.name);
                        }
                        instance.name = name;

                        let atom = ProjectUndoAtom::AddRegionItemInstance(
                            self.server_ctx.curr_region,
                            instance,
                        );
                        atom.redo(&mut self.project, ui, ctx, &mut self.server_ctx);
                        UNDOMANAGER.write().unwrap().add_undo(atom, ctx);
                    }
                }
                TheEvent::FileRequesterResult(id, paths) => {
                    // Load a palette from a file
                    if id.name == "Palette Import" {
                        for p in paths {
                            let contents = std::fs::read_to_string(p).unwrap_or("".to_string());
                            let prev = self.project.art_palette.clone();
                            let prev_materials = self.project.art_palette_materials.clone();
                            self.project.load_art_palette_from_text(contents);
                            *PALETTE.write().unwrap() = self.project.art_palette.clone();
                            {
                                let mut rusterix = RUSTERIX.write().unwrap();
                                rusterix.assets.palette = self.project.art_palette.clone();
                                rusterix.assets.palette_materials =
                                    crate::undo::project_helper::palette_material_values(
                                        &self.project,
                                    );
                                rusterix.assets.palette_material_ids =
                                    crate::undo::project_helper::palette_material_ids(
                                        &self.project,
                                    );
                                rusterix.set_tiles(self.project.tiles.clone(), true);
                                rusterix.set_tile_groups(self.project.tile_groups.clone());
                            }

                            if let Some(palette_picker) = ui.get_palette_picker("Palette Picker") {
                                let index = palette_picker.index();

                                palette_picker.set_palette(self.project.art_palette.clone());
                                if let Some(widget) = ui.get_widget("Palette Color Picker") {
                                    if let Some(color) = &self.project.art_palette[index] {
                                        widget.set_value(TheValue::ColorObject(color.clone()));
                                    }
                                }
                                if let Some(widget) = ui.get_widget("Palette Hex Edit") {
                                    if let Some(color) = &self.project.art_palette[index] {
                                        widget.set_value(TheValue::Text(color.to_hex()));
                                    }
                                }
                            }
                            redraw = true;

                            let undo = ProjectUndoAtom::PaletteEdit(
                                prev,
                                prev_materials,
                                self.project.art_palette.clone(),
                                self.project.art_palette_materials.clone(),
                            );
                            UNDOMANAGER.write().unwrap().add_undo(undo, ctx);
                        }
                    } else
                    // Open
                    if id.name == "Open" {
                        for p in paths {
                            if let Some(loaded) = Self::load_project_from_json_path(&p) {
                                self.open_project_as_session(
                                    loaded,
                                    Some(p.clone()),
                                    ui,
                                    ctx,
                                    &mut update_server_icons,
                                    &mut redraw,
                                );
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Project loaded successfully.".to_string(),
                                ));
                            } else {
                                self.replace_next_project_load_in_active_tab = false;
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Unable to load project!".to_string(),
                                ));
                            }
                        }
                    } else if id.name == "Save As" {
                        for p in paths {
                            let p = Self::ensure_project_extension(p);
                            self.persist_active_region_view_state();
                            let json = serde_json::to_string(&self.project);
                            if let Ok(json) = json {
                                if std::fs::write(p.clone(), json).is_ok() {
                                    self.project_path = Some(p);
                                    UNDOMANAGER.write().unwrap().mark_saved();
                                    DOCKMANAGER.write().unwrap().mark_saved();
                                    if self.active_session < self.sessions.len() {
                                        self.sessions[self.active_session].dirty = false;
                                    }
                                    self.sync_active_session_from_editor();
                                    self.rebuild_project_tabs(ui);
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Project saved successfully.".to_string(),
                                    ))
                                } else {
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Unable to save project!".to_string(),
                                    ))
                                }
                            }
                        }
                    }
                }
                TheEvent::StateChanged(id, state) => {
                    if id.name == "Help" {
                        self.server_ctx.help_mode = state == TheWidgetState::Clicked;
                    }
                    if id.name == "GameInput" {
                        self.server_ctx.game_input_mode = state == TheWidgetState::Clicked;
                    } else if id.name == "Starter Project List Item"
                        && state == TheWidgetState::Selected
                    {
                        self.selected_starter_manifest_id = self
                            .starter_projects
                            .iter()
                            .find(|entry| entry.id == id.uuid)
                            .map(|entry| entry.manifest_id.clone());
                        redraw = true;
                    } else if id.name == Self::STARTER_CREATE_ID {
                        let selected_manifest_id =
                            self.selected_starter_manifest_id.clone().or_else(|| {
                                self.starter_projects
                                    .first()
                                    .map(|entry| entry.manifest_id.clone())
                            });
                        if let Some(manifest_id) = selected_manifest_id {
                            if let Some(project) = self.load_named_starter_project(&manifest_id) {
                                ui.clear_dialog();
                                self.open_project_as_session(
                                    project,
                                    None,
                                    ui,
                                    ctx,
                                    &mut update_server_icons,
                                    &mut redraw,
                                );
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_starter_initialized"),
                                ));
                            } else {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    fl!("status_starter_load_failed"),
                                ));
                            }
                        }
                        ctx.ui.set_widget_state(
                            Self::STARTER_CREATE_ID.to_string(),
                            TheWidgetState::None,
                        );
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == Self::STARTER_CANCEL_ID {
                        ui.clear_dialog();
                        ctx.ui.set_widget_state(
                            Self::STARTER_CANCEL_ID.to_string(),
                            TheWidgetState::None,
                        );
                        ctx.ui.clear_hover();
                        self.open_project_as_session(
                            Self::load_empty_project_template(),
                            None,
                            ui,
                            ctx,
                            &mut update_server_icons,
                            &mut redraw,
                        );
                        redraw = true;
                    } else if id.name == "New" {
                        self.open_starter_project_dialog(ui, ctx);
                        ctx.ui.send(TheEvent::SetStatusText(
                            TheId::empty(),
                            fl!("status_starter_choose"),
                        ));
                        ctx.ui
                            .set_widget_state("New".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Logo" {
                        _ = open::that("https://eldiron.com");
                        ctx.ui
                            .set_widget_state("Logo".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Patreon" {
                        _ = open::that("https://www.patreon.com/eldiron");
                        ctx.ui
                            .set_widget_state("Patreon".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Update" {
                        #[cfg(all(
                            feature = "self-update",
                            any(target_os = "windows", target_os = "linux")
                        ))]
                        {
                            let updater = self.self_updater.lock().unwrap();

                            if updater.has_newer_release() {
                                self.self_update_tx
                                    .send(SelfUpdateEvent::UpdateConfirm(
                                        updater.latest_release().cloned().unwrap(),
                                    ))
                                    .unwrap();
                            } else {
                                if let Some(statusbar) = ui.get_widget("Statusbar") {
                                    statusbar
                                        .as_statusbar()
                                        .unwrap()
                                        .set_text(fl!("info_update_check"));
                                }

                                let updater = Arc::clone(&self.self_updater);
                                let tx = self.self_update_tx.clone();

                                thread::spawn(move || {
                                    let mut updater = updater.lock().unwrap();

                                    match updater.fetch_release_list() {
                                        Ok(_) => {
                                            if updater.has_newer_release() {
                                                tx.send(SelfUpdateEvent::UpdateConfirm(
                                                    updater.latest_release().cloned().unwrap(),
                                                ))
                                                .unwrap();
                                            } else {
                                                tx.send(SelfUpdateEvent::AlreadyUpToDate).unwrap();
                                            }
                                        }
                                        Err(err) => {
                                            tx.send(SelfUpdateEvent::UpdateError(err.to_string()))
                                                .unwrap();
                                        }
                                    }
                                });
                            }

                            ctx.ui
                                .set_widget_state("Update".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        }
                        #[cfg(all(feature = "self-update", target_os = "macos"))]
                        {
                            _ = open::that("https://github.com/markusmoenig/Eldiron/releases");
                            ctx.ui
                                .set_widget_state("Update".to_string(), TheWidgetState::None);
                            ctx.ui.clear_hover();
                            redraw = true;
                        }
                    } else if id.name == "Open" {
                        ctx.ui.open_file_requester(
                            TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                            "Open".into(),
                            TheFileExtension::new("Eldiron".into(), vec!["eldiron".to_string()]),
                        );
                        ctx.ui
                            .set_widget_state("Open".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Close" {
                        if self.active_session_has_changes() {
                            let uuid = Uuid::new_v4();
                            let width = 380;
                            let height = 110;

                            let mut canvas = TheCanvas::new();
                            canvas.limiter_mut().set_max_size(Vec2::new(width, height));

                            let mut hlayout: TheHLayout = TheHLayout::new(TheId::empty());
                            hlayout.limiter_mut().set_max_width(width);

                            let mut text_widget =
                                TheText::new(TheId::named_with_id("Dialog Value", uuid));
                            text_widget.set_text(
                                "This tab has unsaved changes. Close it anyway?".to_string(),
                            );
                            text_widget.limiter_mut().set_max_width(280);
                            hlayout.add_widget(Box::new(text_widget));

                            canvas.set_layout(hlayout);
                            ui.show_dialog(
                                "Close Project Tab",
                                canvas,
                                vec![TheDialogButtonRole::Accept, TheDialogButtonRole::Reject],
                                ctx,
                            );
                        } else {
                            self.close_active_session(
                                ui,
                                ctx,
                                &mut update_server_icons,
                                &mut redraw,
                            );
                        }
                        ctx.ui
                            .set_widget_state("Close".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    } else if id.name == "Save" {
                        if let Some(path) = self.project_path.clone() {
                            let path = Self::ensure_project_extension(path);
                            let mut success = false;
                            // if let Ok(output) = postcard::to_allocvec(&self.project) {
                            self.persist_active_region_view_state();
                            if let Ok(output) = serde_json::to_string(&self.project) {
                                if std::fs::write(&path, output).is_ok() {
                                    self.project_path = Some(path.clone());
                                    UNDOMANAGER.write().unwrap().mark_saved();
                                    DOCKMANAGER.write().unwrap().mark_saved();
                                    if self.active_session < self.sessions.len() {
                                        self.sessions[self.active_session].dirty = false;
                                    }
                                    self.sync_active_session_from_editor();
                                    self.rebuild_project_tabs(ui);
                                    ctx.ui.send(TheEvent::SetStatusText(
                                        TheId::empty(),
                                        "Project saved successfully.".to_string(),
                                    ));
                                    success = true;
                                }
                            }

                            if !success {
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Unable to save project!".to_string(),
                                ))
                            }
                        } else {
                            ctx.ui.send(TheEvent::StateChanged(
                                TheId::named("Save As"),
                                TheWidgetState::Clicked,
                            ));
                            ctx.ui
                                .set_widget_state("Save".to_string(), TheWidgetState::None);
                        }
                    } else if id.name == "Save As" {
                        ctx.ui.save_file_requester(
                            TheId::named_with_id(id.name.as_str(), Uuid::new_v4()),
                            "Save".into(),
                            TheFileExtension::new(
                                "Eldiron".into(),
                                vec![Self::PROJECT_EXTENSION.to_string()],
                            ),
                        );
                        ctx.ui
                            .set_widget_state("Save As".to_string(), TheWidgetState::None);
                        ctx.ui.clear_hover();
                        redraw = true;
                    }
                    // Server
                    else if id.name == "Play" {
                        let state = RUSTERIX.read().unwrap().server.state;
                        if state == rusterix::ServerState::Paused {
                            self.pending_game_messages.clear();
                            self.pending_game_choices.clear();
                            TEXTGAME.write().unwrap().reset();
                            if self.server_ctx.text_game_mode {
                                TEXTGAME.write().unwrap().sync_output(ui, ctx);
                            }
                            RUSTERIX.write().unwrap().server.continue_instances();
                            update_server_icons = true;
                        } else {
                            if state == rusterix::ServerState::Off {
                                self.pending_game_messages.clear();
                                self.pending_game_choices.clear();
                                TEXTGAME.write().unwrap().reset();
                                if self.server_ctx.text_game_mode {
                                    TEXTGAME.write().unwrap().sync_output(ui, ctx);
                                }
                                start_server(
                                    &mut RUSTERIX.write().unwrap(),
                                    &mut self.project,
                                    true,
                                );
                                RUSTERIX.write().unwrap().clear_say_messages();
                                let commands =
                                    setup_client(&mut RUSTERIX.write().unwrap(), &mut self.project);
                                RUSTERIX
                                    .write()
                                    .unwrap()
                                    .server
                                    .process_client_commands(commands);
                                warmup_runtime(
                                    &mut RUSTERIX.write().unwrap(),
                                    &mut self.project,
                                    3,
                                );
                                ctx.ui.send(TheEvent::SetStatusText(
                                    TheId::empty(),
                                    "Server has been started.".to_string(),
                                ));
                                ui.set_widget_value("LogEdit", ctx, TheValue::Text(String::new()));
                                self.last_processed_log_len = 0;
                                RUSTERIX.write().unwrap().player_camera = PlayerCamera::D2;
                            }
                            update_server_icons = true;
                        }
                    } else if id.name == "Pause" {
                        let state = RUSTERIX.read().unwrap().server.state;
                        if state == rusterix::ServerState::Running {
                            RUSTERIX.write().unwrap().server.pause();
                            update_server_icons = true;
                        }
                    } else if id.name == "Stop" {
                        RUSTERIX.write().unwrap().server.stop();
                        RUSTERIX.write().unwrap().clear_say_messages();
                        RUSTERIX.write().unwrap().player_camera = PlayerCamera::D2;
                        {
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.client.scene.d2_dynamic.clear();
                            rusterix.client.scene.d3_dynamic.clear();
                            rusterix.client.scene.dynamic_lights.clear();
                            rusterix.scene_handler.clear_runtime_overlays();
                            rusterix.set_dirty();
                        }

                        ui.set_widget_value("InfoView", ctx, TheValue::Text("".into()));
                        insert_content_into_maps(&mut self.project);
                        update_server_icons = true;

                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Render SceneManager Map"),
                            TheValue::Empty,
                        ));
                    } else if id.name == "Show Settings" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::ProjectSettings,
                        );
                        redraw = true;
                    } else if id.name == "Show Rules" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::GameRules,
                        );
                        redraw = true;
                    } else if id.name == "Show Locales" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::GameLocales,
                        );
                        redraw = true;
                    } else if id.name == "Show Audio FX" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::GameAudioFx,
                        );
                        redraw = true;
                    } else if id.name == "Show Authoring" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::GameAuthoring,
                        );
                        redraw = true;
                    } else if id.name == "Show Debug Log" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::DebugLog,
                        );
                        redraw = true;
                    } else if id.name == "Show Console" {
                        set_project_context(
                            ctx,
                            ui,
                            &self.project,
                            &mut self.server_ctx,
                            ProjectContext::Console,
                        );
                        redraw = true;
                    } else if id.name == "Undo" || id.name == "Redo" {
                        let mut refresh_action_ui = false;
                        if ui.focus_widget_supports_undo_redo(ctx) {
                            if id.name == "Undo" {
                                ui.undo(ctx);
                            } else {
                                ui.redo(ctx);
                            }
                        } else if DOCKMANAGER.read().unwrap().current_dock_supports_undo() {
                            if id.name == "Undo" {
                                DOCKMANAGER.write().unwrap().undo(
                                    ui,
                                    ctx,
                                    &mut self.project,
                                    &mut self.server_ctx,
                                );
                            } else {
                                DOCKMANAGER.write().unwrap().redo(
                                    ui,
                                    ctx,
                                    &mut self.project,
                                    &mut self.server_ctx,
                                );
                            }
                            refresh_action_ui = true;
                        } else {
                            let mut manager = UNDOMANAGER.write().unwrap();

                            if id.name == "Undo" {
                                manager.undo(&mut self.server_ctx, &mut self.project, ui, ctx);
                            } else {
                                manager.redo(&mut self.server_ctx, &mut self.project, ui, ctx);
                            }
                            refresh_action_ui = true;
                        }

                        // Keep action list and TOML params in sync only when project/dock state changed.
                        if refresh_action_ui {
                            // Drop focus to avoid stale focused text-edit state surviving toolbar rebuilds.
                            ctx.ui.clear_focus();
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Action List"),
                                TheValue::Empty,
                            ));
                            ctx.ui.send(TheEvent::Custom(
                                TheId::named("Update Action Parameters"),
                                TheValue::Empty,
                            ));
                        }
                    } else if id.name == "Cut" {
                        if ui.focus_widget_supports_clipboard(ctx) {
                            // Widget specific
                            ui.cut(ctx);
                        } else {
                            // Global
                            ctx.ui.send(TheEvent::Cut);
                        }
                    } else if id.name == "Copy" {
                        if ui.focus_widget_supports_clipboard(ctx) {
                            // Widget specific
                            ui.copy(ctx);
                        } else {
                            // Global
                            ctx.ui.send(TheEvent::Copy);
                        }
                    } else if id.name == "Paste" {
                        Self::refresh_system_text_clipboard(ctx);
                        if ui.focus_widget_supports_clipboard(ctx) {
                            // Widget specific
                            ui.paste(ctx);
                        } else {
                            // Global
                            if let Some(value) = &ctx.ui.clipboard {
                                ctx.ui.send(TheEvent::Paste(
                                    value.clone(),
                                    ctx.ui.clipboard_app_type.clone(),
                                ));
                            } else {
                                ctx.ui.send(TheEvent::Paste(
                                    TheValue::Empty,
                                    ctx.ui.clipboard_app_type.clone(),
                                ));
                            }
                        }
                    }
                }
                TheEvent::ValueChanged(id, value) => {
                    if id.name == "Server Time Slider" {
                        if let TheValue::Time(time) = value {
                            self.project.time = time;
                            let mut rusterix = RUSTERIX.write().unwrap();
                            rusterix.client.set_server_time(time);

                            if rusterix.server.state == rusterix::ServerState::Running {
                                if let Some(map) = self.project.get_map(&self.server_ctx) {
                                    rusterix.server.set_time(&map.id, time);
                                }
                            }
                            rusterix.set_dirty();
                            redraw = true;
                        }
                    } else if id.name == TextGameState::GAME_INPUT_ID {
                        if let Some(command) = value.to_string() {
                            self.pending_text_game_command =
                                Some((id.name.clone(), command.clone()));
                            redraw = true;
                        }
                    } else if id.name == TextGameState::DOCK_INPUT_ID {
                        if let Some(command) = value.to_string() {
                            self.pending_text_game_command =
                                Some((id.name.clone(), command.clone()));
                            redraw = true;
                        }
                    }
                }
                _ => {}
            }
        }

        #[cfg(all(
            feature = "self-update",
            any(target_os = "windows", target_os = "linux", target_os = "macos")
        ))]
        while let Ok(event) = self.self_update_rx.try_recv() {
            match event {
                SelfUpdateEvent::AlreadyUpToDate => {
                    Self::set_update_button(ui, ctx, None);

                    let text = str!("Eldiron is already up-to-date.");
                    let uuid = Uuid::new_v4();

                    let width = 300;
                    let height = 100;

                    let mut canvas = TheCanvas::new();
                    canvas.limiter_mut().set_max_size(Vec2::new(width, height));

                    let mut hlayout: TheHLayout = TheHLayout::new(TheId::empty());
                    hlayout.limiter_mut().set_max_width(width);

                    let mut text_widget = TheText::new(TheId::named_with_id("Dialog Value", uuid));
                    text_widget.set_text(text.to_string());
                    text_widget.limiter_mut().set_max_width(200);
                    hlayout.add_widget(Box::new(text_widget));

                    canvas.set_layout(hlayout);

                    ui.show_dialog(
                        "Eldiron Up-to-Date",
                        canvas,
                        vec![TheDialogButtonRole::Accept],
                        ctx,
                    );
                }
                SelfUpdateEvent::UpdateAvailable(release) => {
                    Self::set_update_button(ui, ctx, Some(&release));
                }
                SelfUpdateEvent::UpdateCompleted(release) => {
                    Self::set_update_button(ui, ctx, None);

                    if let Some(statusbar) = ui.get_widget("Statusbar") {
                        statusbar.as_statusbar().unwrap().set_text(format!(
                            "Updated to version {}. Please restart the application to enjoy the new features.",
                            release.version
                        ));
                    }
                }
                SelfUpdateEvent::UpdateConfirm(release) => {
                    Self::set_update_button(ui, ctx, Some(&release));

                    let text = &format!("Update to version {}?", release.version);
                    let uuid = Uuid::new_v4();

                    let width = 300;
                    let height = 100;

                    let mut canvas = TheCanvas::new();
                    canvas.limiter_mut().set_max_size(Vec2::new(width, height));

                    let mut hlayout: TheHLayout = TheHLayout::new(TheId::empty());
                    hlayout.limiter_mut().set_max_width(width);

                    let mut text_widget = TheText::new(TheId::named_with_id("Dialog Value", uuid));
                    text_widget.set_text(text.to_string());
                    text_widget.limiter_mut().set_max_width(200);
                    hlayout.add_widget(Box::new(text_widget));

                    canvas.set_layout(hlayout);

                    ui.show_dialog(
                        "Update Eldiron",
                        canvas,
                        vec![TheDialogButtonRole::Accept, TheDialogButtonRole::Reject],
                        ctx,
                    );
                }
                SelfUpdateEvent::UpdateError(err) => {
                    Self::set_update_button(ui, ctx, None);

                    if let Some(statusbar) = ui.get_widget("Statusbar") {
                        statusbar
                            .as_statusbar()
                            .unwrap()
                            .set_text(format!("Failed to update Eldiron: {err}"));
                    }
                }
                SelfUpdateEvent::UpdateStart(release) => {
                    Self::set_update_button(ui, ctx, None);

                    if let Some(statusbar) = ui.get_widget("Statusbar") {
                        statusbar
                            .as_statusbar()
                            .unwrap()
                            .set_text(format!("Updating to version {}...", release.version));
                    }
                }
            }
        }

        if update_server_icons {
            self.update_server_state_icons(ui);
            redraw = true;
        }

        let active_dirty = UNDOMANAGER.read().unwrap().has_unsaved()
            || DOCKMANAGER.read().unwrap().has_dock_changes();
        if self.active_session < self.sessions.len()
            && self.sessions[self.active_session].dirty != active_dirty
        {
            self.sessions[self.active_session].dirty = active_dirty;
            self.rebuild_project_tabs(ui);
            redraw = true;
        }
        if active_dirty != self.last_active_dirty {
            self.last_active_dirty = active_dirty;
            self.rebuild_project_tabs(ui);
            redraw = true;
        }

        self.update_counter += 1;
        if self.update_counter > 2 {
            self.sidebar.startup = false;
        }
        redraw
    }

    fn mouse_motion(&mut self, delta_x: f32, delta_y: f32, ctx: &mut TheContext) -> bool {
        if self.server_ctx.game_input_mode
            || self.server_ctx.editor_view_mode == EditorViewMode::D2
            || self.server_ctx.curr_map_tool_type == MapToolType::Game
        {
            return false;
        }

        let Some(region) = self.project.get_region_mut(&self.server_ctx.curr_region) else {
            return false;
        };

        let delta = Vec2::new(delta_x, delta_y);
        let mut handled = false;
        if self.server_ctx.editor_view_mode == EditorViewMode::FirstP
            && self.server_ctx.editor_fly_nav_active
            && self.server_ctx.editor_fly_nav_mouse_down
        {
            EDITCAMERA
                .write()
                .unwrap()
                .mouse_delta_firstp(region, delta);
            handled = true;
        } else if self.server_ctx.editor_view_mode == EditorViewMode::Orbit {
            EDITCAMERA.write().unwrap().mouse_delta_orbit(delta);
            handled = true;
        } else if self.server_ctx.editor_view_mode == EditorViewMode::Iso {
            EDITCAMERA.write().unwrap().pan_3d_by_delta(
                region,
                &self.server_ctx,
                Vec2::new(delta_x.round() as i32, delta_y.round() as i32),
                Vec2::new(ctx.width as i32, ctx.height as i32),
            );
            handled = true;
        }

        if handled {
            RUSTERIX.write().unwrap().set_dirty();
            ctx.ui.redraw_all = true;
        }
        handled
    }

    /// Returns true if there are changes
    fn has_changes(&self) -> bool {
        if self.active_session_has_changes() {
            return true;
        }

        for (index, session) in self.sessions.iter().enumerate() {
            if index != self.active_session && session.dirty {
                return true;
            }
        }

        false
    }

    fn window_moved(&mut self, x: i32, y: i32) {
        self.window_state.x = Some(x);
        self.window_state.y = Some(y);
        self.save_window_state();
    }

    fn window_resized(&mut self, width: usize, height: usize) {
        if width > 0 && height > 0 {
            self.window_state.width = Some(width);
            self.window_state.height = Some(height);
            self.save_window_state();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_project_extension_appends_when_missing() {
        let path = PathBuf::from("/tmp/My Project");

        assert_eq!(
            Editor::ensure_project_extension(path),
            PathBuf::from("/tmp/My Project.eldiron")
        );
    }

    #[test]
    fn ensure_project_extension_leaves_existing_extension() {
        let path = PathBuf::from("/tmp/My Project.eldiron");

        assert_eq!(Editor::ensure_project_extension(path.clone()), path);
    }

    #[test]
    fn ensure_project_extension_treats_existing_extension_case_insensitively() {
        let path = PathBuf::from("/tmp/My Project.ELDIRON");

        assert_eq!(Editor::ensure_project_extension(path.clone()), path);
    }

    #[test]
    fn ensure_project_extension_appends_after_other_suffixes() {
        let path = PathBuf::from("/tmp/My Project.backup");

        assert_eq!(
            Editor::ensure_project_extension(path),
            PathBuf::from("/tmp/My Project.backup.eldiron")
        );
    }
}

pub trait EldironEditor {
    fn update_server_state_icons(&mut self, ui: &mut TheUI);
}

impl EldironEditor for Editor {
    fn update_server_state_icons(&mut self, ui: &mut TheUI) {
        let rusterix = RUSTERIX.read().unwrap();
        if rusterix.server.state == rusterix::ServerState::Running {
            if let Some(button) = ui.get_widget("Play") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-fill".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Pause") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-pause".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Stop") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("stop".to_string());
                }
            }
        } else if rusterix.server.state == rusterix::ServerState::Paused {
            if let Some(button) = ui.get_widget("Play") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Pause") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-pause-fill".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Stop") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("stop".to_string());
                }
            }
        } else if rusterix.server.state == rusterix::ServerState::Off {
            if let Some(button) = ui.get_widget("Play") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Pause") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("play-pause".to_string());
                }
            }
            if let Some(button) = ui.get_widget("Stop") {
                if let Some(button) = button.as_menubar_button() {
                    button.set_icon_name("stop-fill".to_string());
                }
            }
        }
    }
}
