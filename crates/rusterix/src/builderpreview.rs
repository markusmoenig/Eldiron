use crate::prelude::*;
use buildergraph::{
    BuilderAssembly, BuilderCutMask, BuilderCutMode, BuilderCutShape, BuilderDetailPlacement,
    BuilderMasonryPattern, BuilderOutputSpec, BuilderOutputTarget, BuilderPreviewHost,
    BuilderPreviewSurface, BuilderPrimitive, BuilderSurfaceDetail, BuilderTransform,
};
use theframework::prelude::Uuid;
use vek::{Vec2, Vec3, Vec4};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PreviewVariants {
    Single,
    AllLineDirections,
}

#[derive(Clone, Copy)]
pub struct BuilderPreviewOptions {
    pub size: u32,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub azimuth_deg: f32,
    pub elevation_deg: f32,
    pub scale: Option<f32>,
    pub variants: PreviewVariants,
}

impl Default for BuilderPreviewOptions {
    fn default() -> Self {
        Self {
            size: 384,
            width: None,
            height: None,
            azimuth_deg: 135.0,
            elevation_deg: 35.264_39,
            scale: None,
            variants: PreviewVariants::Single,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BuilderPreviewImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

struct DetailPreviewBatch {
    batch: Batch3D,
    material_slot: Option<String>,
    tile_alias: Option<String>,
}

pub fn render_builder_preview(
    assembly: &BuilderAssembly,
    spec: BuilderOutputSpec,
    preview_host: &BuilderPreviewHost,
    options: BuilderPreviewOptions,
) -> Result<BuilderPreviewImage, String> {
    let assets = Assets::default();
    render_builder_preview_with_assets(assembly, spec, preview_host, options, &assets)
}

pub fn render_builder_preview_with_assets(
    assembly: &BuilderAssembly,
    spec: BuilderOutputSpec,
    preview_host: &BuilderPreviewHost,
    options: BuilderPreviewOptions,
    assets: &Assets,
) -> Result<BuilderPreviewImage, String> {
    if spec.target == BuilderOutputTarget::Linedef
        && options.variants == PreviewVariants::AllLineDirections
    {
        let yaws = [
            0.0_f32,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            std::f32::consts::PI * 1.5,
            std::f32::consts::FRAC_PI_4,
        ];
        let single_w = options.width.unwrap_or(options.size) as usize;
        let single_h = options.height.unwrap_or(options.size) as usize;
        let mut combined = vec![0_u8; single_w * yaws.len() * single_h * 4];
        for (index, yaw) in yaws.iter().enumerate() {
            let rendered =
                render_preview_variant(assembly, spec, preview_host, options, *yaw, assets)?;
            blit_variant(
                &mut combined,
                single_w * yaws.len(),
                rendered,
                single_w,
                single_h,
                index * single_w,
            );
        }
        return Ok(BuilderPreviewImage {
            width: single_w as u32 * yaws.len() as u32,
            height: single_h as u32,
            pixels: combined,
        });
    }

    Ok(BuilderPreviewImage {
        width: options.width.unwrap_or(options.size),
        height: options.height.unwrap_or(options.size),
        pixels: render_preview_variant(assembly, spec, preview_host, options, 0.0, assets)?,
    })
}

fn render_preview_variant(
    assembly: &BuilderAssembly,
    spec: BuilderOutputSpec,
    preview_host: &BuilderPreviewHost,
    options: BuilderPreviewOptions,
    host_yaw: f32,
    assets: &Assets,
) -> Result<Vec<u8>, String> {
    const PREVIEW_SSAA: usize = 2;

    let target = preview_render_target(spec.target, preview_host);
    let dims = preview_render_dims(preview_host);

    let mut scene = Scene::empty();
    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

    for primitive in &assembly.primitives {
        let mut batch = batch_for_primitive(primitive, spec, dims)?;
        batch.cull_mode = CullMode::Off;
        if host_yaw != 0.0 {
            rotate_batch_y(&mut batch, host_yaw);
        }
        batch = style_batch_color(batch, primitive_material_slot(primitive), 0.24);
        extend_bounds(&mut min, &mut max, &batch.vertices);
        scene.d3_static.push(batch);
    }

    for detail in &assembly.surface_details {
        for detail_batch in batches_for_surface_detail(detail, target, dims) {
            let mut batch = detail_batch.batch;
            batch.cull_mode = CullMode::Off;
            if host_yaw != 0.0 {
                rotate_batch_y(&mut batch, host_yaw);
            }
            batch = style_batch(
                batch,
                detail_batch.material_slot.as_deref(),
                detail_preview_pixel_source(detail_batch.tile_alias.as_deref(), assets),
                0.42,
                true,
            );
            extend_bounds(&mut min, &mut max, &batch.vertices);
            scene.d3_static.push(batch);
        }
    }

    for host_batch in host_reference_batches(target, dims, &assembly.cuts) {
        let mut batch = host_batch;
        if host_yaw != 0.0 {
            rotate_batch_y(&mut batch, host_yaw);
        }
        extend_bounds(&mut min, &mut max, &batch.vertices);
        scene.d3_static.push(batch);
    }

    if scene.d3_static.is_empty() {
        return Err("builder assembly has no primitives".to_string());
    }

    let floor = floor_batch(min, max);
    extend_bounds(&mut min, &mut max, &floor.vertices);
    scene.d3_static.push(floor);
    scene.compute_static_normals();

    let center = (min + max) * 0.5;
    let extent = max - min;

    let mut camera = <D3IsoCamera as D3Camera>::new();
    camera.center = center;
    camera.azimuth_deg = options.azimuth_deg + host_yaw.to_degrees();
    camera.elevation_deg = options.elevation_deg;
    camera.height_clearance = 0.0;
    camera.distance = extent.magnitude().max(4.0);
    camera.scale = options
        .scale
        .unwrap_or_else(|| (extent.x.max(extent.y).max(extent.z) * 0.85).max(1.5));
    camera.near = 0.1;
    camera.far = 200.0;

    let (_forward, _right, up) = camera.basis_vectors();
    let light_pos = camera.position() + up * extent.y.max(1.0) * 1.2;
    let light = Light::new(LightType::Point)
        .with_position(light_pos)
        .with_color([0.98, 0.96, 0.92])
        .with_intensity(0.35)
        .with_start_distance(0.0)
        .with_end_distance(extent.magnitude().max(6.0) * 3.0)
        .compile();
    scene.lights.push(light);

    let width = options.width.unwrap_or(options.size) as usize;
    let height = options.height.unwrap_or(options.size) as usize;
    let render_width = width * PREVIEW_SSAA;
    let render_height = height * PREVIEW_SSAA;
    let mut pixels = vec![0_u8; render_width * render_height * 4];
    let view = camera.view_matrix();
    let proj = camera.projection_matrix(render_width as f32, render_height as f32);
    let mut rasterizer = Rasterizer::setup(None, view, proj)
        .render_mode(RenderMode::render_3d())
        .background([46, 48, 52, 255])
        .ambient(Vec4::new(0.34, 0.35, 0.38, 1.0));
    rasterizer.rasterize(
        &mut scene,
        &mut pixels,
        render_width,
        render_height,
        64,
        assets,
    );

    Ok(downsample_rgba_box(
        &pixels,
        render_width,
        render_height,
        PREVIEW_SSAA,
    ))
}

fn preview_render_target(
    target: BuilderOutputTarget,
    preview_host: &BuilderPreviewHost,
) -> BuilderOutputTarget {
    if target == BuilderOutputTarget::Sector && preview_host.surface == BuilderPreviewSurface::Wall
    {
        BuilderOutputTarget::Linedef
    } else {
        target
    }
}

fn preview_render_dims(preview_host: &BuilderPreviewHost) -> Vec3<f32> {
    match preview_host.surface {
        BuilderPreviewSurface::Floor => Vec3::new(
            preview_host.width.max(0.01),
            preview_host.height.max(0.01),
            preview_host.depth.max(0.01),
        ),
        BuilderPreviewSurface::Wall => Vec3::new(
            preview_host.width.max(0.01),
            preview_host.depth.max(0.01),
            preview_host.height.max(0.01),
        ),
    }
}

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

fn batch_for_primitive(
    primitive: &BuilderPrimitive,
    spec: BuilderOutputSpec,
    dims: Vec3<f32>,
) -> Result<Batch3D, String> {
    match primitive {
        BuilderPrimitive::Box {
            size,
            transform,
            host_position_normalized,
            host_position_y_normalized,
            host_scale_y_normalized,
            host_scale_x_normalized,
            host_scale_z_normalized,
            ..
        } => {
            let scaled = Vec3::new(
                scale_x(
                    size.x * transform.scale.x,
                    *host_scale_x_normalized,
                    dims,
                    spec.target,
                ),
                scale_y(size.y * transform.scale.y, *host_scale_y_normalized, dims),
                scale_z(
                    size.z * transform.scale.z,
                    *host_scale_z_normalized,
                    dims,
                    spec.target,
                ),
            );
            let translation = scaled_translation(
                transform,
                *host_position_normalized,
                *host_position_y_normalized,
                dims,
            );
            let center = translation
                + rotate_y(
                    rotate_x(Vec3::new(0.0, scaled.y * 0.5, 0.0), transform.rotation_x),
                    transform.rotation_y,
                );
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            let mut uvs = Vec::new();
            add_box_mesh(
                &mut vertices,
                &mut indices,
                &mut uvs,
                center,
                scaled,
                transform.rotation_x,
                transform.rotation_y,
            );
            Ok(Batch3D::new(vertices, indices, uvs))
        }
        BuilderPrimitive::Cylinder {
            length,
            radius,
            transform,
            host_position_normalized,
            host_position_y_normalized,
            host_scale_y_normalized,
            host_scale_x_normalized,
            ..
        } => {
            let scaled_length =
                scale_y(*length * transform.scale.y, *host_scale_y_normalized, dims);
            let scaled_radius = if *host_scale_x_normalized {
                *radius * transform.scale.z * dims.x
            } else {
                *radius * transform.scale.z
            };
            let translation = scaled_translation(
                transform,
                *host_position_normalized,
                *host_position_y_normalized,
                dims,
            );
            let center = translation
                + rotate_y(
                    rotate_x(
                        Vec3::new(0.0, scaled_length * 0.5, 0.0),
                        transform.rotation_x,
                    ),
                    transform.rotation_y,
                );
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            let mut uvs = Vec::new();
            add_cylinder_mesh(
                &mut vertices,
                &mut indices,
                &mut uvs,
                center,
                scaled_length,
                scaled_radius,
                transform.rotation_x,
                transform.rotation_y,
                18,
            );
            Ok(Batch3D::new(vertices, indices, uvs))
        }
    }
}

fn primitive_material_slot(primitive: &BuilderPrimitive) -> Option<&str> {
    match primitive {
        BuilderPrimitive::Box { material_slot, .. } => material_slot.as_deref(),
        BuilderPrimitive::Cylinder { material_slot, .. } => material_slot.as_deref(),
    }
}

fn style_batch_color(batch: Batch3D, material_slot: Option<&str>, ambient: f32) -> Batch3D {
    style_batch(batch, material_slot, None, ambient, true)
}

fn style_batch(
    batch: Batch3D,
    material_slot: Option<&str>,
    pixel_source: Option<PixelSource>,
    ambient: f32,
    receives_light: bool,
) -> Batch3D {
    batch
        .source(pixel_source.unwrap_or_else(|| PixelSource::Pixel(material_color(material_slot))))
        .ambient_color(Vec3::broadcast(ambient))
        .receives_light(receives_light)
}

fn detail_preview_pixel_source(alias: Option<&str>, assets: &Assets) -> Option<PixelSource> {
    tile_id_by_alias(assets, alias?).map(PixelSource::TileId)
}

fn tile_id_by_alias(assets: &Assets, alias: &str) -> Option<Uuid> {
    let needle = alias.trim();
    if needle.is_empty() {
        return None;
    }
    assets.tiles.iter().find_map(|(id, tile)| {
        tile.alias
            .split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace())
            .any(|part| part.eq_ignore_ascii_case(needle))
            .then_some(*id)
    })
}

fn material_color(slot: Option<&str>) -> [u8; 4] {
    match slot {
        Some("TOP") => [191, 164, 118, 255],
        Some("LEGS") => [138, 104, 66, 255],
        Some("BASE") => [160, 108, 68, 255],
        Some("TORCH") => [82, 72, 58, 255],
        Some("FLAME") => [235, 146, 58, 255],
        Some("TRIM") => [188, 160, 112, 255],
        Some("COLUMN") => [166, 151, 126, 255],
        Some("STONE") => [150, 145, 134, 255],
        Some("WOOD") => [139, 95, 55, 255],
        Some("WALL") => [92, 68, 48, 255],
        Some("PLANK") => [164, 122, 76, 255],
        Some("ROOF") => [86, 62, 42, 255],
        Some("ROOF_PLANK") => [150, 105, 62, 255],
        Some("BEAM") => [96, 64, 38, 255],
        _ => [168, 140, 108, 255],
    }
}

fn batches_for_surface_detail(
    detail: &BuilderSurfaceDetail,
    target: BuilderOutputTarget,
    dims: Vec3<f32>,
) -> Vec<DetailPreviewBatch> {
    match detail {
        BuilderSurfaceDetail::Rect {
            min,
            max,
            offset,
            inset,
            shape,
            material_slot,
            tile_alias,
            ..
        } => rect_detail_batches(*min, *max, *offset, *inset, *shape, target, dims)
            .into_iter()
            .map(|batch| DetailPreviewBatch {
                batch,
                material_slot: material_slot.clone(),
                tile_alias: tile_alias.clone(),
            })
            .collect(),
        BuilderSurfaceDetail::Column {
            center,
            height,
            radius,
            offset,
            base_height,
            cap_height,
            transition_height,
            segments,
            placement,
            material_slot,
            rect_material_slot,
            cyl_material_slot,
            tile_alias,
            ..
        } => column_detail_batches(
            *center,
            *height,
            *radius,
            *offset,
            *base_height,
            *cap_height,
            *transition_height,
            usize::from(*segments),
            *placement,
            material_slot.as_deref(),
            rect_material_slot.as_deref(),
            cyl_material_slot.as_deref(),
            tile_alias.as_deref(),
            target,
            dims,
        ),
        BuilderSurfaceDetail::Masonry {
            min,
            max,
            block,
            mortar,
            offset,
            pattern,
            material_slot,
            tile_alias,
            ..
        } => masonry_detail_batches(*min, *max, *block, *mortar, *offset, *pattern, target, dims)
            .into_iter()
            .map(|batch| DetailPreviewBatch {
                batch,
                material_slot: material_slot.clone(),
                tile_alias: tile_alias.clone(),
            })
            .collect(),
    }
}

fn masonry_detail_batches(
    min: Vec2<f32>,
    max: Vec2<f32>,
    block: Vec2<f32>,
    mortar: f32,
    offset: f32,
    pattern: BuilderMasonryPattern,
    target: BuilderOutputTarget,
    dims: Vec3<f32>,
) -> Vec<Batch3D> {
    masonry_block_rects(min, max, block, mortar, pattern)
        .into_iter()
        .map(|(block_min, block_max)| {
            surface_slab_batch(
                block_min.x,
                block_min.y,
                block_max.x,
                block_max.y,
                offset,
                target,
                dims,
            )
        })
        .collect()
}

fn rect_detail_batches(
    min: vek::Vec2<f32>,
    max: vek::Vec2<f32>,
    offset: f32,
    inset: f32,
    shape: BuilderCutShape,
    target: BuilderOutputTarget,
    dims: Vec3<f32>,
) -> Vec<Batch3D> {
    let u0 = min.x.min(max.x);
    let u1 = min.x.max(max.x);
    let v0 = min.y.min(max.y);
    let v1 = min.y.max(max.y);
    if (u1 - u0) <= 0.001 || (v1 - v0) <= 0.001 {
        return Vec::new();
    }

    match shape {
        BuilderCutShape::Fill => vec![surface_slab_batch(u0, v0, u1, v1, offset, target, dims)],
        BuilderCutShape::Border => {
            let inset = inset.max(0.01).min((u1 - u0).min(v1 - v0) * 0.45);
            let mut batches = Vec::new();
            batches.push(surface_slab_batch(
                u0,
                v0,
                u1,
                v0 + inset,
                offset,
                target,
                dims,
            ));
            batches.push(surface_slab_batch(
                u0,
                v1 - inset,
                u1,
                v1,
                offset,
                target,
                dims,
            ));
            batches.push(surface_slab_batch(
                u0,
                v0 + inset,
                u0 + inset,
                v1 - inset,
                offset,
                target,
                dims,
            ));
            batches.push(surface_slab_batch(
                u1 - inset,
                v0 + inset,
                u1,
                v1 - inset,
                offset,
                target,
                dims,
            ));
            batches
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn column_detail_batches(
    center: vek::Vec2<f32>,
    height: f32,
    radius: f32,
    offset: f32,
    base_height: f32,
    cap_height: f32,
    transition_height: f32,
    segments: usize,
    placement: BuilderDetailPlacement,
    material_slot: Option<&str>,
    rect_material_slot: Option<&str>,
    cyl_material_slot: Option<&str>,
    tile_alias: Option<&str>,
    target: BuilderOutputTarget,
    dims: Vec3<f32>,
) -> Vec<DetailPreviewBatch> {
    let height = height.max(0.01);
    let radius = radius.max(0.01);
    let segments = segments.clamp(6, 48);
    let base_height = base_height.max(0.0);
    let cap_height = cap_height.max(0.0);
    let transition_height =
        column_transition_height(height, base_height, cap_height, transition_height);
    let mut batches = Vec::new();
    let rect_material_slot = rect_material_slot.or(material_slot).map(str::to_string);
    let cyl_material_slot = cyl_material_slot.or(material_slot).map(str::to_string);
    let tile_alias = tile_alias.map(str::to_string);
    let add_rect_batch = |batches: &mut Vec<DetailPreviewBatch>, batch: Batch3D| {
        batches.push(DetailPreviewBatch {
            batch,
            material_slot: rect_material_slot.clone(),
            tile_alias: tile_alias.clone(),
        });
    };
    let add_cyl_batch = |batches: &mut Vec<DetailPreviewBatch>, batch: Batch3D| {
        batches.push(DetailPreviewBatch {
            batch,
            material_slot: cyl_material_slot.clone(),
            tile_alias: tile_alias.clone(),
        });
    };

    match target {
        BuilderOutputTarget::Sector | BuilderOutputTarget::VertexPair => {
            let base_x = center.x - dims.x * 0.5;
            let base_z = center.y - dims.z * 0.5;
            let y0 = match placement {
                BuilderDetailPlacement::Relief => offset.max(0.0),
                BuilderDetailPlacement::Attached => offset.max(0.0),
                BuilderDetailPlacement::Structural => 0.0,
                BuilderDetailPlacement::Freestanding => 0.0,
            };
            let shaft_base_y = y0 + base_height + transition_height;
            let shaft_height =
                (height - base_height - cap_height - transition_height * 2.0).max(0.02);
            if base_height > 0.0 {
                add_rect_batch(
                    &mut batches,
                    Batch3D::from_box(
                        base_x - radius * 1.45,
                        y0,
                        base_z - radius * 1.45,
                        radius * 2.9,
                        base_height,
                        radius * 2.9,
                    ),
                );
            }
            if transition_height > 0.0 {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                let mut uvs = Vec::new();
                add_tapered_cylinder_mesh(
                    &mut vertices,
                    &mut indices,
                    &mut uvs,
                    Vec3::new(base_x, y0 + base_height + transition_height * 0.5, base_z),
                    transition_height,
                    radius * 1.45,
                    radius,
                    0.0,
                    0.0,
                    segments,
                );
                add_cyl_batch(&mut batches, Batch3D::new(vertices, indices, uvs));
            }
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            let mut uvs = Vec::new();
            add_cylinder_mesh(
                &mut vertices,
                &mut indices,
                &mut uvs,
                Vec3::new(base_x, shaft_base_y + shaft_height * 0.5, base_z),
                shaft_height,
                radius,
                0.0,
                0.0,
                segments,
            );
            add_cyl_batch(&mut batches, Batch3D::new(vertices, indices, uvs));
            if transition_height > 0.0 {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                let mut uvs = Vec::new();
                add_tapered_cylinder_mesh(
                    &mut vertices,
                    &mut indices,
                    &mut uvs,
                    Vec3::new(
                        base_x,
                        shaft_base_y + shaft_height + transition_height * 0.5,
                        base_z,
                    ),
                    transition_height,
                    radius,
                    radius * 1.45,
                    0.0,
                    0.0,
                    segments,
                );
                add_cyl_batch(&mut batches, Batch3D::new(vertices, indices, uvs));
            }
            if cap_height > 0.0 {
                add_rect_batch(
                    &mut batches,
                    Batch3D::from_box(
                        base_x - radius * 1.45,
                        shaft_base_y + shaft_height + transition_height,
                        base_z - radius * 1.45,
                        radius * 2.9,
                        cap_height,
                        radius * 2.9,
                    ),
                );
            }
        }
        BuilderOutputTarget::Linedef => {
            if placement == BuilderDetailPlacement::Relief {
                let add_wall_slab =
                    |u0: f32, v0: f32, u1: f32, v1: f32, batches: &mut Vec<DetailPreviewBatch>| {
                        if u1 > u0 && v1 > v0 {
                            add_rect_batch(
                                batches,
                                surface_slab_batch(u0, v0, u1, v1, offset, target, dims),
                            );
                        }
                    };
                let half_base = radius * 1.45;
                let half_transition = (half_base + radius) * 0.5;
                let shaft_y0 = center.y + base_height + transition_height;
                let shaft_height =
                    (height - base_height - cap_height - transition_height * 2.0).max(0.02);

                if base_height > 0.0 {
                    add_wall_slab(
                        center.x - half_base,
                        center.y,
                        center.x + half_base,
                        center.y + base_height,
                        &mut batches,
                    );
                }
                if transition_height > 0.0 {
                    add_wall_slab(
                        center.x - half_transition,
                        center.y + base_height,
                        center.x + half_transition,
                        center.y + base_height + transition_height,
                        &mut batches,
                    );
                }
                add_wall_slab(
                    center.x - radius,
                    shaft_y0,
                    center.x + radius,
                    shaft_y0 + shaft_height,
                    &mut batches,
                );
                if transition_height > 0.0 {
                    add_wall_slab(
                        center.x - half_transition,
                        shaft_y0 + shaft_height,
                        center.x + half_transition,
                        shaft_y0 + shaft_height + transition_height,
                        &mut batches,
                    );
                }
                if cap_height > 0.0 {
                    add_wall_slab(
                        center.x - half_base,
                        center.y + height - cap_height,
                        center.x + half_base,
                        center.y + height,
                        &mut batches,
                    );
                }
                return batches;
            }

            let x = center.x - dims.x * 0.5;
            let y = center.y + base_height + transition_height;
            let cap_depth = if placement == BuilderDetailPlacement::Attached {
                radius * 2.9
            } else {
                radius * 0.7
            };
            let z = if placement == BuilderDetailPlacement::Attached {
                offset.abs().max(cap_depth * 0.5)
            } else if placement == BuilderDetailPlacement::Structural {
                offset
            } else {
                -offset
            };
            let shaft_height =
                (height - base_height - cap_height - transition_height * 2.0).max(0.02);
            if base_height > 0.0 {
                add_rect_batch(
                    &mut batches,
                    Batch3D::from_box(
                        x - radius * 1.45,
                        center.y,
                        z - cap_depth * 0.5,
                        radius * 2.9,
                        base_height,
                        cap_depth,
                    ),
                );
            }
            if transition_height > 0.0 {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                let mut uvs = Vec::new();
                add_tapered_cylinder_mesh(
                    &mut vertices,
                    &mut indices,
                    &mut uvs,
                    Vec3::new(x, center.y + base_height + transition_height * 0.5, z),
                    transition_height,
                    radius * 1.45,
                    radius,
                    0.0,
                    0.0,
                    segments,
                );
                add_cyl_batch(&mut batches, Batch3D::new(vertices, indices, uvs));
            }
            let mut vertices = Vec::new();
            let mut indices = Vec::new();
            let mut uvs = Vec::new();
            add_cylinder_mesh(
                &mut vertices,
                &mut indices,
                &mut uvs,
                Vec3::new(x, y + shaft_height * 0.5, z),
                shaft_height,
                radius,
                0.0,
                0.0,
                segments,
            );
            add_cyl_batch(&mut batches, Batch3D::new(vertices, indices, uvs));
            if transition_height > 0.0 {
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                let mut uvs = Vec::new();
                add_tapered_cylinder_mesh(
                    &mut vertices,
                    &mut indices,
                    &mut uvs,
                    Vec3::new(x, y + shaft_height + transition_height * 0.5, z),
                    transition_height,
                    radius,
                    radius * 1.45,
                    0.0,
                    0.0,
                    segments,
                );
                add_cyl_batch(&mut batches, Batch3D::new(vertices, indices, uvs));
            }
            if cap_height > 0.0 {
                add_rect_batch(
                    &mut batches,
                    Batch3D::from_box(
                        x - radius * 1.45,
                        y + shaft_height + transition_height,
                        z - cap_depth * 0.5,
                        radius * 2.9,
                        cap_height,
                        cap_depth,
                    ),
                );
            }
        }
    }

    batches
}

fn surface_slab_batch(
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    offset: f32,
    target: BuilderOutputTarget,
    dims: Vec3<f32>,
) -> Batch3D {
    let thickness = offset.abs().max(0.035);
    match target {
        BuilderOutputTarget::Sector | BuilderOutputTarget::VertexPair => {
            let x = u0 - dims.x * 0.5;
            let z = v0 - dims.z * 0.5;
            let y = if offset < 0.0 { 0.0 } else { -thickness };
            Batch3D::from_box(x, y, z, u1 - u0, thickness, v1 - v0)
        }
        BuilderOutputTarget::Linedef => {
            let x = u0 - dims.x * 0.5;
            let y = v0;
            let z = if offset < 0.0 { 0.0 } else { -thickness };
            Batch3D::from_box(x, y, z, u1 - u0, v1 - v0, thickness)
        }
    }
}

fn masonry_block_rects(
    min: Vec2<f32>,
    max: Vec2<f32>,
    block: Vec2<f32>,
    mortar: f32,
    pattern: BuilderMasonryPattern,
) -> Vec<(Vec2<f32>, Vec2<f32>)> {
    let raw_min = min;
    let raw_max = max;
    let min = Vec2::new(raw_min.x.min(raw_max.x), raw_min.y.min(raw_max.y));
    let max = Vec2::new(raw_min.x.max(raw_max.x), raw_min.y.max(raw_max.y));
    let block = Vec2::new(block.x.max(0.001), block.y.max(0.001));
    let mortar = mortar.max(0.0);
    let edge_margin = (mortar * 0.5).max(0.002);
    let gap = mortar * 0.5;
    let area_min = min + Vec2::broadcast(edge_margin);
    let area_max = max - Vec2::broadcast(edge_margin);
    if area_max.x <= area_min.x || area_max.y <= area_min.y {
        return Vec::new();
    }

    let mut rects = Vec::new();
    let mut row = 0usize;
    let mut y0 = area_min.y;
    while y0 < area_max.y - 0.001 && row < 512 {
        let y1 = (y0 + block.y).min(area_max.y);
        let row_offset = match pattern {
            BuilderMasonryPattern::Grid => 0.0,
            BuilderMasonryPattern::RunningBond if row % 2 == 1 => block.x * 0.5,
            BuilderMasonryPattern::RunningBond => 0.0,
        };
        let mut x0 = area_min.x - row_offset;
        let mut col = 0usize;
        while x0 < area_max.x - 0.001 && col < 1024 {
            let x1 = x0 + block.x;
            let clipped_min = Vec2::new(x0.max(area_min.x) + gap, y0 + gap);
            let clipped_max = Vec2::new(x1.min(area_max.x) - gap, y1 - gap);
            if clipped_max.x > clipped_min.x + 0.001 && clipped_max.y > clipped_min.y + 0.001 {
                rects.push((clipped_min, clipped_max));
            }
            x0 += block.x;
            col += 1;
        }
        y0 += block.y;
        row += 1;
    }

    rects
}

fn scale_x(value: f32, normalized: bool, dims: Vec3<f32>, _target: BuilderOutputTarget) -> f32 {
    if normalized { value * dims.x } else { value }
}

fn scale_y(value: f32, normalized: bool, dims: Vec3<f32>) -> f32 {
    if normalized { value * dims.y } else { value }
}

fn scale_z(value: f32, normalized: bool, dims: Vec3<f32>, _target: BuilderOutputTarget) -> f32 {
    if normalized { value * dims.z } else { value }
}

fn scaled_translation(
    transform: &BuilderTransform,
    pos_normalized: bool,
    pos_y_normalized: bool,
    dims: Vec3<f32>,
) -> Vec3<f32> {
    Vec3::new(
        if pos_normalized {
            transform.translation.x * dims.x
        } else {
            transform.translation.x
        },
        if pos_y_normalized {
            transform.translation.y * dims.y
        } else {
            transform.translation.y
        },
        if pos_normalized {
            transform.translation.z * dims.z
        } else {
            transform.translation.z
        },
    )
}

fn floor_batch(min: Vec3<f32>, max: Vec3<f32>) -> Batch3D {
    let pad = 0.3;
    let y = min.y.min(0.0) - 0.02;
    let x0 = min.x - pad;
    let x1 = max.x + pad;
    let z0 = min.z - pad;
    let z1 = max.z + pad;
    let vertices = vec![
        [x0, y, z0, 1.0],
        [x1, y, z0, 1.0],
        [x1, y, z1, 1.0],
        [x0, y, z1, 1.0],
    ];
    let indices = vec![(0, 1, 2), (0, 2, 3)];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    Batch3D::new(vertices, indices, uvs)
        .source(PixelSource::Pixel([54, 56, 60, 255]))
        .ambient_color(Vec3::broadcast(0.18))
        .receives_light(true)
}

fn host_reference_batches(
    target: BuilderOutputTarget,
    dims: Vec3<f32>,
    cuts: &[BuilderCutMask],
) -> Vec<Batch3D> {
    match target {
        BuilderOutputTarget::Linedef => host_wall_reference_batches(dims, cuts),
        BuilderOutputTarget::Sector => {
            let mut plane =
                Batch3D::from_box(-dims.x * 0.5, -0.01, -dims.z * 0.5, dims.x, 0.02, dims.z)
                    .source(PixelSource::Pixel([72, 76, 82, 255]))
                    .ambient_color(Vec3::broadcast(0.18))
                    .receives_light(true);
            plane.cull_mode = CullMode::Off;
            vec![plane]
        }
        BuilderOutputTarget::VertexPair => Vec::new(),
    }
}

fn host_wall_reference_batches(dims: Vec3<f32>, cuts: &[BuilderCutMask]) -> Vec<Batch3D> {
    let mut spans = vec![(0.0_f32, dims.x)];
    for cut in cuts {
        let BuilderCutMask::Rect { min, max, mode, .. } = cut else {
            continue;
        };
        if !matches!(mode, BuilderCutMode::Cut | BuilderCutMode::Replace) {
            continue;
        }
        let x0 = min.x.min(max.x).clamp(0.0, dims.x);
        let x1 = min.x.max(max.x).clamp(0.0, dims.x);
        let y0 = min.y.min(max.y);
        let y1 = min.y.max(max.y);
        if x1 <= x0 || y0 > 0.001 || y1 < dims.y - 0.001 {
            continue;
        }
        let mut next = Vec::new();
        for (span0, span1) in spans {
            if x1 <= span0 || x0 >= span1 {
                next.push((span0, span1));
            } else {
                if x0 > span0 {
                    next.push((span0, x0));
                }
                if x1 < span1 {
                    next.push((x1, span1));
                }
            }
        }
        spans = next;
    }

    spans
        .into_iter()
        .filter_map(|(x0, x1)| {
            let width = x1 - x0;
            (width > 0.001).then(|| {
                let mut wall =
                    Batch3D::from_box(x0 - dims.x * 0.5, 0.0, -0.01, width, dims.y, 0.02)
                        .source(PixelSource::Pixel([78, 82, 88, 255]))
                        .ambient_color(Vec3::broadcast(0.20))
                        .receives_light(true);
                wall.cull_mode = CullMode::Off;
                wall
            })
        })
        .collect()
}

fn extend_bounds(min: &mut Vec3<f32>, max: &mut Vec3<f32>, vertices: &[[f32; 4]]) {
    for vertex in vertices {
        min.x = min.x.min(vertex[0]);
        min.y = min.y.min(vertex[1]);
        min.z = min.z.min(vertex[2]);
        max.x = max.x.max(vertex[0]);
        max.y = max.y.max(vertex[1]);
        max.z = max.z.max(vertex[2]);
    }
}

fn rotate_x(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x, v.y * c - v.z * s, v.y * s + v.z * c)
}

fn rotate_y(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x * c - v.z * s, v.y, v.x * s + v.z * c)
}

fn rotate_batch_y(batch: &mut Batch3D, yaw: f32) {
    for vertex in &mut batch.vertices {
        let rotated = rotate_y(Vec3::new(vertex[0], vertex[1], vertex[2]), yaw);
        vertex[0] = rotated.x;
        vertex[1] = rotated.y;
        vertex[2] = rotated.z;
    }
}

fn blit_variant(
    destination: &mut [u8],
    dest_width: usize,
    source: Vec<u8>,
    src_width: usize,
    src_height: usize,
    dest_x: usize,
) {
    for y in 0..src_height {
        let dest_row = (y * dest_width + dest_x) * 4;
        let src_row = y * src_width * 4;
        destination[dest_row..dest_row + src_width * 4]
            .copy_from_slice(&source[src_row..src_row + src_width * 4]);
    }
}

fn column_transition_height(height: f32, base_height: f32, cap_height: f32, requested: f32) -> f32 {
    let requested = requested.max(0.0);
    if requested <= 0.0 {
        return 0.0;
    }
    let available = height - base_height - cap_height;
    if available <= 0.03 {
        return 0.0;
    }
    requested.min((available - 0.02) * 0.5)
}

fn add_box_mesh(
    vertices: &mut Vec<[f32; 4]>,
    indices: &mut Vec<(usize, usize, usize)>,
    uvs: &mut Vec<[f32; 2]>,
    center: Vec3<f32>,
    size: Vec3<f32>,
    rotation_x: f32,
    rotation_y: f32,
) {
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;
    let local = [
        // Front face
        Vec3::new(-hx, -hy, -hz),
        Vec3::new(hx, -hy, -hz),
        Vec3::new(hx, hy, -hz),
        Vec3::new(-hx, hy, -hz),
        // Back face
        Vec3::new(-hx, -hy, hz),
        Vec3::new(hx, -hy, hz),
        Vec3::new(hx, hy, hz),
        Vec3::new(-hx, hy, hz),
        // Left face
        Vec3::new(-hx, -hy, -hz),
        Vec3::new(-hx, hy, -hz),
        Vec3::new(-hx, hy, hz),
        Vec3::new(-hx, -hy, hz),
        // Right face
        Vec3::new(hx, -hy, -hz),
        Vec3::new(hx, hy, -hz),
        Vec3::new(hx, hy, hz),
        Vec3::new(hx, -hy, hz),
        // Top face
        Vec3::new(-hx, hy, -hz),
        Vec3::new(hx, hy, -hz),
        Vec3::new(hx, hy, hz),
        Vec3::new(-hx, hy, hz),
        // Bottom face
        Vec3::new(-hx, -hy, -hz),
        Vec3::new(hx, -hy, -hz),
        Vec3::new(hx, -hy, hz),
        Vec3::new(-hx, -hy, hz),
    ];
    let base = vertices.len();
    for point in local {
        let rotated = rotate_y(rotate_x(point, rotation_x), rotation_y);
        vertices.push([
            center.x + rotated.x,
            center.y + rotated.y,
            center.z + rotated.z,
            1.0,
        ]);
    }
    uvs.extend_from_slice(&[
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
    ]);
    indices.extend_from_slice(&[
        (base, base + 1, base + 2),
        (base, base + 2, base + 3),
        (base + 4, base + 6, base + 5),
        (base + 4, base + 7, base + 6),
        (base + 8, base + 9, base + 10),
        (base + 8, base + 10, base + 11),
        (base + 12, base + 14, base + 13),
        (base + 12, base + 15, base + 14),
        (base + 16, base + 17, base + 18),
        (base + 16, base + 18, base + 19),
        (base + 20, base + 23, base + 22),
        (base + 20, base + 22, base + 21),
    ]);
}

#[allow(clippy::too_many_arguments)]
fn add_tapered_cylinder_mesh(
    vertices: &mut Vec<[f32; 4]>,
    indices: &mut Vec<(usize, usize, usize)>,
    uvs: &mut Vec<[f32; 2]>,
    center: Vec3<f32>,
    length: f32,
    bottom_radius: f32,
    top_radius: f32,
    rotation_x: f32,
    rotation_y: f32,
    segments: usize,
) {
    let half = length * 0.5;
    let base = vertices.len();

    for ring in 0..2 {
        let y = if ring == 0 { -half } else { half };
        let radius = if ring == 0 { bottom_radius } else { top_radius };
        for i in 0..segments {
            let t = i as f32 / segments as f32 * std::f32::consts::TAU;
            let local = Vec3::new(t.cos() * radius, y, t.sin() * radius);
            let rotated = rotate_y(rotate_x(local, rotation_x), rotation_y);
            vertices.push([
                center.x + rotated.x,
                center.y + rotated.y,
                center.z + rotated.z,
                1.0,
            ]);
            uvs.push([i as f32 / segments as f32, ring as f32]);
        }
    }

    for i in 0..segments {
        let next = (i + 1) % segments;
        let b0 = base + i;
        let b1 = base + next;
        let t0 = base + segments + i;
        let t1 = base + segments + next;
        indices.push((b0, b1, t1));
        indices.push((b0, t1, t0));
    }
}

fn add_cylinder_mesh(
    vertices: &mut Vec<[f32; 4]>,
    indices: &mut Vec<(usize, usize, usize)>,
    uvs: &mut Vec<[f32; 2]>,
    center: Vec3<f32>,
    length: f32,
    radius: f32,
    rotation_x: f32,
    rotation_y: f32,
    segments: usize,
) {
    let half = length * 0.5;
    let base = vertices.len();

    for ring in 0..2 {
        let y = if ring == 0 { -half } else { half };
        for i in 0..segments {
            let t = i as f32 / segments as f32 * std::f32::consts::TAU;
            let local = Vec3::new(t.cos() * radius, y, t.sin() * radius);
            let rotated = rotate_y(rotate_x(local, rotation_x), rotation_y);
            vertices.push([
                center.x + rotated.x,
                center.y + rotated.y,
                center.z + rotated.z,
                1.0,
            ]);
            uvs.push([i as f32 / segments as f32, ring as f32]);
        }
    }

    let bottom_center = vertices.len();
    let rotated_bottom = rotate_y(rotate_x(Vec3::new(0.0, -half, 0.0), rotation_x), rotation_y);
    vertices.push([
        center.x + rotated_bottom.x,
        center.y + rotated_bottom.y,
        center.z + rotated_bottom.z,
        1.0,
    ]);
    uvs.push([0.5, 0.5]);

    let top_center = vertices.len();
    let rotated_top = rotate_y(rotate_x(Vec3::new(0.0, half, 0.0), rotation_x), rotation_y);
    vertices.push([
        center.x + rotated_top.x,
        center.y + rotated_top.y,
        center.z + rotated_top.z,
        1.0,
    ]);
    uvs.push([0.5, 0.5]);

    for i in 0..segments {
        let next = (i + 1) % segments;
        let b0 = base + i;
        let b1 = base + next;
        let t0 = base + segments + i;
        let t1 = base + segments + next;
        indices.push((b0, b1, t1));
        indices.push((b0, t1, t0));
        indices.push((bottom_center, b1, b0));
        indices.push((top_center, t0, t1));
    }
}
