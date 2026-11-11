use crate::blocks::{
    BLOCK_COLUMN_SEGMENTS, BLOCK_OPERATION_ERASE, BLOCK_OPERATION_PLACE, BLOCK_STROKE_LINE,
    BLOCK_STROKE_RECT, BlockAsset, adjusted_rotated_bounds, block_asset, block_asset_default_color,
    block_asset_default_sizing, block_assets, block_component_kind, bundled_prefab,
    bundled_prefabs, component_uses_cylinder, cylinder_vertices_and_faces, default_block_asset_id,
    ensure_block_asset_default_palette, ensure_prefab_default_surfaces,
    localized_block_asset_description, localized_block_asset_name, prefab_object_default_color,
    prefab_uses_auto_sizing, upgrade_bundled_prefab_geometry,
};
use crate::editor::{RUSTERIX, SCENEMANAGER, UNDOMANAGER};
use crate::prelude::*;
use rusterix::D3Camera;
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};

enum MeshPreviewResult {
    Catalog(Vec<(Uuid, String, String)>),
    Preview(
        Uuid,
        rusterix::GeometryObject,
        IndexMap<Uuid, rusterix::Tile>,
        TheRGBABuffer,
    ),
    Rejected(Uuid),
}

use vek::Vec4;

const BLOCKS_DOCK_BOARD: &str = "Blocks Dock Board";
const BLOCKS_DOCK_TABS: &str = "Blocks Dock Categories";
const BLOCKS_DOCK_OPERATION: &str = "Blocks Dock Operation";
const BLOCKS_DOCK_STROKE: &str = "Blocks Dock Stroke";
const BLOCKS_DOCK_DAMAGE: &str = "Blocks Dock Damage";
const BLOCKS_DOCK_DUPLICATE: &str = "Blocks Dock Duplicate Prefab";
const BLOCKS_DOCK_DELETE: &str = "Blocks Dock Delete Prefab";
pub const BLOCKS_DOCK_SYNC_EVENT: &str = "Blocks Dock Sync";

const BLOCK_PREVIEW_COLORS: [[u8; 4]; 3] = [
    [196, 196, 196, 255],
    [156, 156, 156, 255],
    [116, 116, 116, 255],
];
struct BlocksDockPreviews;

impl BlocksDockPreviews {
    fn downsample_rgba_box(src: &[u8], width: usize, height: usize, factor: usize) -> Vec<u8> {
        if factor <= 1 {
            return src.to_vec();
        }

        let dst_width = width / factor;
        let dst_height = height / factor;
        let mut out = vec![0_u8; dst_width * dst_height * 4];
        let samples = (factor * factor) as u32;

        for y in 0..dst_height {
            for x in 0..dst_width {
                let mut acc = [0_u32; 4];
                for sy in 0..factor {
                    for sx in 0..factor {
                        let src_x = x * factor + sx;
                        let src_y = y * factor + sy;
                        let index = (src_y * width + src_x) * 4;
                        acc[0] += src[index] as u32;
                        acc[1] += src[index + 1] as u32;
                        acc[2] += src[index + 2] as u32;
                        acc[3] += src[index + 3] as u32;
                    }
                }

                let dst = (y * dst_width + x) * 4;
                out[dst] = (acc[0] / samples) as u8;
                out[dst + 1] = (acc[1] / samples) as u8;
                out[dst + 2] = (acc[2] / samples) as u8;
                out[dst + 3] = (acc[3] / samples) as u8;
            }
        }
        out
    }

    fn render_asset_preview(asset: &BlockAsset, size: i32) -> TheRGBABuffer {
        const SSAA: usize = 2;

        let width = size.max(32) as usize;
        let height = size.max(32) as usize;
        let render_width = width * SSAA;
        let render_height = height * SSAA;

        let mut scene = rusterix::Scene::empty();
        let mut min = Vec3::broadcast(f32::INFINITY);
        let mut max = Vec3::broadcast(f32::NEG_INFINITY);

        for index in 0..asset.boxes.len() {
            let Some((box_min, box_max)) =
                adjusted_rotated_bounds(asset, index, block_asset_default_sizing(asset), 0)
            else {
                continue;
            };
            min.x = min.x.min(box_min.x);
            min.y = min.y.min(box_min.y);
            min.z = min.z.min(box_min.z);
            max.x = max.x.max(box_max.x);
            max.y = max.y.max(box_max.y);
            max.z = max.z.max(box_max.z);

            let component = block_component_kind(asset, index);
            let mut batch = if component_uses_cylinder(component) {
                Self::cylinder_batch(box_min, box_max)
            } else {
                rusterix::Batch3D::from_box(
                    box_min.x,
                    box_min.y,
                    box_min.z,
                    box_max.x - box_min.x,
                    box_max.y - box_min.y,
                    box_max.z - box_min.z,
                )
            }
            .source(rusterix::PixelSource::Pixel(
                block_asset_default_color(asset, component)
                    .unwrap_or(BLOCK_PREVIEW_COLORS[index % BLOCK_PREVIEW_COLORS.len()]),
            ))
            .cull_mode(rusterix::CullMode::Off);
            batch.ambient_color = Vec3::new(0.42, 0.44, 0.48);
            scene.d3_static.push(batch);
        }

        if scene.d3_static.is_empty() {
            return TheRGBABuffer::new(TheDim::sized(size, size));
        }

        scene.compute_static_normals();

        let center = (min + max) * 0.5;
        let extent = (max - min).map(|value| value.max(0.1));
        let mut camera = <rusterix::D3IsoCamera as rusterix::D3Camera>::new();
        camera.center = center;
        camera.azimuth_deg = 135.0;
        camera.elevation_deg = 34.0;
        camera.height_clearance = 0.0;
        camera.distance = extent.magnitude().max(4.0);
        camera.scale = (extent.x.max(extent.y).max(extent.z) * 0.78).max(1.35);
        camera.near = 0.1;
        camera.far = 80.0;

        let (_forward, _right, up) = camera.basis_vectors();
        let light_pos = camera.position() + up * extent.y.max(1.0) * 1.5;
        scene.lights.push(
            rusterix::Light::new(rusterix::LightType::Point)
                .with_position(light_pos)
                .with_color([0.98, 0.96, 0.92])
                .with_intensity(0.46)
                .with_start_distance(0.0)
                .with_end_distance(extent.magnitude().max(6.0) * 3.0)
                .compile(),
        );

        let mut pixels = vec![0_u8; render_width * render_height * 4];
        let mut rasterizer = rusterix::Rasterizer::setup(
            None,
            camera.view_matrix(),
            camera.projection_matrix(render_width as f32, render_height as f32),
        )
        .render_mode(rusterix::RenderMode::render_3d())
        .background([0, 0, 0, 0])
        .ambient(Vec4::new(0.38, 0.40, 0.43, 1.0));
        rasterizer.preserve_transparency = true;
        rasterizer.rasterize(
            &mut scene,
            &mut pixels,
            render_width,
            render_height,
            64,
            &rusterix::Assets::default(),
        );

        let pixels = Self::downsample_rgba_box(&pixels, render_width, render_height, SSAA);
        TheRGBABuffer::from(pixels, width as u32, height as u32)
    }

    fn render_prop_preview(asset: &rusterix::BlockPropAsset, size: i32) -> TheRGBABuffer {
        let mut catalog = IndexMap::default();
        catalog.insert(asset.id, asset.clone());
        let instance = rusterix::BlockPropInstance::new(asset.id);
        let resolution = rusterix::resolve_block_prop_geometry(&[instance], &catalog);
        Self::render_objects_preview(
            &resolution.geometry_objects,
            size,
            &rusterix::Assets::default(),
        )
    }
    fn render_objects_preview(
        objects: &[rusterix::GeometryObject],
        size: i32,
        assets: &rusterix::Assets,
    ) -> TheRGBABuffer {
        Self::render_scene_preview(objects, size, size, assets)
    }
    fn render_scene_preview(
        objects: &[rusterix::GeometryObject],
        width: i32,
        height: i32,
        assets: &rusterix::Assets,
    ) -> TheRGBABuffer {
        const SSAA: usize = 2;
        let width = width.max(32) as usize;
        let height = height.max(32) as usize;
        let render_width = width * SSAA;
        let render_height = height * SSAA;
        let mut scene = rusterix::Scene::empty();
        let mut min = Vec3::broadcast(f32::INFINITY);
        let mut max = Vec3::broadcast(f32::NEG_INFINITY);

        for object in objects {
            let vertices = object
                .vertices
                .iter()
                .map(|vertex| {
                    let world = object.transform_point(*vertex);
                    min.x = min.x.min(world.x);
                    min.y = min.y.min(world.y);
                    min.z = min.z.min(world.z);
                    max.x = max.x.max(world.x);
                    max.y = max.y.max(world.y);
                    max.z = max.z.max(world.z);
                    [world.x, world.y, world.z, 1.0]
                })
                .collect::<Vec<_>>();
            for face in &object.faces {
                if face.indices.len() < 3 {
                    continue;
                }
                let vertices = face
                    .indices
                    .iter()
                    .filter_map(|i| vertices.get(*i).copied())
                    .collect::<Vec<_>>();
                if vertices.len() != face.indices.len() {
                    continue;
                }
                let indices = (1..vertices.len() - 1).map(|i| (0, i, i + 1)).collect();
                let uvs = if face.uvs.len() == vertices.len() {
                    face.uvs.iter().map(|uv| [uv.x, uv.y]).collect()
                } else {
                    vec![[0., 0.]; vertices.len()]
                };
                let source = face
                    .tile
                    .as_ref()
                    .filter(|source| match source {
                        rusterix::PixelSource::TileId(id) => assets.tiles.contains_key(id),
                        rusterix::PixelSource::Pixel(_) | rusterix::PixelSource::Color(_) => true,
                        _ => false,
                    })
                    .cloned()
                    .unwrap_or_else(|| {
                        rusterix::PixelSource::Pixel(prefab_object_default_color(object))
                    });
                let source = match source {
                    rusterix::PixelSource::TileId(id) => {
                        rusterix::PixelSource::StaticTileIndex(assets.tile_indices[&id])
                    }
                    rusterix::PixelSource::Color(color) => {
                        rusterix::PixelSource::Pixel(color.to_u8_array())
                    }
                    source => source,
                };
                let mut batch = rusterix::Batch3D::new(vertices, indices, uvs)
                    .source(source)
                    .cull_mode(rusterix::CullMode::Off);
                batch.ambient_color = Vec3::new(0.42, 0.44, 0.48);
                scene.d3_static.push(batch);
            }
        }

        if scene.d3_static.is_empty() {
            return TheRGBABuffer::new(TheDim::sized(width as i32, height as i32));
        }
        scene.compute_static_normals();
        let center = (min + max) * 0.5;
        let extent = (max - min).map(|value| value.max(0.1));
        let mut camera = <rusterix::D3IsoCamera as rusterix::D3Camera>::new();
        camera.center = center;
        camera.azimuth_deg = 135.0;
        camera.elevation_deg = 34.0;
        camera.height_clearance = 0.0;
        camera.distance = extent.magnitude().max(4.0);
        camera.scale = (extent.x.max(extent.y).max(extent.z) * 0.78).max(1.35);
        camera.near = 0.1;
        camera.far = 80.0;

        let mut pixels = vec![0_u8; render_width * render_height * 4];
        let mut rasterizer = rusterix::Rasterizer::setup(
            None,
            camera.view_matrix(),
            camera.projection_matrix(render_width as f32, render_height as f32),
        )
        .render_mode(rusterix::RenderMode::render_3d())
        .background([0, 0, 0, 0])
        .ambient(Vec4::new(0.38, 0.40, 0.43, 1.0));
        rasterizer.preserve_transparency = true;
        rasterizer.rasterize(
            &mut scene,
            &mut pixels,
            render_width,
            render_height,
            64,
            assets,
        );
        let pixels = Self::downsample_rgba_box(&pixels, render_width, render_height, SSAA);
        TheRGBABuffer::from(pixels, width as u32, height as u32)
    }

    fn cylinder_batch(min: Vec3<f32>, max: Vec3<f32>) -> rusterix::Batch3D {
        let (vertices, faces) = cylinder_vertices_and_faces(min, max, BLOCK_COLUMN_SEGMENTS);
        let mut batch_vertices = vertices
            .into_iter()
            .map(|vertex| [vertex.x, vertex.y, vertex.z, 1.0])
            .collect::<Vec<_>>();
        let mut uvs = vec![[0.0, 0.0]; batch_vertices.len()];
        let mut indices = Vec::new();

        for face in faces {
            if face.len() < 3 {
                continue;
            }
            let first = face[0];
            for pair in face[1..].windows(2) {
                indices.push((first, pair[0], pair[1]));
            }
        }

        if batch_vertices.is_empty() {
            batch_vertices.push([min.x, min.y, min.z, 1.0]);
            uvs.push([0.0, 0.0]);
        }

        rusterix::Batch3D::new(batch_vertices, indices, uvs)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockCatalogTab {
    Blocks,
    Furniture,
    Decoration,
    Lighting,
    User,
    Meshes,
}

impl BlockCatalogTab {
    const ALL: [Self; 6] = [
        Self::Blocks,
        Self::Furniture,
        Self::Decoration,
        Self::Lighting,
        Self::User,
        Self::Meshes,
    ];

    fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::Blocks)
    }

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0)
    }

    fn label(self) -> String {
        match self {
            Self::Blocks => fl!("props_blocks"),
            Self::Furniture => fl!("props_furniture"),
            Self::Decoration => fl!("props_decoration"),
            Self::Lighting => fl!("props_lighting"),
            Self::User => fl!("props_project"),
            Self::Meshes => fl!("props_meshes"),
        }
    }
}

pub struct BlocksDock {
    selected: Uuid,
    active_tab: BlockCatalogTab,
    project_assets: Vec<rusterix::BlockPropAsset>,
    bundled_overrides: HashMap<Uuid, rusterix::BlockPropAsset>,
    preview_cache: HashMap<(Uuid, i32), TheRGBABuffer>,
    mesh_assets: Vec<(Uuid, String, String)>,
    mesh_revision: Option<(u64, u64, ThePalette, String)>,
    mesh_previews: HashMap<Uuid, TheRGBABuffer>,
    mesh_templates: HashMap<Uuid, rusterix::GeometryObject>,
    mesh_textures: HashMap<Uuid, IndexMap<Uuid, rusterix::Tile>>,
    mesh_worker: Option<Mutex<mpsc::Receiver<MeshPreviewResult>>>,
    mesh_cancel: Arc<AtomicU64>,
    mesh_requests: Option<mpsc::Sender<Vec<Uuid>>>,
    mesh_requested: std::collections::HashSet<Uuid>,
    mesh_requested_selection: Option<Uuid>,
}

impl BlocksDock {
    const PREVIEW_SIZE: i32 = 72;

    fn board_name(tab: BlockCatalogTab) -> String {
        format!("{BLOCKS_DOCK_BOARD} {}", tab.index())
    }

    fn tab_from_board_name(name: &str) -> Option<BlockCatalogTab> {
        let index = name.strip_prefix(&format!("{BLOCKS_DOCK_BOARD} "))?;
        index
            .parse::<usize>()
            .ok()
            .filter(|index| *index < BlockCatalogTab::ALL.len())
            .map(BlockCatalogTab::from_index)
    }

    fn block_asset_tab(asset: &BlockAsset) -> BlockCatalogTab {
        if asset.name == "Table" {
            BlockCatalogTab::Furniture
        } else {
            BlockCatalogTab::Blocks
        }
    }

    fn bundled_asset_tab(asset: &rusterix::BlockPropAsset) -> BlockCatalogTab {
        match asset.category.as_str() {
            "Furniture" => BlockCatalogTab::Furniture,
            "Decoration" => BlockCatalogTab::Decoration,
            "Lighting" => BlockCatalogTab::Lighting,
            _ => BlockCatalogTab::Decoration,
        }
    }

    fn catalog_ids(&self, tab: BlockCatalogTab) -> Vec<Uuid> {
        let built_in_blocks = block_assets()
            .iter()
            .filter(|asset| Self::block_asset_tab(asset) == tab)
            .map(|asset| asset.id);
        let bundled = bundled_prefabs()
            .iter()
            .filter(|asset| Self::bundled_asset_tab(asset) == tab)
            .map(|asset| asset.id);
        let user = self
            .project_assets
            .iter()
            .filter(move |_| tab == BlockCatalogTab::User)
            .map(|asset| asset.id);
        let meshes = self
            .mesh_assets
            .iter()
            .filter(move |_| tab == BlockCatalogTab::Meshes)
            .map(|a| a.0);
        built_in_blocks
            .chain(bundled)
            .chain(user)
            .chain(meshes)
            .collect()
    }

    fn selected_index(&self, tab: BlockCatalogTab) -> Option<usize> {
        self.catalog_ids(tab)
            .iter()
            .position(|asset_id| *asset_id == self.selected)
    }

    fn sync_mesh_previews(&mut self, project: &Project) {
        let revision = (
            project.procedural_appearances.revision(),
            project.procedural_materials.revision(),
            project.palette.clone(),
            project.config.clone(),
        );
        if self.mesh_revision.as_ref() == Some(&revision) {
            return;
        }
        self.mesh_revision = Some(revision);
        let previous = std::mem::take(&mut self.mesh_templates);
        let previous_textures = std::mem::take(&mut self.mesh_textures);
        let previous_icons = self.mesh_previews.clone();
        let generation = self.mesh_cancel.fetch_add(1, Ordering::Relaxed) + 1;
        let cancel = self.mesh_cancel.clone();
        let mut host = crate::tools::props_recipe::preview_snapshot(project);
        let (send, receive) = mpsc::sync_channel(2);
        let (requests, jobs) = mpsc::channel::<Vec<Uuid>>();
        self.mesh_worker = Some(Mutex::new(receive));
        self.mesh_requests = Some(requests);
        self.mesh_requested.clear();
        self.mesh_requested_selection = None;
        std::thread::spawn(move || {
            // Preview baking must not monopolize the editor's global Rayon pool.
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(2)
                .build()
                .unwrap();
            for textures in previous_textures.into_values() {
                host.tiles.extend(textures);
            }
            let entries = crate::tools::props_recipe::candidates(&host);
            let ids = entries
                .iter()
                .map(|a| a.0)
                .collect::<std::collections::HashSet<_>>();
            if cancel.load(Ordering::Relaxed) != generation
                || send.send(MeshPreviewResult::Catalog(entries)).is_err()
            {
                return;
            }
            // Only visible rows and explicit selections get evaluated. Names can
            // be scrolled immediately, even while an expensive recipe is baking.
            let mut pending = std::collections::VecDeque::new();
            let mut completed = std::collections::HashSet::new();
            loop {
                if pending.is_empty() {
                    let Ok(batch) = jobs.recv() else {
                        return;
                    };
                    pending = batch.into();
                }
                // A scroll/selection takes priority before the next recipe.
                for newer in jobs.try_iter() {
                    pending = newer.into();
                }
                if let Some(id) = pending.pop_front() {
                    if !completed.insert(id) {
                        continue;
                    }
                    if cancel.load(Ordering::Relaxed) != generation {
                        return;
                    }
                    if !ids.contains(&id) {
                        continue;
                    }
                    let result = pool.install(|| {
                        crate::tools::props_recipe::template(&mut host, id, previous.get(&id))
                    });
                    let result = if let Ok(object) = result
                        && !object.faces.is_empty()
                    {
                        // Clone each material once, not once per triangle.
                        let textures = object
                            .faces
                            .iter()
                            .filter_map(|f| match f.tile.as_ref() {
                                Some(rusterix::PixelSource::TileId(id)) => Some(*id),
                                _ => None,
                            })
                            .collect::<std::collections::HashSet<_>>()
                            .into_iter()
                            .filter_map(|id| host.tiles.get(&id).map(|t| (id, t.clone())))
                            .collect::<IndexMap<_, _>>();
                        let mut assets = rusterix::Assets::default();
                        assets.set_tiles(textures.clone());
                        let icon = if previous.get(&id) == Some(&object)
                            && previous_icons.contains_key(&id)
                        {
                            previous_icons[&id].clone()
                        } else {
                            pool.install(|| {
                                BlocksDockPreviews::render_objects_preview(
                                    std::slice::from_ref(&object),
                                    Self::PREVIEW_SIZE,
                                    &assets,
                                )
                            })
                        };
                        MeshPreviewResult::Preview(id, object, textures, icon)
                    } else {
                        MeshPreviewResult::Rejected(id)
                    };
                    if cancel.load(Ordering::Relaxed) != generation || send.send(result).is_err() {
                        return;
                    }
                }
            }
        });
    }

    fn request_visible_meshes(&mut self, ui: &mut TheUI, server: &ServerContext) {
        let mut wanted = vec![];
        if server.block_recipe_pending
            && let Some(id) = server.curr_block_asset_id
        {
            wanted.push(id);
        }
        if self.active_tab == BlockCatalogTab::Meshes
            && let Some(board) = ui.get_icon_grid_view(&Self::board_name(BlockCatalogTab::Meshes))
        {
            wanted.extend(self.mesh_assets[board.visible_items()].iter().map(|a| a.0));
        }
        wanted.retain(|id| !self.mesh_templates.contains_key(id));
        let next = wanted
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let selected = server
            .block_recipe_pending
            .then_some(server.curr_block_asset_id)
            .flatten();
        if next != self.mesh_requested || selected != self.mesh_requested_selection {
            self.mesh_requested_selection = selected;
            self.mesh_requested = next;
            if let Some(requests) = &self.mesh_requests {
                let _ = requests.send(wanted);
            }
        }
    }

    fn install_mesh_textures(
        project: &mut Project,
        server: &ServerContext,
        textures: &IndexMap<Uuid, rusterix::Tile>,
    ) {
        let mut added = false;
        for (id, tile) in textures {
            if !project.tiles.contains_key(id) {
                project.tiles.insert(*id, tile.clone());
                added = true;
            }
        }
        if added {
            crate::utils::editor_scene_refresh_material_textures(project, server);
        }
    }

    fn sync_project_assets(&mut self, project: &Project) {
        self.sync_mesh_previews(project);
        let bundled_overrides = project
            .block_props
            .iter()
            .filter(|(id, _)| bundled_prefab(**id).is_some())
            .map(|(id, asset)| (*id, asset.clone()))
            .collect::<HashMap<_, _>>();
        let assets = project
            .block_props
            .values()
            .filter(|asset| bundled_prefab(asset.id).is_none())
            .cloned()
            .collect::<Vec<_>>();
        if self.project_assets != assets || self.bundled_overrides != bundled_overrides {
            self.project_assets = assets;
            self.bundled_overrides = bundled_overrides;
            self.preview_cache.clear();
        }
    }

    fn icon_items(&mut self, tab: BlockCatalogTab) -> Vec<TheIconGridItem> {
        let asset_ids = self.catalog_ids(tab);
        let mut items = Vec::with_capacity(asset_ids.len());
        for asset_id in asset_ids {
            if let Some((id, label, _)) = self.mesh_assets.iter().find(|a| a.0 == asset_id) {
                items.push(TheIconGridItem {
                    label: label.clone(),
                    status: fl!("props_mesh_hint"),
                    icon: self.mesh_previews.get(id).cloned(),
                });
            } else if let Some(asset) = block_asset(asset_id) {
                let icon = self
                    .preview_cache
                    .entry((asset.id, Self::PREVIEW_SIZE))
                    .or_insert_with(|| {
                        BlocksDockPreviews::render_asset_preview(asset, Self::PREVIEW_SIZE)
                    })
                    .clone();
                items.push(TheIconGridItem {
                    label: localized_block_asset_name(asset),
                    status: format!(
                        "{}: {}",
                        localized_block_asset_name(asset),
                        localized_block_asset_description(asset)
                    ),
                    icon: Some(icon),
                });
            } else if let Some(asset) = bundled_prefab(asset_id) {
                let displayed = self.bundled_overrides.get(&asset.id).unwrap_or(asset);
                let state = match self.bundled_overrides.get(&asset.id) {
                    Some(project_asset) if project_asset != asset => "Modified",
                    Some(_) => "Bundled Copy",
                    None => "Bundled",
                };
                let icon = self
                    .preview_cache
                    .entry((asset.id, Self::PREVIEW_SIZE))
                    .or_insert_with(|| {
                        BlocksDockPreviews::render_prop_preview(displayed, Self::PREVIEW_SIZE)
                    })
                    .clone();
                items.push(TheIconGridItem {
                    label: format!("[{state}] {}", displayed.name),
                    status: format!("{} · {} · editable linked Prefab", displayed.name, state),
                    icon: Some(icon),
                });
            } else if let Some(asset) = self
                .project_assets
                .iter()
                .find(|asset| asset.id == asset_id)
            {
                let icon = self
                    .preview_cache
                    .entry((asset.id, Self::PREVIEW_SIZE))
                    .or_insert_with(|| {
                        BlocksDockPreviews::render_prop_preview(asset, Self::PREVIEW_SIZE)
                    })
                    .clone();
                items.push(TheIconGridItem {
                    label: asset.name.clone(),
                    status: fl!(
                        "prefab_catalog_hover",
                        name = asset.name.as_str(),
                        part_count = asset.parts.len()
                    ),
                    icon: Some(icon),
                });
            }
        }

        items
    }

    fn ensure_selection(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let selected = server_ctx.curr_block_asset_id.unwrap_or(self.selected);
        if block_asset(selected).is_some()
            || bundled_prefab(selected).is_some()
            || project.block_props.contains_key(&selected)
            || project.procedural_appearances.contains_key(&selected)
        {
            self.selected = selected;
        } else {
            self.selected = default_block_asset_id();
        }
        server_ctx.curr_block_asset_id = Some(self.selected);
        server_ctx.curr_block_asset_name = block_asset(self.selected)
            .map(|asset| asset.name.to_string())
            .or_else(|| bundled_prefab(self.selected).map(|asset| asset.name.clone()))
            .or_else(|| {
                project
                    .block_props
                    .get(&self.selected)
                    .map(|asset| asset.name.clone())
            })
            .or_else(|| {
                self.mesh_assets
                    .iter()
                    .find(|a| a.0 == self.selected)
                    .map(|a| a.1.clone())
            });
    }

    fn sync_widgets(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.sync_project_assets(project);
        self.ensure_selection(project, server_ctx);

        for tab in BlockCatalogTab::ALL {
            let selected = self.selected_index(tab);
            let items = self.icon_items(tab);
            if let Some(board) = ui.get_icon_grid_view(&Self::board_name(tab)) {
                board.set_items(items);
                if board.selected() != selected {
                    board.set_selected(selected);
                }
            }
        }
        let tab_layout_name = BLOCKS_DOCK_TABS.to_string();
        if let Some(layout) = ui.canvas.get_layout(Some(&tab_layout_name), None)
            && let Some(tab_layout) = layout.as_tab_layout()
        {
            tab_layout.set_index(self.active_tab.index());
        }
        ctx.ui.redraw_all = true;
    }

    fn update_overlay(ctx: &mut TheContext) {
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Geometry Overlay 3D"),
            TheValue::Empty,
        ));
    }

    fn delete_selected_project_prefab(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let asset_id = self.selected;
        if !project.block_props.contains_key(&asset_id) {
            return false;
        }

        let before = project.clone();
        match crate::block_props::delete_unused_block_prop(project, asset_id) {
            Ok(name) => {
                self.preview_cache
                    .retain(|(cached_id, _), _| *cached_id != asset_id);
                self.selected = default_block_asset_id();
                self.active_tab = BlockCatalogTab::Blocks;
                server_ctx.curr_block_asset_id = Some(self.selected);
                server_ctx.curr_block_asset_name =
                    block_asset(self.selected).map(|asset| asset.name.to_string());

                RUSTERIX
                    .write()
                    .unwrap()
                    .set_block_props(project.block_props.clone());
                SCENEMANAGER
                    .write()
                    .unwrap()
                    .set_block_props(project.block_props.clone());
                crate::utils::editor_scene_refresh_prefab_assets(&before, project, server_ctx);
                UNDOMANAGER.write().unwrap().add_undo(
                    ProjectUndoAtom::ProjectEdit(
                        fl!("prefab_delete"),
                        Box::new(before),
                        Box::new(project.clone()),
                    ),
                    ctx,
                );

                self.sync_widgets(ui, ctx, project, server_ctx);
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_prefab_deleted", name = name.as_str()),
                ));
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Action List"),
                    TheValue::Empty,
                ));
                Self::update_overlay(ctx);
                true
            }
            Err(message) => {
                ctx.ui
                    .send(TheEvent::SetStatusText(TheId::empty(), message));
                true
            }
        }
    }

    fn duplicate_name(project: &Project, source_name: &str) -> String {
        let first = fl!("prefab_duplicate_name", name = source_name);
        if project
            .block_props
            .values()
            .all(|asset| asset.name != first)
        {
            return first;
        }
        let mut number = 2;
        loop {
            let candidate = fl!(
                "prefab_duplicate_name_numbered",
                name = source_name,
                number = number
            );
            if project
                .block_props
                .values()
                .all(|asset| asset.name != candidate)
            {
                return candidate;
            }
            number += 1;
        }
    }

    fn duplicate_selected_prefab(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let source = project
            .block_props
            .get(&self.selected)
            .cloned()
            .or_else(|| bundled_prefab(self.selected).cloned())
            .or_else(|| {
                block_asset(self.selected)
                    .map(|asset| crate::blocks::editable_prefab_from_block_asset(asset))
            });
        let Some(source) = source else {
            return false;
        };

        let before = project.clone();
        let duplicate_name = Self::duplicate_name(project, &source.name);
        let duplicate_id = crate::block_props::duplicate_block_prop_asset(
            project,
            &source,
            duplicate_name.clone(),
        );
        if ensure_prefab_default_surfaces(project, duplicate_id) {
            crate::undo::project_helper::refresh_palette_runtime(project);
        }
        self.selected = duplicate_id;
        self.active_tab = BlockCatalogTab::User;
        server_ctx.curr_block_asset_id = Some(duplicate_id);
        server_ctx.curr_block_asset_name = Some(duplicate_name.clone());

        RUSTERIX
            .write()
            .unwrap()
            .set_block_props(project.block_props.clone());
        SCENEMANAGER
            .write()
            .unwrap()
            .set_block_props(project.block_props.clone());
        crate::utils::editor_scene_refresh_prefab_assets(&before, project, server_ctx);
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::ProjectEdit(
                fl!("prefab_duplicate"),
                Box::new(before),
                Box::new(project.clone()),
            ),
            ctx,
        );

        self.sync_widgets(ui, ctx, project, server_ctx);
        ctx.ui.send(TheEvent::SetStatusText(
            TheId::empty(),
            fl!("status_prefab_duplicated", name = duplicate_name.as_str()),
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));
        Self::update_overlay(ctx);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_browser_routes_real_wheel_input_while_previews_are_stalled() {
        let mut project = Project::default();
        let mut server = ServerContext::default();
        let mut dock = BlocksDock::new();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(440, 240, 1.);
        ui.init(&mut ctx);
        ui.canvas.root = true;
        ui.canvas.set_center(dock.setup(&mut ctx));
        ui.canvas.set_dim(TheDim::sized(440, 240), &mut ctx);
        ui.canvas.layout(440, 240, &mut ctx);
        dock.active_tab = BlockCatalogTab::Meshes;
        ui.get_layout(BLOCKS_DOCK_TABS)
            .unwrap()
            .as_tab_layout()
            .unwrap()
            .set_index(5);
        dock.mesh_revision = Some((
            project.procedural_appearances.revision(),
            project.procedural_materials.revision(),
            project.palette.clone(),
            project.config.clone(),
        ));
        let (send, receive) = mpsc::sync_channel(2);
        let (requests, jobs) = mpsc::channel();
        dock.mesh_worker = Some(Mutex::new(receive));
        dock.mesh_requests = Some(requests);
        let ids = (0..120).map(|_| Uuid::new_v4()).collect::<Vec<_>>();
        send.send(MeshPreviewResult::Catalog(
            ids.iter()
                .enumerate()
                .map(|(i, id)| (*id, format!("Mesh {i:03}"), String::new()))
                .collect(),
        ))
        .unwrap();
        dock.poll_background(&mut ui, &mut ctx, &mut project, &mut server);
        let first = jobs.try_recv().unwrap();
        assert!(
            first.len() < ids.len() / 2,
            "do not enqueue the complete project"
        );
        let mut frame = vec![0; 440 * 240 * 4];
        ui.draw(&mut frame, &mut ctx);
        let name = BlocksDock::board_name(BlockCatalogTab::Meshes);
        let dim = *ui.get_widget(&name).unwrap().dim();
        ui.hover((dim.x + 40) as f32, (dim.y + 40) as f32, &mut ctx);
        assert!(
            ui.mouse_wheel((0, -240), &mut ctx),
            "real input must reach the active tab"
        );
        let scrolled = ui.get_icon_grid_view(&name).unwrap().visible_items();
        assert!(scrolled.start > 0);
        dock.poll_background(&mut ui, &mut ctx, &mut project, &mut server);
        let requested = jobs.try_recv().unwrap();
        assert!(requested.contains(&ids[scrolled.start]));
        assert!(
            !requested.contains(&ids[0]),
            "discard off-screen work on scroll"
        );
        let selected = ids[scrolled.end - 1];
        project.procedural_appearances.insert(
            selected,
            include_str!("../../embedded/meshes/recipes/fence.mzgeometry.json").into(),
        );
        dock.mesh_revision = Some((
            project.procedural_appearances.revision(),
            project.procedural_materials.revision(),
            project.palette.clone(),
            project.config.clone(),
        ));
        dock.handle_event(
            &TheEvent::IndexChanged(TheId::named(&name), scrolled.end - 1),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert_eq!(server.curr_block_asset_id, Some(selected));
        assert!(server.block_recipe_pending);
        assert!(
            server.block_recipe_template.is_none(),
            "selection must not evaluate on the UI thread"
        );
        crate::tools::props_recipe::prepare(&mut project, &mut server).unwrap();
        assert!(server.block_recipe_template.is_none());
        dock.poll_background(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(jobs.try_recv().unwrap()[0], selected);
        server.block_recipe_pending = false;
        let id = ids[scrolled.start];
        send.send(MeshPreviewResult::Preview(
            id,
            rusterix::GeometryObject::new("Mesh"),
            IndexMap::new(),
            TheRGBABuffer::new(TheDim::sized(72, 72)),
        ))
        .unwrap();
        dock.poll_background(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(
            ui.get_icon_grid_view(&name).unwrap().visible_items(),
            scrolled
        );
        ui.draw(&mut frame, &mut ctx);
        assert!(ui.mouse_wheel((0, 120), &mut ctx));
        assert!(ui.get_icon_grid_view(&name).unwrap().visible_items().start < scrolled.start);
        dock.reset_for_project_switch();
    }

    #[test]
    #[ignore = "native GPU atlas inspection"]
    fn three_d_atlas_skips_iso_sprite_sheets_and_keeps_shared_materials() {
        let mut runtime = rusterix::Rusterix::new_without_audio();
        let sprite_id = Uuid::new_v4();
        let ground_id = Uuid::new_v4();
        let bitmap_id = Uuid::new_v4();
        let mut sprite =
            rusterix::Tile::from_texture(rusterix::Texture::from_color([120, 130, 140, 255]));
        sprite.id = sprite_id;
        sprite.isometric = Some(rusterix::isometric::SpriteLayout::default());
        let mut ground = sprite.clone();
        ground.id = ground_id;
        ground.isometric.as_mut().unwrap().ground = true;
        let mut bitmap = ground.clone();
        bitmap.id = bitmap_id;
        bitmap.isometric.as_mut().unwrap().bitmap = true;
        runtime.refresh_tile_textures_cpu(IndexMap::from([
            (sprite_id, sprite),
            (ground_id, ground),
            (bitmap_id, bitmap),
        ]));
        let map = Map::default();
        let mut pixels = vec![0; 64 * 64 * 4];
        runtime.draw_d3(&map, &mut pixels, 64, 64);
        let vm = runtime.scene_handler.vm.active_vm();
        assert!(vm.get_tile_data(sprite_id).is_none());
        assert!(vm.get_tile_data(bitmap_id).is_none());
        assert!(vm.get_tile_data(ground_id).is_some());
        assert!(runtime.assets.tiles.contains_key(&sprite_id));
        runtime.draw_d2(&map, &mut pixels, 64, 64);
        assert!(
            runtime
                .scene_handler
                .vm
                .active_vm()
                .get_tile_data(sprite_id)
                .is_some()
        );
        assert!(
            runtime
                .scene_handler
                .vm
                .active_vm()
                .get_tile_data(bitmap_id)
                .is_some()
        );
    }

    #[test]
    fn prefab_catalog_separates_built_in_categories_from_user_assets() {
        let mut dock = <BlocksDock as Dock>::new();
        let user = rusterix::BlockPropAsset::new("My Prefab");
        let user_id = user.id;
        dock.project_assets.push(user);

        let block_ids = dock.catalog_ids(BlockCatalogTab::Blocks);
        let furniture_ids = dock.catalog_ids(BlockCatalogTab::Furniture);
        let decoration_ids = dock.catalog_ids(BlockCatalogTab::Decoration);
        let lighting_ids = dock.catalog_ids(BlockCatalogTab::Lighting);
        let user_ids = dock.catalog_ids(BlockCatalogTab::User);
        let table_id = block_assets()
            .iter()
            .find(|asset| asset.name == "Table")
            .expect("Table block asset")
            .id;
        let plate_id = bundled_prefabs()
            .iter()
            .find(|asset| asset.name == "Ceramic Plate")
            .expect("Ceramic Plate decoration")
            .id;
        let torch_id = bundled_prefabs()
            .iter()
            .find(|asset| asset.name == "Wall Torch")
            .expect("Wall Torch lighting fixture")
            .id;

        assert!(!block_ids.contains(&table_id));
        assert!(furniture_ids.contains(&table_id));
        assert!(decoration_ids.contains(&plate_id));
        assert!(lighting_ids.contains(&torch_id));
        assert_eq!(user_ids, vec![user_id]);
    }
    #[test]
    #[ignore = "native Props UI review; generated project and screenshots stay ignored"]
    fn capture_3d_props_overlay_and_recipe_placement() {
        use crate::tools::MapEvent;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let out = root.join("generated/props");
        std::fs::create_dir_all(&out).unwrap();
        let mut project = Project::default();
        let id = Uuid::new_v4();
        project.procedural_appearances.insert(
            id,
            include_str!("../../embedded/meshes/recipes/fence.mzgeometry.json").into(),
        );
        let region = &mut project.regions[0];
        region.name = "3D Props Review".into();
        region.map.camera = MapCamera::ThreeDIso;
        let mut floor = rusterix::GeometryObject::box_from_bounds(
            "Floor",
            Vec3::new(-2., -0.2, -2.),
            Vec3::new(10., 0., 7.),
        );
        for f in &mut floor.faces {
            f.tile = Some(rusterix::PixelSource::Pixel([76, 79, 71, 255]));
        }
        let mut wall = rusterix::GeometryObject::box_from_bounds(
            "Mounting wall",
            Vec3::new(0., 0., 0.),
            Vec3::new(5., 3., 0.25),
        );
        for f in &mut wall.faces {
            f.tile = Some(rusterix::PixelSource::Pixel([153, 142, 125, 255]));
        }
        region.map.geometry_objects = vec![floor, wall];
        // Ordinary authored door, using the existing component and pivot machinery.
        let mut leaf = rusterix::GeometryObject::box_from_bounds(
            "Door Leaf",
            Vec3::zero(),
            Vec3::new(1.2, 2.3, 0.12),
        );
        for f in &mut leaf.faces {
            f.tile = Some(rusterix::PixelSource::Pixel([104, 63, 38, 255]));
        }
        let door = rusterix::BlockPropAsset::new_authored("Hinged Door", vec![leaf]);
        let door_id = door.id;
        let part_id = door.parts[0].id;
        project.block_props.insert(door_id, door);
        let component =
            crate::block_props::configure_prefab_door(&mut project, door_id, part_id, 95.).unwrap();
        let mut closed = rusterix::BlockPropInstance::new(door_id);
        closed.world_transform[3] = [5., 0., 0., 1.];
        let mut open = rusterix::BlockPropInstance::new(door_id);
        open.world_transform[3] = [7., 0., 0., 1.];
        rusterix::set_block_prop_door_open(&mut open, component, true);
        assert_eq!(
            rusterix::block_prop_door_is_open(&project.block_props[&door_id], &closed, component),
            Some(false)
        );
        assert_eq!(
            rusterix::block_prop_door_is_open(&project.block_props[&door_id], &open, component),
            Some(true)
        );
        project.regions[0]
            .map
            .block_prop_instances
            .extend([closed, open]);
        let table = bundled_prefabs()
            .iter()
            .find(|a| a.category == "Furniture" && !a.support_surfaces.is_empty())
            .unwrap()
            .clone();
        let torch = bundled_prefabs()
            .iter()
            .find(|a| a.name == "Wall Torch")
            .unwrap()
            .clone();
        let table_id = table.id;
        let torch_id = torch.id;
        assert!(!torch.light_effects.is_empty());
        assert!(!torch.particle_effects.is_empty());
        project.block_props.insert(table_id, table);
        project.block_props.insert(torch_id, torch);
        let mut table = rusterix::BlockPropInstance::new(table_id);
        table.world_transform[3] = [2., 0., 3., 1.];
        let mut torch = rusterix::BlockPropInstance::new(torch_id);
        torch.world_transform[3] = [2., 1., 0.3, 1.];
        project.regions[0]
            .map
            .block_prop_instances
            .extend([table, torch]);
        let mut server = ServerContext::default();
        server.curr_region = project.regions[0].id;
        server.pc = ProjectContext::Region(server.curr_region);
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(1440, 960, 1.);
        ui.set_theme(Box::new(TheBrownTheme::new()), &mut ctx);
        crate::hud::set_theme(&TheBrownTheme::new());
        ui.init(&mut ctx);
        let map_canvas =
            crate::mapeditor::MapEditor::new().init_ui(&mut ui, &mut ctx, &mut project);
        let mut dock = BlocksDock::new();
        let bottom = dock.setup(&mut ctx);
        let mut split = TheSharedVLayout::new(TheId::named("Props Review Split"));
        split.add_canvas(map_canvas);
        split.add_canvas(bottom);
        split.set_shared_ratio(0.75);
        split.set_mode(TheSharedVLayoutMode::Shared);
        let mut center = TheCanvas::new();
        center.set_layout(split);
        ui.canvas.set_center(center);
        let mut tools = crate::toollist::ToolList::new();
        let props = tools
            .get_game_tool_uuid_by_command_id("tool.blocks")
            .unwrap();
        // The 2D and 2D Iso cameras cannot activate 3D Props.
        assert!(!tools.set_tool(props, &mut ui, &mut ctx, &mut project, &mut server));
        assert!(!server.block_tool_active);
        server.editor_view_mode = EditorViewMode::Orbit;
        assert!(tools.set_tool(props, &mut ui, &mut ctx, &mut project, &mut server));
        dock.activate(&mut ui, &mut ctx, &project, &mut server);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        server.curr_block_asset_id = Some(id);
        server.block_recipe_pending = true;
        while !dock.mesh_templates.contains_key(&id) && std::time::Instant::now() < deadline {
            dock.poll_background(&mut ui, &mut ctx, &mut project, &mut server);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(dock.mesh_templates.contains_key(&id));
        assert!(ui.get_text_layout("Blocks Dock Inspector").is_none());
        assert!(ui.get_widget("Blocks Dock Operation").is_none());
        let index = dock
            .catalog_ids(BlockCatalogTab::Meshes)
            .iter()
            .position(|i| *i == id)
            .unwrap();
        dock.handle_event(
            &TheEvent::IndexChanged(
                TheId::named(&BlocksDock::board_name(BlockCatalogTab::Meshes)),
                index,
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        tools.handle_event(
            &TheEvent::Custom(TheId::named("Props Selection Changed"), TheValue::Empty),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert_eq!(server.curr_block_asset_id, Some(id));
        assert!(server.pc.is_region());
        let mut pixels = vec![0; 1440 * 960 * 4];
        ui.draw(&mut pixels, &mut ctx);
        let mut assets = rusterix::Assets::default();
        assets.set_tiles(project.tiles.clone());
        assets.set_block_props(project.block_props.clone());
        let mut buffer = TheRGBABuffer::new(TheDim::sized(1000, 650));
        tools.get_current_tool().draw_hud(
            &mut buffer,
            &mut project.regions[0].map,
            &mut ctx,
            &mut server,
            &assets,
        );
        let events = ui.add_state_listener("Props Review".into());
        ui.update(&mut ctx);
        for _ in events.try_iter() {}
        // The whole overlay gesture, including a release outside it, is consumed.
        server.hover_cursor_3d = Some(Vec3::new(3., 0., 4.));
        let count = project.regions[0].map.geometry_objects.len();
        let at = Vec2::new(70, 144);
        assert!(
            tools
                .get_current_tool()
                .map_event(
                    MapEvent::MapClicked(at),
                    &mut ui,
                    &mut ctx,
                    &mut project.regions[0].map,
                    &mut server
                )
                .is_none()
        );
        assert!(
            tools
                .get_current_tool()
                .map_event(
                    MapEvent::MapDragged(Vec2::new(650, 450)),
                    &mut ui,
                    &mut ctx,
                    &mut project.regions[0].map,
                    &mut server
                )
                .is_none()
        );
        assert!(
            tools
                .get_current_tool()
                .map_event(
                    MapEvent::MapUp(Vec2::new(650, 450)),
                    &mut ui,
                    &mut ctx,
                    &mut project.regions[0].map,
                    &mut server
                )
                .is_none()
        );
        ui.update(&mut ctx);
        let rotation = events
            .try_iter()
            .filter(|e| matches!(e,TheEvent::ValueChanged(id,_) if id.name=="Props Rotation"))
            .collect::<Vec<_>>();
        assert_eq!(rotation.len(), 1);
        for event in rotation {
            tools.handle_event(&event, &mut ui, &mut ctx, &mut project, &mut server);
        }
        assert_eq!(server.block_rotation_quarters, 1);
        assert_eq!(project.regions[0].map.geometry_objects.len(), count);
        // Map click places linked geometry without opening an editor or adding a Prefab.
        let prefabs = project.block_props.len();
        tools.get_current_tool().map_event(
            MapEvent::MapClicked(Vec2::new(650, 450)),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        let undo = tools.get_current_tool().map_event(
            MapEvent::MapUp(Vec2::new(650, 450)),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        assert!(matches!(undo, Some(ProjectUndoAtom::MapEdit(..))));
        assert_eq!(project.block_props.len(), prefabs);
        assert_eq!(project.regions[0].map.geometry_objects.len(), count + 1);
        let object = project.regions[0].map.geometry_objects.last().unwrap();
        let transform = object.transform;
        assert_eq!(
            shared::geometry_recipe::Binding::read(object)
                .unwrap()
                .asset,
            id
        );
        let doc = denrim_materials::geometry::Document::parse(&project.procedural_appearances[&id])
            .unwrap();
        let controls = crate::tools::props_recipe::published(&doc);
        let index = controls
            .iter()
            .position(|p| p.control.id == "count")
            .unwrap();
        tools.handle_event(
            &TheEvent::ValueChanged(
                TheId::named(&format!("Props Recipe {id} {index}")),
                TheValue::Int(7),
            ),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        assert_eq!(
            project.regions[0]
                .map
                .geometry_objects
                .last()
                .unwrap()
                .transform,
            transform
        );
        let changed =
            denrim_materials::geometry::Document::parse(&project.procedural_appearances[&id])
                .unwrap();
        assert_eq!(
            crate::tools::props_recipe::published(&changed)[index]
                .control
                .value,
            7.
        );

        std::fs::write(
            out.join("Props3D.eldiron"),
            shared::project_io::encode_project(&project).unwrap(),
        )
        .unwrap();
        // Render the actual native geometry and mount the actual browser and overlay.
        let mut scene = project.regions[0].map.geometry_objects.clone();
        scene.extend(
            rusterix::resolve_block_prop_geometry(
                &project.regions[0].map.block_prop_instances,
                &project.block_props,
            )
            .geometry_objects,
        );
        assets.set_tiles(project.tiles.clone());
        let view = ui.get_render_view("PolyView").unwrap();
        let dim = *view.dim();
        *view.render_buffer_mut() =
            BlocksDockPreviews::render_scene_preview(&scene, dim.width, dim.height, &assets);
        tools.draw_hud(
            view.render_buffer_mut(),
            &mut project.regions[0].map,
            &mut ctx,
            &mut server,
            &assets,
        );
        ui.draw(&mut pixels, &mut ctx);
        std::fs::write(
            out.join("props-overlay.png"),
            TheRGBABuffer::from(pixels.clone(), 1440, 960)
                .to_png()
                .unwrap(),
        )
        .unwrap();
        tools.get_current_tool().map_event(
            MapEvent::MapClicked(Vec2::new(300, 178)),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        tools.get_current_tool().map_event(
            MapEvent::MapUp(Vec2::new(300, 178)),
            &mut ui,
            &mut ctx,
            &mut project.regions[0].map,
            &mut server,
        );
        ui.draw(&mut pixels, &mut ctx);
        assert!(
            ui.get_widget(&format!("Props Recipe {id} {index}"))
                .is_some()
        );
        std::fs::write(
            out.join("props-controls.png"),
            TheRGBABuffer::from(pixels, 1440, 960).to_png().unwrap(),
        )
        .unwrap();
    }
}

impl Dock for BlocksDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            selected: default_block_asset_id(),
            active_tab: BlockCatalogTab::Blocks,
            project_assets: Vec::new(),
            bundled_overrides: HashMap::new(),
            preview_cache: HashMap::new(),
            mesh_assets: vec![],
            mesh_revision: None,
            mesh_previews: HashMap::new(),
            mesh_templates: HashMap::new(),
            mesh_textures: HashMap::new(),
            mesh_worker: None,
            mesh_cancel: Arc::new(AtomicU64::new(0)),
            mesh_requests: None,
            mesh_requested: Default::default(),
            mesh_requested_selection: None,
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut center = TheCanvas::new();

        let mut tabs = TheTabLayout::new(TheId::named(BLOCKS_DOCK_TABS));
        for tab in BlockCatalogTab::ALL {
            let mut board_canvas = TheCanvas::new();
            let mut board = TheIconGridView::new(TheId::named(&Self::board_name(tab)));
            board.set_cell_size(88);
            board.set_icon_size(Self::PREVIEW_SIZE);
            board.set_show_labels(tab == BlockCatalogTab::Meshes);
            board.set_icon_padding(6);
            board.set_spacing(8);
            board.set_content_padding(10);
            board_canvas.set_widget(board);
            tabs.add_canvas(tab.label().to_string(), board_canvas);
        }
        center.set_layout(tabs);

        canvas.set_center(center);
        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.sync_project_assets(project);
        self.ensure_selection(project, server_ctx);
        self.active_tab = if block_asset(self.selected).is_some() {
            block_asset(self.selected)
                .map(Self::block_asset_tab)
                .unwrap_or(BlockCatalogTab::Blocks)
        } else if let Some(asset) = bundled_prefab(self.selected) {
            Self::bundled_asset_tab(asset)
        } else if project.procedural_appearances.contains_key(&self.selected) {
            BlockCatalogTab::Meshes
        } else {
            BlockCatalogTab::User
        };
        if self.active_tab == BlockCatalogTab::Meshes {
            server_ctx.block_recipe_template = self.mesh_templates.get(&self.selected).cloned();
            server_ctx.block_recipe_pending = server_ctx.block_recipe_template.is_none();
        }
        self.sync_widgets(ui, ctx, project, server_ctx);
    }

    fn reset_for_project_switch(&mut self) {
        self.mesh_cancel.fetch_add(1, Ordering::Relaxed);
        self.mesh_worker = None;
        self.mesh_requests = None;
        self.mesh_requested.clear();
        self.mesh_requested_selection = None;
        self.mesh_revision = None;
        self.mesh_assets.clear();
        self.mesh_templates.clear();
        self.mesh_textures.clear();
        self.mesh_previews.clear();
        self.project_assets.clear();
        self.bundled_overrides.clear();
        self.preview_cache.clear();
    }
    fn poll_background(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) -> bool {
        self.sync_mesh_previews(project);
        let mut results = vec![];
        let mut finished = false;
        if let Some(receive) = &self.mesh_worker {
            let receive = receive.lock().unwrap();
            // Bounded delivery keeps a large completed catalog from monopolizing a frame.
            for _ in 0..4 {
                match receive.try_recv() {
                    Ok(result) => results.push(result),
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                }
            }
        }
        if finished {
            self.mesh_worker = None;
        }
        let changed = !results.is_empty();
        for result in results {
            match result {
                MeshPreviewResult::Catalog(entries) => {
                    self.mesh_assets = entries;
                    if server.curr_block_asset_id.is_some_and(|id| {
                        project.procedural_appearances.contains_key(&id)
                            && !self.mesh_assets.iter().any(|a| a.0 == id)
                    }) {
                        self.selected = default_block_asset_id();
                        server.curr_block_asset_id = Some(self.selected);
                        server.block_recipe_pending = false;
                        server.block_recipe_template = None;
                    }
                    self.ensure_selection(project, server);
                    let items = self.icon_items(BlockCatalogTab::Meshes);
                    let selected = self.selected_index(BlockCatalogTab::Meshes);
                    if let Some(board) =
                        ui.get_icon_grid_view(&Self::board_name(BlockCatalogTab::Meshes))
                    {
                        board.set_items(items);
                        if board.selected() != selected {
                            board.set_selected(selected);
                        }
                    }
                }
                MeshPreviewResult::Preview(id, object, textures, icon) => {
                    if let Some(index) = self.mesh_assets.iter().position(|a| a.0 == id)
                        && let Some(board) =
                            ui.get_icon_grid_view(&Self::board_name(BlockCatalogTab::Meshes))
                    {
                        board.set_item_icon(index, Some(icon.clone()));
                    }
                    if server.curr_block_asset_id == Some(id) && server.block_recipe_pending {
                        server.block_recipe_pending = false;
                        server.block_recipe_template = Some(object.clone());
                        Self::install_mesh_textures(project, server, &textures);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Props Selection Changed"),
                            TheValue::Empty,
                        ));
                    }
                    self.mesh_templates.insert(id, object);
                    self.mesh_textures.insert(id, textures);
                    self.mesh_previews.insert(id, icon);
                }
                MeshPreviewResult::Rejected(id) => {
                    self.mesh_assets.retain(|a| a.0 != id);
                    if server.curr_block_asset_id == Some(id) {
                        server.block_recipe_pending = false;
                        server.block_recipe_template = None;
                        self.selected = default_block_asset_id();
                        server.curr_block_asset_id = Some(self.selected);
                        self.ensure_selection(project, server);
                        ctx.ui.send(TheEvent::Custom(
                            TheId::named("Props Selection Changed"),
                            TheValue::Empty,
                        ));
                    }
                    let selected = self.selected_index(BlockCatalogTab::Meshes);
                    let items = self.icon_items(BlockCatalogTab::Meshes);
                    if let Some(board) =
                        ui.get_icon_grid_view(&Self::board_name(BlockCatalogTab::Meshes))
                    {
                        board.set_items(items);
                        if board.selected() != selected {
                            board.set_selected(selected);
                        }
                    }
                }
            }
        }
        self.request_visible_meshes(ui, server);
        if changed {
            ctx.ui.redraw_all = true;
        }
        changed
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::IndexChanged(id, index)
                if id.name == format!("{BLOCKS_DOCK_TABS} Tabbar") =>
            {
                self.active_tab = BlockCatalogTab::from_index(*index);
                self.sync_widgets(ui, ctx, project, server_ctx);
                true
            }
            TheEvent::IndexChanged(id, index) if Self::tab_from_board_name(&id.name).is_some() => {
                let tab = Self::tab_from_board_name(&id.name).unwrap_or(self.active_tab);
                let Some(asset_id) = self.catalog_ids(tab).get(*index).copied() else {
                    return false;
                };
                self.active_tab = tab;
                let palette_before = project.palette.clone();
                let palette_materials_before = project.palette_materials.clone();
                // Bundled Prefabs become project-owned on first use.
                // The stable ID means existing placements and later edits keep
                // referencing the same ordinary serialized Prefab asset.
                let mut prefab_assets_changed = false;
                if !project.block_props.contains_key(&asset_id)
                    && let Some(asset) = bundled_prefab(asset_id)
                {
                    project.block_props.insert(asset_id, asset.clone());
                    prefab_assets_changed = true;
                }
                if project.block_props.contains_key(&asset_id) {
                    prefab_assets_changed |= upgrade_bundled_prefab_geometry(project, asset_id);
                    prefab_assets_changed |= ensure_prefab_default_surfaces(project, asset_id);
                }
                if let Some(asset) = block_asset(asset_id) {
                    prefab_assets_changed |= ensure_block_asset_default_palette(project, asset);
                }
                if prefab_assets_changed {
                    let palette_changed = project.palette != palette_before
                        || project.palette_materials != palette_materials_before;
                    if palette_changed {
                        crate::undo::project_helper::refresh_palette_runtime(project);
                    }
                    RUSTERIX
                        .write()
                        .unwrap()
                        .set_block_props(project.block_props.clone());
                    SCENEMANAGER
                        .write()
                        .unwrap()
                        .sync_block_props(project.block_props.clone());
                }
                self.selected = asset_id;
                server_ctx.curr_block_asset_id = Some(asset_id);
                if let Some(asset) = block_asset(asset_id) {
                    server_ctx.block_height_cells = block_asset_default_sizing(asset).height_cells;
                } else if let Some(asset) = project.block_props.get(&asset_id)
                    && prefab_uses_auto_sizing(asset)
                {
                    server_ctx.block_height_cells = asset.placement.footprint[1].max(1) as f32;
                }
                server_ctx.curr_block_asset_name = block_asset(asset_id)
                    .map(|asset| asset.name.to_string())
                    .or_else(|| bundled_prefab(asset_id).map(|asset| asset.name.clone()))
                    .or_else(|| {
                        project
                            .block_props
                            .get(&asset_id)
                            .map(|asset| asset.name.clone())
                    });
                if let Some((_, label, _)) = self.mesh_assets.iter().find(|a| a.0 == asset_id) {
                    server_ctx.curr_block_asset_name = Some(label.clone());
                }
                server_ctx.block_recipe_pending =
                    tab == BlockCatalogTab::Meshes && !self.mesh_templates.contains_key(&asset_id);
                server_ctx.block_recipe_template = self.mesh_templates.get(&asset_id).cloned();
                if let Some(textures) = self.mesh_textures.get(&asset_id) {
                    Self::install_mesh_textures(project, server_ctx, textures);
                }
                if !server_ctx.block_recipe_pending && server_ctx.block_recipe_template.is_none() {
                    if let Err(error) = crate::tools::props_recipe::prepare(project, server_ctx) {
                        server_ctx.block_recipe_template = None;
                        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                    } else if server_ctx.block_recipe_template.is_some() {
                        crate::utils::editor_scene_refresh_material_textures(project, server_ctx);
                    }
                }
                self.sync_widgets(ui, ctx, project, server_ctx);
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Props Selection Changed"),
                    TheValue::Empty,
                ));
                if let Some(asset) = block_asset(asset_id) {
                    let asset_name = localized_block_asset_name(asset);
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        format!("{}", fl!("status_block_selected", asset_name = asset_name)),
                    ));
                } else if let Some(asset) = project.block_props.get(&asset_id) {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        fl!("status_prefab_selected", name = asset.name.as_str()),
                    ));
                }
                Self::update_overlay(ctx);
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == BLOCKS_DOCK_OPERATION => {
                server_ctx.block_operation =
                    (*index as i32).clamp(BLOCK_OPERATION_PLACE, BLOCK_OPERATION_ERASE);
                self.sync_widgets(ui, ctx, project, server_ctx);
                Self::update_overlay(ctx);
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == BLOCKS_DOCK_STROKE => {
                server_ctx.block_stroke_mode =
                    (*index as i32).clamp(BLOCK_STROKE_LINE, BLOCK_STROKE_RECT);
                self.sync_widgets(ui, ctx, project, server_ctx);
                Self::update_overlay(ctx);
                true
            }
            TheEvent::IndexChanged(id, index) if id.name == BLOCKS_DOCK_DAMAGE => {
                server_ctx.block_damage_enabled = *index == 1;
                self.sync_widgets(ui, ctx, project, server_ctx);
                Self::update_overlay(ctx);
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == BLOCKS_DOCK_DUPLICATE =>
            {
                self.duplicate_selected_prefab(ui, ctx, project, server_ctx)
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == BLOCKS_DOCK_DELETE =>
            {
                self.delete_selected_project_prefab(ui, ctx, project, server_ctx)
            }
            TheEvent::Custom(id, TheValue::Empty) if id.name == BLOCKS_DOCK_SYNC_EVENT => {
                self.sync_widgets(ui, ctx, project, server_ctx);
                true
            }
            _ => false,
        }
    }
}
