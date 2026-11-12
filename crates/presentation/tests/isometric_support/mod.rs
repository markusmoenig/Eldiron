use eldiron_presentation::isometric::{GpuMaterialSource, SurfaceKind, SurfaceTriangle};
use renderkit::{GpuContext, ReadbackTicket, wgpu};
use rusterix::{
    Map, PixelSource, Texture, Tile, TileUvPages, Value, Vertex, isometric::SpriteLayout,
};
use scenevm::isometric::SpriteContacts;
use std::sync::Arc;
use theframework::prelude::Uuid;

pub(super) fn capture(gpu: &GpuContext, image: Arc<renderkit::GpuImage>) -> Vec<u8> {
    let mut c = ReadbackTicket::capture(gpu, image).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    c.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}
pub(super) fn material(color: [u8; 3], spatial: bool) -> Tile {
    let n = if spatial { 8 } else { 2 };
    let textures = (0..n)
        .map(|page| {
            let data = (0..16)
                .flat_map(|i| {
                    [
                        color[0].saturating_add(page * 7),
                        color[1].saturating_add(i * 3),
                        color[2],
                        255,
                    ]
                })
                .collect();
            let mut tex = Texture::new(data, 4, 4);
            tex.set_materials_all(0.4 + page as f32 * 0.04, 0.1, 1., 0.1);
            tex
        })
        .collect::<Vec<_>>();
    let mut tile = Tile::from_texture(textures[0].clone());
    tile.baked_material_data = textures.iter().map(Texture::material_bytes).collect();
    tile.textures = textures;
    tile.isometric = Some(SpriteLayout {
        ground: true,
        pixels_per_unit: 16.,
        frames: vec![Texture::from_color([255; 4])],
        ..Default::default()
    });
    if spatial {
        tile.uv_pages = Some(TileUvPages {
            origin: [-1, -1],
            size: [2, 2],
            units_per_page: 1,
            animation_frames: 2,
            repeat_period: [0.; 2],
            axes: [[0.; 3]; 2],
        });
    }
    tile
}
#[allow(dead_code)] // Some integration suites exercise terrain without floor sectors.
pub(super) fn sector(map: &mut Map, id: Uuid) -> u32 {
    for (i, p) in [[-1., -1.], [2., -1.], [2., 1.], [-1., 1.]]
        .into_iter()
        .enumerate()
    {
        map.vertices.push(Vertex::new(i as u32, p[0], p[1]));
    }
    for i in 0..4 {
        map.create_linedef_manual(i, (i + 1) % 4);
    }
    let sector = map.close_polygon_manual().unwrap();
    map.find_sector_mut(sector)
        .unwrap()
        .properties
        .set("source", Value::Source(PixelSource::TileId(id)));
    sector
}
pub(super) struct Prepared {
    pub(super) vertices: [[f32; 3]; 3],
    pub(super) source: Arc<GpuMaterialSource>,
    pub(super) secondary: Option<(Arc<GpuMaterialSource>, [f32; 4])>,
    pub(super) cell: Option<[i32; 2]>,
}
pub(super) fn triangles<'a>(
    prepared: &'a [Prepared],
    kind: SurfaceKind,
    repeat: bool,
    bounds: [f32; 4],
) -> Vec<SurfaceTriangle<'a>> {
    prepared
        .iter()
        .map(|t| SurfaceTriangle {
            vertices: t.vertices,
            source: &t.source,
            secondary: t.secondary.as_ref().map(|(s, w)| (s.as_ref(), *w)),
            paint: None,
            kind,
            repeat,
            uv_bounds: bounds,
            clip_cell: t.cell,
            contacts: SpriteContacts {
                owner: 0,
                protected: &[],
            },
        })
        .collect()
}
pub(super) fn mismatch(actual: &[u8], expected: &[u8], label: &str) {
    if actual != expected {
        let at = actual
            .iter()
            .zip(expected)
            .position(|(a, b)| a != b)
            .unwrap();
        let n = actual.iter().zip(expected).filter(|(a, b)| a != b).count();
        panic!(
            "{label}: first different byte {at}, GPU {}, reference {}, {n} total differing channels",
            actual[at], expected[at]
        );
    }
}

pub(super) fn save_png(path: &str, size: (u32, u32), pixels: &[u8]) {
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), size.0, size.1);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
}
