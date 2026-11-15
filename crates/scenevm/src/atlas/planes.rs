//! Unbuilt packed atlas planes are metadata, not an eager pair of large images.
//! Explicit CPU exports preserve logical pixels without retaining a derived copy.
use super::*;

fn bytes(texture: &Texture) -> usize {
    (texture.width as usize)
        .checked_mul(texture.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .expect("atlas plane dimensions overflow")
}
pub(super) fn ensure_storage(inner: &mut SharedAtlasInner) {
    inner.atlas.data.resize(bytes(&inner.atlas), 0);
    inner
        .atlas_material
        .data
        .resize(bytes(&inner.atlas_material), 0);
}
pub(super) fn snapshot(texture: &Texture, fill: [u8; 4]) -> Vec<u8> {
    if !texture.data.is_empty() {
        return texture.data.clone();
    }
    let mut pixels = vec![0; bytes(texture)];
    if fill != [0; 4] {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&fill);
        }
    }
    pixels
}
pub(super) fn copy(texture: &Texture, fill: [u8; 4], dst: &mut [u8], width: u32, height: u32) {
    if !texture.data.is_empty() {
        texture.copy_to_slice(dst, width, height);
        return;
    }
    // Match Texture's clipped row-copy policy, including short destinations.
    let row_bytes = texture.width.min(width) as usize * 4;
    let stride = width as usize * 4;
    for row in 0..texture.height.min(height) as usize {
        let start = row * stride;
        let Some(destination) = dst.get_mut(start..start + row_bytes) else {
            break;
        };
        if fill == [0; 4] {
            destination.fill(0);
        } else {
            for pixel in destination.chunks_exact_mut(4) {
                pixel.copy_from_slice(&fill);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbuilt_planes_preserve_cpu_exports_without_retained_storage() {
        let atlas = SharedAtlas::new(3, 2);
        assert_eq!(atlas.dims(), (3, 2));
        assert_eq!(atlas.derived_cpu_plane_bytes(), 0);
        assert_eq!(atlas.atlas_pixels(), vec![0; 24]);
        assert_eq!(atlas.material_atlas_pixels(), vec![0; 24]);
        let mut expected = Texture::new(3, 2);
        for short in [false, true] {
            let len = if short { 4 * 4 + 3 } else { 4 * 3 * 4 };
            let mut actual = vec![77; len];
            let mut reference = actual.clone();
            atlas.copy_atlas_to_slice(&mut actual, 4, 3);
            expected.copy_to_slice(&mut reference, 4, 3);
            assert_eq!(actual, reference);
        }
        atlas.clear();
        fill_default_material_pixels(&mut expected.data);
        assert_eq!(atlas.material_atlas_pixels(), expected.data);
        let mut actual = vec![77; 4 * 3 * 4];
        let mut reference = actual.clone();
        atlas.copy_material_atlas_to_slice(&mut actual, 4, 3);
        expected.copy_to_slice(&mut reference, 4, 3);
        assert_eq!(actual, reference);
        assert_eq!(atlas.derived_cpu_plane_bytes(), 0);
        atlas.resize(7, 5);
        assert_eq!(atlas.dims(), (7, 5));
        assert_eq!(atlas.material_atlas_pixels(), vec![0; 7 * 5 * 4]);
        assert_eq!(atlas.derived_cpu_plane_bytes(), 0);
        atlas.ensure_built();
        assert_eq!(
            atlas.material_atlas_pixels(),
            default_material_frame(7 * 5 * 4)
        );
    }

    #[test]
    fn automatic_layout_allocates_actual_planes_instead_of_the_initial_hint() {
        let atlas = SharedAtlas::new(4096, 4096);
        let shared = atlas.clone();
        assert_eq!(atlas.derived_cpu_plane_bytes(), 0);
        let tile = Uuid::new_v4();
        atlas.add_tile(tile, 1, 1, vec![vec![11, 22, 33, 255]], vec![]);
        assert_eq!(atlas.derived_cpu_plane_bytes(), 0);
        assert!(atlas.ensure_built());
        assert_eq!(atlas.dims(), (256, 256));
        assert_eq!(atlas.derived_cpu_plane_bytes(), 256 * 256 * 8);
        assert_eq!(
            shared.derived_cpu_plane_bytes(),
            atlas.derived_cpu_plane_bytes()
        );
        let rect = atlas.frame_rect(&tile, 0).unwrap();
        let offset = (rect.y * 256 + rect.x) as usize * 4;
        assert_eq!(
            &atlas.atlas_pixels()[offset..offset + 4],
            &[11, 22, 33, 255]
        );
    }
}
