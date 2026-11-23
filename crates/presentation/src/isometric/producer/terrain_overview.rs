//! Broad views show a bounded terrain mesh and the authored floor outlines.
use super::*;
use rusterix::{Texture, chunkbuilder::terrain_surface::TerrainSurface, isometric::ScenePlan};
use theframework_core::Uuid;

const OVERVIEW_TRIANGLE_BUDGET: usize = 2048;
type Swatches = HashMap<Uuid, Option<Arc<GpuMaterialSource>>>;

impl IsometricSceneRenderer {
    pub(super) fn prepare_terrain_overview(
        &mut self,
        input: &IsometricSceneInput<'_>,
        plan: &ScenePlan<'_>,
        model: &TerrainSurface,
        visible: BBox,
        size: (u32, u32),
        swatches: &mut Swatches,
    ) -> Result<Vec<PreparedSurface>> {
        let policy = SurfaceMaterials::for_terrain(input.map, input.assets);
        let mut triangles = Vec::new();
        model.preview_triangles(visible, OVERVIEW_TRIANGLE_BUDGET, |tri| triangles.push(tri));
        let mut surfaces = Vec::new();
        for tri in triangles {
            let center = (tri[0] + tri[1] + tri[2]) / 3.;
            let Some(material) = policy.cell((center.x.floor() as i32, center.z.floor() as i32)) else {
                continue;
            };
            if let Some(source) = self.overview_swatch(input, material.primary, swatches)? {
                surfaces.push(overview_surface(tri.map(|v| v.into_array()), source, SurfaceKind::Terrain));
            }
        }
        // Thin roads and watercourses can be narrower than a sampled terrain
        // cell. Keep their polygon coverage at every overview scale.
        let mut floors = plan.floors.clone();
        floors.sort_by_key(|(sector, _)| sector.layer.unwrap_or(0));
        for (sector, tile) in floors {
            surfaces.extend(self.prepare_overview_floor(input, sector, tile, size, swatches)?);
        }
        Ok(surfaces)
    }

    pub(super) fn prepare_overview_floor(
        &mut self,
        input: &IsometricSceneInput<'_>,
        sector: &Sector,
        tile: &Tile,
        size: (u32, u32),
        swatches: &mut Swatches,
    ) -> Result<Vec<PreparedSurface>> {
        let height = sector.properties.get_float_default("floor_height", 0.);
        let visible = input.projection.visible_bounds(size, [height; 2]);
        if !visible.intersects(&sector.bounding_box(input.map)) {
            return Ok(Vec::new());
        }
        let Some((points, indices)) = sector.generate_geometry(input.map) else {
            return Ok(Vec::new());
        };
        let Some(source) = self.overview_swatch(input, tile, swatches)? else {
            return Ok(Vec::new());
        };
        Ok(indices.into_iter().map(|(a, b, c)| {
            let vertices = [a, b, c].map(|i| [points[i][0], height, points[i][1]]);
            overview_surface(vertices, source.clone(), SurfaceKind::Floor)
        }).collect())
    }

    fn overview_swatch(
        &mut self,
        input: &IsometricSceneInput<'_>,
        tile: &Tile,
        swatches: &mut Swatches,
    ) -> Result<Option<Arc<GpuMaterialSource>>> {
        if let Some(source) = swatches.get(&tile.id) {
            return Ok(source.clone());
        }
        let source = swatch_color(tile).map(|color| {
            let mut swatch = Tile::from_texture(Texture::from_color(color));
            swatch.id = tile.id;
            self.overview_materials.prepare(&swatch, 0, input.source_version)
        }).transpose()?;
        swatches.insert(tile.id, source.clone());
        Ok(source)
    }
}

fn swatch_color(tile: &Tile) -> Option<[u8; 4]> {
    let texture = tile.textures.first()?;
    if texture.width == 0 || texture.height == 0
        || texture.width.checked_mul(texture.height)?.checked_mul(4)? > texture.data.len()
    {
        return None;
    }
    const STEPS: usize = 8;
    let mut weight = 0u32;
    let mut rgb = [0u32; 3];
    for y in 0..STEPS {
        for x in 0..STEPS {
            let [r, g, b, a] = texture.get_pixel(
                ((2 * x + 1) * texture.width / (2 * STEPS)) as u32,
                ((2 * y + 1) * texture.height / (2 * STEPS)) as u32,
            );
            let alpha = u32::from(a);
            weight += alpha;
            for (sum, channel) in rgb.iter_mut().zip([r, g, b]) {
                *sum += u32::from(channel) * alpha;
            }
        }
    }
    (weight > 0).then(|| {
        let [r, g, b] = rgb.map(|channel| (channel / weight) as u8);
        [r, g, b, (weight / (STEPS * STEPS) as u32) as u8]
    })
}

fn overview_surface(
    vertices: [[f32; 3]; 3],
    source: Arc<GpuMaterialSource>,
    kind: SurfaceKind,
) -> PreparedSurface {
    PreparedSurface {
        vertices,
        source,
        secondary: None,
        paint: None,
        kind,
        repeat: true,
        bounds: [0.; 4],
        cell: None,
    }
}
