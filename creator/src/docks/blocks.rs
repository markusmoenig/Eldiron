use crate::blocks::{
    BLOCK_COLUMN_SEGMENTS, BLOCK_OPERATION_ERASE, BLOCK_OPERATION_PLACE, BLOCK_STROKE_LINE,
    BLOCK_STROKE_RECT, BlockAsset, BlockSizing, adjusted_rotated_bounds, asset_supports_height,
    asset_supports_width, block_asset, block_assets, block_component_kind, component_uses_cylinder,
    cylinder_vertices_and_faces, default_block_asset_id, localized_block_asset_description,
    localized_block_asset_name,
};
use crate::prelude::*;
use rusterix::D3Camera;
use std::collections::HashMap;
use vek::Vec4;

const BLOCKS_DOCK_BOARD: &str = "Blocks Dock Board";
const BLOCKS_DOCK_INSPECTOR: &str = "Blocks Dock Inspector";
const BLOCKS_DOCK_OPERATION: &str = "Blocks Dock Operation";
const BLOCKS_DOCK_STROKE: &str = "Blocks Dock Stroke";
const BLOCKS_DOCK_DAMAGE: &str = "Blocks Dock Damage";
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
                adjusted_rotated_bounds(asset, index, BlockSizing::default(), 0)
            else {
                continue;
            };
            min.x = min.x.min(box_min.x);
            min.y = min.y.min(box_min.y);
            min.z = min.z.min(box_min.z);
            max.x = max.x.max(box_max.x);
            max.y = max.y.max(box_max.y);
            max.z = max.z.max(box_max.z);

            let mut batch = if component_uses_cylinder(block_component_kind(asset, index)) {
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
                BLOCK_PREVIEW_COLORS[index % BLOCK_PREVIEW_COLORS.len()],
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
        const SSAA: usize = 2;

        let width = size.max(32) as usize;
        let height = size.max(32) as usize;
        let render_width = width * SSAA;
        let render_height = height * SSAA;
        let mut catalog = IndexMap::default();
        catalog.insert(asset.id, asset.clone());
        let instance = rusterix::BlockPropInstance::new(asset.id);
        let resolution = rusterix::resolve_block_prop_geometry(&[instance], &catalog);
        let mut scene = rusterix::Scene::empty();
        let mut min = Vec3::broadcast(f32::INFINITY);
        let mut max = Vec3::broadcast(f32::NEG_INFINITY);

        for (object_index, object) in resolution.geometry_objects.iter().enumerate() {
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
            let mut indices = Vec::new();
            for face in &object.faces {
                let Some(first) = face.indices.first().copied() else {
                    continue;
                };
                for edge in face.indices[1..].windows(2) {
                    indices.push((first, edge[0], edge[1]));
                }
            }
            if vertices.is_empty() || indices.is_empty() {
                continue;
            }
            let vertex_count = vertices.len();
            let mut batch =
                rusterix::Batch3D::new(vertices, indices, vec![[0.0, 0.0]; vertex_count])
                    .source(rusterix::PixelSource::Pixel(
                        BLOCK_PREVIEW_COLORS[object_index % BLOCK_PREVIEW_COLORS.len()],
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

pub struct BlocksDock {
    selected: Uuid,
    project_assets: Vec<rusterix::BlockPropAsset>,
    preview_cache: HashMap<(Uuid, i32), TheRGBABuffer>,
}

impl BlocksDock {
    const PREVIEW_SIZE: i32 = 72;

    fn catalog_ids(&self) -> Vec<Uuid> {
        block_assets()
            .iter()
            .map(|asset| asset.id)
            .chain(self.project_assets.iter().map(|asset| asset.id))
            .collect()
    }

    fn selected_index(&self) -> Option<usize> {
        self.catalog_ids()
            .iter()
            .position(|asset_id| *asset_id == self.selected)
    }

    fn sync_project_assets(&mut self, project: &Project) {
        let assets = project.block_props.values().cloned().collect::<Vec<_>>();
        if self.project_assets != assets {
            self.project_assets = assets;
            self.preview_cache.clear();
        }
    }

    fn icon_items(&mut self) -> Vec<TheIconGridItem> {
        let mut items = Vec::with_capacity(block_assets().len() + self.project_assets.len());

        for asset in block_assets() {
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
        }

        for asset in &self.project_assets {
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

        items
    }

    fn ensure_selection(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let selected = server_ctx.curr_block_asset_id.unwrap_or(self.selected);
        if block_asset(selected).is_some() || project.block_props.contains_key(&selected) {
            self.selected = selected;
        } else {
            self.selected = default_block_asset_id();
        }
        server_ctx.curr_block_asset_id = Some(self.selected);
        server_ctx.curr_block_asset_name = block_asset(self.selected)
            .map(|asset| asset.name.to_string())
            .or_else(|| {
                project
                    .block_props
                    .get(&self.selected)
                    .map(|asset| asset.name.clone())
            });
    }

    fn text(value: impl Into<String>) -> Box<dyn TheWidget> {
        let mut text = TheText::new(TheId::empty());
        text.set_text(value.into());
        Box::new(text)
    }

    fn sync_widgets(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.ensure_selection(project, server_ctx);
        self.sync_project_assets(project);

        let selected = self.selected_index();
        let items = self.icon_items();
        if let Some(board) = ui.get_icon_grid_view(BLOCKS_DOCK_BOARD) {
            board.set_items(items);
            board.set_selected(selected);
        }
        if let Some(widget) = ui.get_widget(BLOCKS_DOCK_OPERATION)
            && let Some(group) = widget.as_group_button()
        {
            group.set_index(
                server_ctx
                    .block_operation
                    .clamp(BLOCK_OPERATION_PLACE, BLOCK_OPERATION_ERASE),
            );
        }
        if let Some(widget) = ui.get_widget(BLOCKS_DOCK_STROKE)
            && let Some(group) = widget.as_group_button()
        {
            group.set_index(
                server_ctx
                    .block_stroke_mode
                    .clamp(BLOCK_STROKE_LINE, BLOCK_STROKE_RECT),
            );
        }
        if let Some(widget) = ui.get_widget(BLOCKS_DOCK_DAMAGE)
            && let Some(group) = widget.as_group_button()
        {
            group.set_index(if server_ctx.block_damage_enabled {
                1
            } else {
                0
            });
        }

        if let Some(layout) = ui.get_text_layout(BLOCKS_DOCK_INSPECTOR) {
            layout.clear();
            if let Some(asset) = block_asset(self.selected) {
                let adjusts = match (asset_supports_height(asset), asset_supports_width(asset)) {
                    (true, true) => fl!("block_adjust_height_width"),
                    (true, false) => fl!("block_adjust_height"),
                    (false, true) => fl!("block_adjust_width"),
                    (false, false) => fl!("block_adjust_fixed"),
                };
                layout.add_pair(
                    fl!("block_label_block"),
                    Self::text(localized_block_asset_name(asset)),
                );
                layout.add_pair(
                    fl!("size"),
                    Self::text(format!(
                        "{} x {} x {}, {}",
                        asset.footprint.x, asset.footprint.y, asset.footprint.z, adjusts
                    )),
                );
                layout.add_pair(
                    fl!("block_label_state"),
                    Self::text(format!(
                        "{} {:.2}, {}{}, {}{}, {}",
                        fl!("block_label_cell_short"),
                        server_ctx.block_grid_cell_size.max(0.05),
                        fl!("block_label_level_short"),
                        server_ctx.block_grid_level,
                        fl!("block_label_rotation_short"),
                        server_ctx.block_rotation_quarters.rem_euclid(4) * 90,
                        if server_ctx.block_damage_enabled {
                            fl!("block_damage_damaged")
                        } else {
                            fl!("block_damage_clean")
                        }
                    )),
                );
                layout.add_pair(
                    fl!("block_label_shape"),
                    Self::text(format!(
                        "{}{}, {}+{}",
                        fl!("block_label_height_short"),
                        server_ctx.block_height_cells.max(1),
                        fl!("block_label_width_short"),
                        server_ctx.block_span_extra_cells.max(0)
                    )),
                );
                layout.add_pair(
                    fl!("block_label_mouse"),
                    Self::text(fl!("block_help_mouse")),
                );
                layout.add_pair(fl!("block_label_keys"), Self::text(fl!("block_help_keys")));
                layout.add_pair(
                    fl!("block_label_resize"),
                    Self::text(fl!("block_help_resize")),
                );
            } else if let Some(asset) = project.block_props.get(&self.selected) {
                let object_count = asset
                    .parts
                    .iter()
                    .map(|part| part.geometry_source.geometry_objects().len())
                    .sum::<usize>();
                layout.add_pair(fl!("name"), Self::text(asset.name.clone()));
                layout.add_pair(
                    fl!("prefab_label_source"),
                    Self::text(fl!("prefab_project_source")),
                );
                layout.add_pair(
                    fl!("prefab_label_geometry"),
                    Self::text(fl!(
                        "prefab_geometry_summary",
                        part_count = asset.parts.len(),
                        object_count = object_count
                    )),
                );
                layout.add_pair(
                    fl!("prefab_label_semantics"),
                    Self::text(fl!(
                        "prefab_semantics_summary",
                        surface_count = asset.support_surfaces.len(),
                        interaction_count = asset.interaction_targets.len()
                    )),
                );
                layout.add_pair(
                    fl!("prefab_label_placement"),
                    Self::text(fl!("prefab_linked_placement")),
                );
                layout.add_pair(
                    fl!("prefab_label_shortcut"),
                    Self::text(fl!("prefab_rotate_shortcut")),
                );
            }
            ctx.ui.relayout = true;
        }
    }

    fn update_overlay(ctx: &mut TheContext) {
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Geometry Overlay 3D"),
            TheValue::Empty,
        ));
    }
}

impl Dock for BlocksDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            selected: default_block_asset_id(),
            project_assets: Vec::new(),
            preview_cache: HashMap::new(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar = TheHLayout::new(TheId::named("Blocks Dock Toolbar"));
        toolbar.set_background_color(None);
        toolbar.set_margin(Vec4::new(10, 1, 6, 1));
        toolbar.set_padding(5);

        let mut operation_group = TheGroupButton::new(TheId::named(BLOCKS_DOCK_OPERATION));
        operation_group.add_text_status(fl!("block_operation_place"), fl!("status_block_place"));
        operation_group
            .add_text_status(fl!("block_operation_replace"), fl!("status_block_replace"));
        operation_group.add_text_status(fl!("block_operation_erase"), fl!("status_block_erase"));
        operation_group.set_item_width(70);
        operation_group.set_index(BLOCK_OPERATION_PLACE);
        toolbar.add_widget(Box::new(operation_group));

        let mut damage_group = TheGroupButton::new(TheId::named(BLOCKS_DOCK_DAMAGE));
        damage_group.add_text_status(fl!("block_damage_clean"), fl!("status_block_damage_clean"));
        damage_group.add_text_status(
            fl!("block_damage_damaged"),
            fl!("status_block_damage_damaged"),
        );
        damage_group.set_item_width(78);
        damage_group.set_index(0);
        toolbar.add_widget(Box::new(damage_group));

        let mut stroke_group = TheGroupButton::new(TheId::named(BLOCKS_DOCK_STROKE));
        stroke_group.add_text_status(fl!("block_stroke_line"), fl!("status_block_line"));
        stroke_group.add_text_status(fl!("block_stroke_rect"), fl!("status_block_rect"));
        stroke_group.set_item_width(68);
        stroke_group.set_index(BLOCK_STROKE_LINE);
        toolbar.add_widget(Box::new(stroke_group));

        toolbar.set_reverse_index(Some(2));
        toolbar_canvas.set_layout(toolbar);
        canvas.set_top(toolbar_canvas);

        let mut center = TheCanvas::new();

        let mut board_canvas = TheCanvas::new();
        let mut board = TheIconGridView::new(TheId::named(BLOCKS_DOCK_BOARD));
        board.set_cell_size(88);
        board.set_icon_size(Self::PREVIEW_SIZE);
        board.set_icon_padding(6);
        board.set_spacing(8);
        board.set_content_padding(10);
        board_canvas.set_widget(board);
        center.set_center(board_canvas);

        let mut inspector_canvas = TheCanvas::new();
        inspector_canvas.limiter_mut().set_min_width(360);
        inspector_canvas.limiter_mut().set_max_width(360);
        let mut inspector = TheTextLayout::new(TheId::named(BLOCKS_DOCK_INSPECTOR));
        inspector.limiter_mut().set_min_width(360);
        inspector.limiter_mut().set_max_width(360);
        inspector.set_margin(Vec4::new(10, 8, 10, 8));
        inspector.set_padding(6);
        inspector.set_text_margin(10);
        inspector.set_fixed_text_width(92);
        inspector.set_text_align(TheHorizontalAlign::Right);
        inspector_canvas.set_layout(inspector);
        center.set_right(inspector_canvas);

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
        self.sync_widgets(ui, ctx, project, server_ctx);
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
            TheEvent::IndexChanged(id, index) if id.name == BLOCKS_DOCK_BOARD => {
                let Some(asset_id) = self.catalog_ids().get(*index).copied() else {
                    return false;
                };
                self.selected = asset_id;
                server_ctx.curr_block_asset_id = Some(asset_id);
                server_ctx.curr_block_asset_name = block_asset(asset_id)
                    .map(|asset| asset.name.to_string())
                    .or_else(|| {
                        project
                            .block_props
                            .get(&asset_id)
                            .map(|asset| asset.name.clone())
                    });
                self.sync_widgets(ui, ctx, project, server_ctx);
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
            TheEvent::Custom(id, TheValue::Empty) if id.name == BLOCKS_DOCK_SYNC_EVENT => {
                self.sync_widgets(ui, ctx, project, server_ctx);
                true
            }
            _ => false,
        }
    }
}
