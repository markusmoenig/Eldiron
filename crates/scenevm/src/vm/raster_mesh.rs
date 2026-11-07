//! Converted world-space mesh vertices retained between Raster3D frames.
use super::Vert3DPod;
use crate::{
    GeoId,
    dynamic::{DynamicMeshVertex, DynamicObject},
};
use std::sync::Arc;
use vek::{Mat4, Vec3, Vec4};

#[derive(Default)]
pub(super) struct MeshCache {
    key: Option<(Mat4<f32>, u32, u32, bool, bool)>,
    source: Arc<Vec<DynamicMeshVertex>>,
    vertices: Vec<Vert3DPod>,
    base: usize,
    indices: Arc<Vec<u32>>,
    pub occluders_unchanged: bool,
    pub bounds: Option<Bounds>,
}

impl MeshCache {
    /// The host can freely edit mesh data, including UVs and normals. Compare
    /// those inputs rather than trusting object identity or vertex counts.
    pub fn append(
        &mut self,
        object: &DynamicObject,
        transform: Mat4<f32>,
        tile: u32,
        out: &mut Vec<Vert3DPod>,
    ) -> bool {
        let opacity = object.opacity.clamp(0., 1.);
        let entity = matches!(object.id, GeoId::Character(_) | GeoId::Item(_));
        let key = (transform, tile, opacity.to_bits(), object.pixel_art, entity);
        let reused = self.key == Some(key)
            && (Arc::ptr_eq(&self.source, &object.mesh_vertices)
                || self.source == object.mesh_vertices);
        self.occluders_unchanged = reused
            && (Arc::ptr_eq(&self.indices, &object.mesh_indices)
                || self.indices == object.mesh_indices);
        if !reused {
            let identity = transform == Mat4::identity();
            self.vertices.clear();
            self.vertices.extend(object.mesh_vertices.iter().map(|v| {
                // MZ already supplies world-space positions. Avoid two full
                // matrix multiplications per vertex for the identity transform.
                let (position, mut normal) = if identity {
                    (v.position, v.normal)
                } else {
                    let p = transform * Vec4::new(v.position.x, v.position.y, v.position.z, 1.);
                    let w = if p.w != 0. { p.w } else { 1. };
                    let n = transform * Vec4::new(v.normal.x, v.normal.y, v.normal.z, 0.);
                    (
                        Vec3::new(p.x / w, p.y / w, p.z / w),
                        Vec3::new(n.x, n.y, n.z),
                    )
                };
                let length2 = normal.x * normal.x + normal.y * normal.y + normal.z * normal.z;
                normal = if length2 <= 1e-8 {
                    Vec3::unit_y()
                } else {
                    let length = length2.sqrt();
                    Vec3::new(normal.x / length, normal.y / length, normal.z / length)
                };
                Vert3DPod {
                    pos: position.into_array(),
                    entity_pixel_target: if entity { 1. } else { 0. },
                    uv: v.uv.into_array(),
                    _pad1: [0.; 2],
                    tile_index: tile,
                    tile_index2: tile | 0x2000_0000,
                    blend_factor: if object.pixel_art { -1. } else { 0. },
                    opacity,
                    normal: normal.into_array(),
                    _pad2: 0.,
                    paint_geo: [0; 4],
                    paint_uv: [0.; 2],
                    _pad3: [0.; 2],
                    paint_geo_fallback: [0; 4],
                }
            }));
            self.bounds = Bounds::from_vertices(&self.vertices);
            self.key = Some(key);
        }
        if !Arc::ptr_eq(&self.source, &object.mesh_vertices) {
            self.source.clone_from(&object.mesh_vertices);
        }
        if !Arc::ptr_eq(&self.indices, &object.mesh_indices) {
            self.indices.clone_from(&object.mesh_indices);
        }
        let same_range = reused && self.base == out.len();
        self.base = out.len();
        out.extend_from_slice(&self.vertices);
        same_range
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Bounds {
    pub min: Vec3<f32>,
    pub max: Vec3<f32>,
}
impl Bounds {
    pub fn from_vertices(vertices: &[Vert3DPod]) -> Option<Self> {
        if vertices.is_empty() {
            return None;
        }
        let mut bounds = Self {
            min: Vec3::broadcast(f32::INFINITY),
            max: Vec3::broadcast(f32::NEG_INFINITY),
        };
        for vertex in vertices {
            bounds.min.x = bounds.min.x.min(vertex.pos[0]);
            bounds.min.y = bounds.min.y.min(vertex.pos[1]);
            bounds.min.z = bounds.min.z.min(vertex.pos[2]);
            bounds.max.x = bounds.max.x.max(vertex.pos[0]);
            bounds.max.y = bounds.max.y.max(vertex.pos[1]);
            bounds.max.z = bounds.max.z.max(vertex.pos[2]);
        }
        Some(bounds)
    }
    pub fn include(target: &mut Option<Self>, other: Option<Self>) {
        if let Some(other) = other {
            if let Some(target) = target {
                target.min = target.min.map2(other.min, f32::min);
                target.max = target.max.map2(other.max, f32::max);
            } else {
                *target = Some(other);
            }
        }
    }
}

/// Raster mesh attributes are uniform across a dynamic object's triangles.
pub(super) struct MeshSpan {
    pub vertex_start: usize,
    pub first_index: usize,
    pub end_index: usize,
    pub tile: u32,
    pub opacity: f32,
    pub id: GeoId,
}

/// Coalesce adjacent changed ranges into one GPU write.
pub(super) fn add_upload_range(ranges: &mut Vec<(usize, usize)>, start: usize, end: usize) {
    if start == end {
        return;
    }
    if let Some(last) = ranges.last_mut()
        && last.1 == start
    {
        last.1 = end;
    } else {
        ranges.push((start, end));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vek::Vec2;
    #[test]
    fn retained_vertices_follow_pose_uv_normal_material_and_transform_edits() {
        let mut object = DynamicObject::default();
        object.id = GeoId::Character(1);
        object.mesh_vertices = vec![DynamicMeshVertex {
            position: Vec3::new(2., 3., 4.),
            uv: Vec2::new(0.2, 0.7),
            normal: Vec3::new(0., 2., 0.),
        }]
        .into();
        let mut cache = MeshCache::default();
        let mut out = vec![];
        assert!(!cache.append(&object, Mat4::identity(), 7, &mut out));
        let initial = bytemuck::cast_slice::<_, u8>(&out).to_vec();
        out.clear();
        assert!(cache.append(&object, Mat4::identity(), 7, &mut out));
        assert_eq!(bytemuck::cast_slice::<_, u8>(&out), initial);
        for edit in 0..7 {
            match edit {
                0 => Arc::make_mut(&mut object.mesh_vertices)[0].position.x += 1.,
                1 => Arc::make_mut(&mut object.mesh_vertices)[0].uv.y += 0.1,
                2 => Arc::make_mut(&mut object.mesh_vertices)[0].normal = Vec3::zero(),
                3 => object.opacity = 0.4,
                4 => object.pixel_art = true,
                5 => object.id = GeoId::Unknown(1),
                _ => Arc::make_mut(&mut object.mesh_vertices).clear(),
            }
            out.clear();
            assert!(
                !cache.append(&object, Mat4::identity(), 7, &mut out),
                "edit {edit}"
            );
        }
        assert!(out.is_empty(), "Removed meshes must not leave old vertices");
        Arc::make_mut(&mut object.mesh_vertices).push(DynamicMeshVertex {
            position: Vec3::new(2., 3., 4.),
            uv: Vec2::zero(),
            normal: Vec3::unit_y(),
        });
        out.clear();
        cache.append(&object, Mat4::identity(), 7, &mut out);
        out.clear();
        assert!(!cache.append(&object, Mat4::identity(), 8, &mut out));
        assert_eq!(out[0].tile_index, 8);
        let transform = Mat4::<f32>::translation_3d(Vec3::<f32>::new(3., 4., 5.))
            * Mat4::<f32>::scaling_3d(Vec3::<f32>::new(2., 3., 4.));
        out.clear();
        assert!(!cache.append(&object, transform, 8, &mut out));
        assert_eq!(out[0].pos, [7., 13., 21.]);
        assert_eq!(out[0].normal, [0., 1., 0.]);
        out.truncate(1);
        assert!(
            !cache.append(&object, transform, 8, &mut out),
            "An unchanged mesh moved to a different GPU range needs uploading"
        );
        assert_eq!(out[1].pos, out[0].pos);
        let original = object.mesh_vertices.clone();
        let mut copy = object.clone();
        Arc::make_mut(&mut copy.mesh_vertices)[0].position.x += 1.;
        assert_eq!(object.mesh_vertices, original);
        assert_ne!(
            copy.mesh_vertices, original,
            "An instance edit must detach shared vertices"
        );
    }
}

#[derive(Clone, Copy)]
pub(super) struct ShadowFrame {
    pub center: Vec3<f32>,
    pub half_w: f32,
    pub half_h: f32,
    pub near: f32,
    pub far: f32,
    pub right: Vec3<f32>,
    pub up: Vec3<f32>,
    pub forward: Vec3<f32>,
}
pub(super) fn shadow_frame(
    shadow_bounds_verts: &[Vert3DPod],
    sun: Vec4<f32>,
    shadow_res: u32,
) -> ShadowFrame {
    let mut shadow_center = Vec3::zero();
    let mut shadow_half_w = 32.0f32;
    let mut shadow_half_h = 32.0f32;
    let mut shadow_near = -32.0f32;
    let mut shadow_far = 32.0f32;
    if !shadow_bounds_verts.is_empty() {
        let mut bmin = Vec3::broadcast(f32::INFINITY);
        let mut bmax = Vec3::broadcast(f32::NEG_INFINITY);
        for v in shadow_bounds_verts {
            let p = Vec3::new(v.pos[0], v.pos[1], v.pos[2]);
            bmin.x = bmin.x.min(p.x);
            bmin.y = bmin.y.min(p.y);
            bmin.z = bmin.z.min(p.z);
            bmax.x = bmax.x.max(p.x);
            bmax.y = bmax.y.max(p.y);
            bmax.z = bmax.z.max(p.z);
        }
        shadow_center = (bmin + bmax) * 0.5;
    }
    let sun_l_raw = Vec3::new(-sun.x, -sun.y, -sun.z);
    let sun_l = if sun_l_raw.magnitude_squared() > 1e-6 {
        sun_l_raw.normalized()
    } else {
        Vec3::new(0.0, -1.0, 0.0)
    };
    let shadow_fwd = -sun_l;
    let up_hint = if shadow_fwd.y.abs() > 0.99 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    let shadow_right = up_hint.cross(shadow_fwd).normalized();
    let shadow_up = shadow_fwd.cross(shadow_right).normalized();
    if !shadow_bounds_verts.is_empty() {
        let mut minx = f32::INFINITY;
        let mut maxx = f32::NEG_INFINITY;
        let mut miny = f32::INFINITY;
        let mut maxy = f32::NEG_INFINITY;
        let mut minz = f32::INFINITY;
        let mut maxz = f32::NEG_INFINITY;
        for v in shadow_bounds_verts {
            let p = Vec3::new(v.pos[0], v.pos[1], v.pos[2]);
            let rel = p - shadow_center;
            let x = rel.dot(shadow_right);
            let y = rel.dot(shadow_up);
            let z = rel.dot(shadow_fwd);
            minx = minx.min(x);
            maxx = maxx.max(x);
            miny = miny.min(y);
            maxy = maxy.max(y);
            minz = minz.min(z);
            maxz = maxz.max(z);
        }
        shadow_half_w = ((maxx - minx) * 0.5 + 2.0).max(1.0);
        shadow_half_h = ((maxy - miny) * 0.5 + 2.0).max(1.0);
        shadow_near = minz - 4.0;
        shadow_far = (maxz + 4.0).max(shadow_near + 1.0);

        let texel_w = (shadow_half_w * 2.0) / (shadow_res as f32).max(1.0);
        let texel_h = (shadow_half_h * 2.0) / (shadow_res as f32).max(1.0);
        if texel_w.is_finite() && texel_w > 0.0 && texel_h.is_finite() && texel_h > 0.0 {
            let center_x = shadow_center.dot(shadow_right);
            let center_y = shadow_center.dot(shadow_up);
            let snapped_x = (center_x / texel_w).round() * texel_w;
            let snapped_y = (center_y / texel_h).round() * texel_h;
            shadow_center +=
                shadow_right * (snapped_x - center_x) + shadow_up * (snapped_y - center_y);
        }
    }

    ShadowFrame {
        center: shadow_center,
        half_w: shadow_half_w,
        half_h: shadow_half_h,
        near: shadow_near,
        far: shadow_far,
        right: shadow_right,
        up: shadow_up,
        forward: shadow_fwd,
    }
}
