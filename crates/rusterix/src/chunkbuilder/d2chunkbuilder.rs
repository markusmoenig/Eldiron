use crate::{Assets, Batch2D, Chunk, ChunkBuilder, Map, PixelSource, Value};
use scenevm::GeoId;
use vek::Vec2;

pub struct D2ChunkBuilder {}

impl Clone for D2ChunkBuilder {
    fn clone(&self) -> Self {
        D2ChunkBuilder {}
    }
}

impl ChunkBuilder for D2ChunkBuilder {
    fn new() -> Self {
        Self {}
    }

    fn boxed_clone(&self) -> Box<dyn ChunkBuilder> {
        Box::new(self.clone())
    }

    fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
    ) {
        let sectors = map.sorted_sectors_by_area();
        for sector in &sectors {
            if !sector.intersects_vertical_slice(map, 0.0, 1.0) {
                continue;
            }

            let bbox = sector.bounding_box(map);

            // Collect occluded sectors and store them in the chunk
            let occlusion = sector.properties.get_float_default("occlusion", 1.0);
            if occlusion < 1.0 {
                let mut occl_bbox = bbox.clone();
                occl_bbox.expand(Vec2::new(0.1, 0.1));
                chunk.occluded_sectors.push((occl_bbox, occlusion));
            }

            if bbox.intersects(&chunk.bbox) && chunk.bbox.contains(bbox.center()) {
                if let Some(geo) = sector.generate_geometry(map) {
                    let mut vertices: Vec<[f32; 2]> = vec![];
                    let mut uvs: Vec<[f32; 2]> = vec![];

                    let mut repeat = true;
                    if sector.properties.get_int_default("tile_mode", 1) == 0 {
                        repeat = false;
                    }

                    let source = sector.properties.get_default_source();
                    for vertex in &geo.0 {
                        let local = Vec2::new(vertex[0], vertex[1]);

                        if !repeat {
                            let uv = [
                                (vertex[0] - bbox.min.x) / (bbox.max.x - bbox.min.x),
                                (vertex[1] - bbox.min.y) / (bbox.max.y - bbox.min.y),
                            ];
                            uvs.push(uv);
                        } else {
                            let texture_scale = 1.0;
                            let uv = [
                                (vertex[0] - bbox.min.x) / texture_scale,
                                (vertex[1] - bbox.min.y) / texture_scale,
                            ];
                            uvs.push(uv);
                        }
                        vertices.push([local.x, local.y]);
                    }

                    if let Some(pixelsource) = source {
                        if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                            vmchunk.add_poly_2d(
                                GeoId::Sector(sector.id),
                                tile.id,
                                vertices.clone(),
                                uvs.clone(),
                                geo.1.clone(),
                                0,
                                true,
                            );
                        }
                    }
                }
            }
        }

        // Walls
        for sector in &sectors {
            let bbox = sector.bounding_box(map);
            if bbox.intersects(&chunk.bbox) && chunk.bbox.contains(bbox.center()) {
                if let Some(hash) = sector.generate_wall_geometry_by_linedef(map) {
                    for (linedef_id, geo) in hash.iter() {
                        let mut source = None;

                        if let Some(linedef) = map.find_linedef(*linedef_id) {
                            if let Some(Value::Source(pixelsource)) =
                                linedef.properties.get("row1_source")
                            {
                                source = Some(pixelsource);
                            }
                        }

                        let mut vertices: Vec<[f32; 2]> = vec![];
                        let mut uvs: Vec<[f32; 2]> = vec![];
                        let bbox = sector.bounding_box(map);

                        let repeat = true;

                        if let Some(pixelsource) = source {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                for vertex in &geo.0 {
                                    let local = Vec2::new(vertex[0], vertex[1]);

                                    if !repeat {
                                        let uv = [
                                            (vertex[0] - bbox.min.x) / (bbox.max.x - bbox.min.x),
                                            (vertex[1] - bbox.min.y) / (bbox.max.y - bbox.min.y),
                                        ];
                                        uvs.push(uv);
                                    } else {
                                        let texture_scale = 1.0;
                                        let uv = [
                                            (vertex[0] - bbox.min.x) / texture_scale,
                                            (vertex[1] - bbox.min.y) / texture_scale,
                                        ];
                                        uvs.push(uv);
                                    }
                                    vertices.push([local.x, local.y]);
                                }

                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    let batch = Batch2D::new(vertices, geo.1.clone(), uvs)
                                        .repeat_mode(if repeat {
                                            crate::RepeatMode::RepeatXY
                                        } else {
                                            crate::RepeatMode::ClampXY
                                        })
                                        .source(PixelSource::StaticTileIndex(texture_index));
                                    chunk.batches2d.push(batch);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add standalone walls
        for linedef in &map.linedefs {
            let bbox = linedef.bounding_box(map);
            if bbox.intersects(&chunk.bbox)
                && chunk.bbox.contains(bbox.center())
                && linedef.sector_ids.is_empty()
                && linedef.properties.get_float_default("wall_width", 0.0) > 0.0
            {
                if let Some(hash) =
                    crate::map::geometry::generate_line_segments_d2(map, &[linedef.id])
                {
                    for (_linedef_id, geo) in hash.iter() {
                        let mut source = None;

                        if let Some(Value::Source(pixelsource)) =
                            linedef.properties.get("row1_source")
                        {
                            source = Some(pixelsource);
                        }

                        let mut vertices: Vec<[f32; 2]> = vec![];
                        let mut uvs: Vec<[f32; 2]> = vec![];

                        if let Some(pixelsource) = source {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    for vertex in &geo.0 {
                                        let local = Vec2::new(vertex[0], vertex[1]);

                                        let texture_scale = 1.0;
                                        let uv = [
                                            (vertex[0]) / texture_scale,
                                            (vertex[1]) / texture_scale,
                                        ];
                                        uvs.push(uv);
                                        vertices.push([local.x, local.y]);
                                    }

                                    let batch = Batch2D::new(vertices, geo.1.clone(), uvs)
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .source(PixelSource::StaticTileIndex(texture_index));
                                    chunk.batches2d.push(batch);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
