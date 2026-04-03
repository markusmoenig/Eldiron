use crate::prelude::*;
use buildergraph::{
    BuilderAssembly, BuilderOutputSpec, BuilderOutputTarget, BuilderPreviewHost, BuilderPrimitive,
    BuilderTransform,
};
use vek::{Vec3, Vec4};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PreviewVariants {
    Single,
    AllLineDirections,
}

#[derive(Clone, Copy)]
pub struct BuilderPreviewOptions {
    pub size: u32,
    pub azimuth_deg: f32,
    pub elevation_deg: f32,
    pub scale: Option<f32>,
    pub variants: PreviewVariants,
}

impl Default for BuilderPreviewOptions {
    fn default() -> Self {
        Self {
            size: 384,
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

pub fn render_builder_preview(
    assembly: &BuilderAssembly,
    spec: BuilderOutputSpec,
    preview_host: &BuilderPreviewHost,
    options: BuilderPreviewOptions,
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
        let single_w = options.size as usize;
        let single_h = options.size as usize;
        let mut combined = vec![0_u8; single_w * yaws.len() * single_h * 4];
        for (index, yaw) in yaws.iter().enumerate() {
            let rendered = render_preview_variant(assembly, spec, preview_host, options, *yaw)?;
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
            width: options.size * yaws.len() as u32,
            height: options.size,
            pixels: combined,
        });
    }

    Ok(BuilderPreviewImage {
        width: options.size,
        height: options.size,
        pixels: render_preview_variant(assembly, spec, preview_host, options, 0.0)?,
    })
}

fn render_preview_variant(
    assembly: &BuilderAssembly,
    spec: BuilderOutputSpec,
    preview_host: &BuilderPreviewHost,
    options: BuilderPreviewOptions,
    host_yaw: f32,
) -> Result<Vec<u8>, String> {
    let dims = Vec3::new(
        preview_host.width.max(0.01),
        preview_host.height.max(0.01),
        preview_host.depth.max(0.01),
    );

    let mut scene = Scene::empty();
    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

    for primitive in &assembly.primitives {
        let mut batch = batch_for_primitive(primitive, spec, dims)?;
        batch.cull_mode = CullMode::Off;
        if host_yaw != 0.0 {
            rotate_batch_y(&mut batch, host_yaw);
        }
        let color = material_color(primitive_material_slot(primitive));
        batch = batch
            .source(PixelSource::Pixel(color))
            .ambient_color(Vec3::broadcast(0.24))
            .receives_light(true);
        extend_bounds(&mut min, &mut max, &batch.vertices);
        scene.d3_static.push(batch);
    }

    for host_batch in host_reference_batches(spec.target, dims) {
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

    let light_pos = center
        + Vec3::new(
            extent.x.max(1.0) * 0.85,
            extent.y.max(1.0) * 1.9,
            extent.z.max(1.0) * 0.30,
        );
    let light = Light::new(LightType::Point)
        .with_position(light_pos)
        .with_color([0.98, 0.96, 0.92])
        .with_intensity(2.5)
        .with_start_distance(0.0)
        .with_end_distance(extent.magnitude().max(6.0) * 3.0)
        .compile();
    scene.lights.push(light);

    let width = options.size as usize;
    let height = options.size as usize;
    let mut pixels = vec![0_u8; width * height * 4];
    let view = camera.view_matrix();
    let proj = camera.projection_matrix(width as f32, height as f32);
    let mut rasterizer = Rasterizer::setup(None, view, proj)
        .render_mode(RenderMode::render_3d())
        .background([30, 32, 36, 255])
        .ambient(Vec4::new(0.34, 0.35, 0.38, 1.0));
    let assets = Assets::default();
    rasterizer.rasterize(&mut scene, &mut pixels, width, height, 64, &assets);

    Ok(pixels)
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
            let center = translation + Vec3::new(0.0, scaled.y * 0.5, 0.0);
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
            let center = translation + Vec3::new(0.0, scaled_length * 0.5, 0.0);
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

fn material_color(slot: Option<&str>) -> [u8; 4] {
    match slot {
        Some("TOP") => [191, 164, 118, 255],
        Some("LEGS") => [138, 104, 66, 255],
        Some("BASE") => [160, 108, 68, 255],
        Some("TORCH") => [82, 72, 58, 255],
        Some("FLAME") => [235, 146, 58, 255],
        _ => [168, 140, 108, 255],
    }
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

fn host_reference_batches(target: BuilderOutputTarget, dims: Vec3<f32>) -> Vec<Batch3D> {
    match target {
        BuilderOutputTarget::Linedef => {
            let mut wall = Batch3D::from_box(-dims.x * 0.5, 0.0, -0.01, dims.x, dims.y, 0.02)
                .source(PixelSource::Pixel([78, 82, 88, 255]))
                .ambient_color(Vec3::broadcast(0.20))
                .receives_light(true);
            wall.cull_mode = CullMode::Off;
            vec![wall]
        }
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
