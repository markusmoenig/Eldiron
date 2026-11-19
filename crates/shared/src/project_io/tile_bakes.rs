//! Archive-only pixel buffers. The Project retains the usual tile and sprite
//! metadata, and legacy archives still carry pixels inline.
use super::{ProjectBinaryCompression, ProjectBinaryReader, ProjectBinaryWriter};
use crate::project::Project;
use rusterix::Tile;
use std::collections::BTreeSet;
use theframework_core::prelude::Uuid;

const ROOT: &str = "binaries/tile-bakes/v1";
const INDEX_LIMIT: u64 = 16 * 1024 * 1024;
const TILE_LIMIT: u64 = 256 * 1024 * 1024;

fn tile_path(id: Uuid) -> String {
    format!("{ROOT}/{id}.bin")
}

// The archive metadata specifies each frame, its dimensions, and the presence
// of extra channels. Each plane is a little-endian length followed by raw bytes.
fn planes(tile: &mut Tile) -> impl Iterator<Item = &mut Vec<u8>> {
    let (frames, surfaces) = match &mut tile.isometric {
        Some(layout) => (layout.frames.as_mut_slice(), layout.surfaces.as_mut_slice()),
        None => (&mut [][..], &mut [][..]),
    };
    tile.textures
        .iter_mut()
        .chain(frames)
        .flat_map(|texture| std::iter::once(&mut texture.data).chain(texture.data_ext.iter_mut()))
        .chain(&mut tile.baked_material_data)
        .chain(surfaces.iter_mut().flat_map(|surface| {
            std::iter::once(&mut surface.data)
                .chain(surface.shadow.iter_mut().map(|shadow| &mut shadow.data))
        }))
}

pub(super) fn externalize(
    project: &mut Project,
    binaries: &mut ProjectBinaryWriter,
) -> Result<Option<String>, String> {
    let mut ids = Vec::new();
    for (&id, tile) in &mut project.tiles {
        if tile.uv_pages.is_none()
            && tile.isometric.is_none()
            && tile.baked_material_data.is_empty()
        {
            continue;
        }
        let mut payload = Vec::new();
        for plane in planes(tile) {
            if payload.len() as u64 + 4 + plane.len() as u64 > TILE_LIMIT {
                return Err(format!(
                    "tile bake {id} exceeds the archive entry size limit"
                ));
            }
            payload.extend_from_slice(&(plane.len() as u32).to_le_bytes());
            payload.extend(std::mem::take(plane));
        }
        if !payload.is_empty() {
            binaries.add(tile_path(id), payload, ProjectBinaryCompression::Deflated)?;
            ids.push(id);
        }
    }
    if ids.is_empty() {
        return Ok(None);
    }
    let index = serde_json::to_vec(&ids).map_err(|error| error.to_string())?;
    if index.len() as u64 > INDEX_LIMIT {
        return Err("tile bake index exceeds the archive size limit".into());
    }
    binaries
        .add(
            format!("{ROOT}/index.json"),
            index,
            ProjectBinaryCompression::Deflated,
        )
        .map(Some)
}

pub(super) fn hydrate(
    project: &mut Project,
    binaries: &mut ProjectBinaryReader<'_, '_>,
    index_path: &str,
) -> Result<(), String> {
    if index_path != format!("{ROOT}/index.json") {
        return Err(format!("unknown tile bake index: {index_path}"));
    }
    let ids: Vec<Uuid> = serde_json::from_slice(&binaries.read(index_path, INDEX_LIMIT)?)
        .map_err(|error| format!("invalid tile bake index: {error}"))?;
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("duplicate tile bake {id}"));
        }
        let tile = project
            .tiles
            .get_mut(&id)
            .ok_or_else(|| format!("unknown baked tile {id}"))?;
        let bytes = binaries.read(&tile_path(id), TILE_LIMIT)?;
        let mut payload = bytes.as_slice();
        for plane in planes(tile) {
            let (length, rest) = payload
                .split_first_chunk::<4>()
                .ok_or_else(|| format!("tile bake {id} is missing a plane length"))?;
            let length = u32::from_le_bytes(*length) as usize;
            let (pixels, rest) = rest
                .split_at_checked(length)
                .ok_or_else(|| format!("tile bake {id} has a truncated pixel plane"))?;
            if !plane.is_empty() {
                return Err(format!("tile bake {id} also contains inline pixels"));
            }
            *plane = pixels.to_vec();
            payload = rest;
        }
        if !payload.is_empty() {
            return Err(format!("tile bake {id} contains unexpected data"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_io::{
        PROJECT_JSON_PATH, PROJECT_MANIFEST_PATH, ProjectArchiveManifest, decode_project,
        encode_project,
    };
    use rusterix::{Texture, TileUvPages, isometric::SpriteLayout};
    use std::io::{Cursor, Read, Write};
    use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

    #[test]
    fn terrain_and_sprite_planes_round_trip_through_binary_entries() {
        let mut project = Project::new();
        let mut terrain = Tile::from_textures(vec![
            Texture::from_color([70, 90, 30, 255]),
            Texture::from_color([90, 70, 30, 255]),
        ]);
        terrain.textures[0].set_normal(0, 0, 0.25, -0.5);
        terrain.baked_material_data = vec![vec![0x91, 0x24], vec![0x43, 0x88]];
        terrain.uv_pages = Some(TileUvPages {
            animation_frames: 1,
            repeat_period: [0.; 2],
            origin: [0; 2],
            size: [2, 1],
            axes: [[0.; 3]; 2],
            units_per_page: 1,
        });
        let mut sprite = Tile::from_texture(Texture::from_color([10, 20, 30, 255]));
        let mut surface = rusterix::isometric::surface::SurfaceFrame::new(1);
        surface.data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert!(surface.set_baked_ambient_occlusion(&[0.2]));
        surface.shadow = Some(rusterix::isometric::surface::ShadowGrid {
            layers: 1,
            emissive: false,
            min: [0.; 3],
            max: [1.; 3],
            width: 1,
            height: 1,
            data: vec![7; 8],
        });
        sprite.isometric = Some(SpriteLayout {
            anchor: [3, 4],
            frames: vec![Texture::from_color([70, 80, 90, 128])],
            surfaces: vec![surface],
            ..Default::default()
        });
        let bitmap = Tile::from_texture(Texture::from_color([100, 110, 120, 255]));
        for tile in [&terrain, &sprite, &bitmap] {
            project.tiles.insert(tile.id, tile.clone());
        }

        let encoded = encode_project(&project).unwrap();
        let mut archive = ZipArchive::new(Cursor::new(&encoded)).unwrap();
        for tile in [&terrain, &sprite] {
            assert!(archive.by_name(&tile_path(tile.id)).is_ok());
        }
        assert!(archive.by_name(&tile_path(bitmap.id)).is_err());
        let json: serde_json::Value =
            serde_json::from_reader(archive.by_name(PROJECT_JSON_PATH).unwrap()).unwrap();
        assert_eq!(
            json["tiles"][terrain.id.to_string()]["textures"][0]["data"],
            serde_json::json!([])
        );
        assert_eq!(
            json["tiles"][sprite.id.to_string()]["isometric"]["surfaces"][0]["data"],
            ""
        );
        assert_eq!(
            json["tiles"][sprite.id.to_string()]["isometric"]["surfaces"][0]["has_baked_ao"],
            true
        );

        let reopened = decode_project(&encoded).unwrap();
        assert_eq!(reopened.tiles[&terrain.id], terrain);
        assert_eq!(reopened.tiles[&sprite.id], sprite);
        assert_eq!(reopened.tiles[&bitmap.id], bitmap);
        let saved_again = encode_project(&reopened).unwrap();
        assert_eq!(decode_project(&saved_again).unwrap().tiles, reopened.tiles);
    }

    #[test]
    fn legacy_inline_tile_bakes_load_and_resave() {
        let mut project = Project::new();
        let mut tile = Tile::from_texture(Texture::from_color([8, 9, 10, 255]));
        tile.baked_material_data = vec![vec![2, 3]];
        tile.isometric = Some(SpriteLayout {
            frames: vec![Texture::from_color([5, 6, 7, 128])],
            ..Default::default()
        });
        project.tiles.insert(tile.id, tile.clone());
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        for (path, data) in [
            (
                PROJECT_MANIFEST_PATH,
                serde_json::to_vec(&ProjectArchiveManifest::default()).unwrap(),
            ),
            (PROJECT_JSON_PATH, serde_json::to_vec(&project).unwrap()),
        ] {
            archive
                .start_file(path, SimpleFileOptions::default())
                .unwrap();
            archive.write_all(&data).unwrap();
        }
        let legacy = archive.finish().unwrap().into_inner();
        let decoded = decode_project(&legacy).unwrap();
        assert_eq!(decoded.tiles[&tile.id], tile);
        let bytes = encode_project(&decoded).unwrap();
        assert_eq!(decode_project(&bytes).unwrap().tiles[&tile.id], tile);
    }

    #[test]
    fn rejects_truncated_tile_bake_payload() {
        let mut project = Project::new();
        let mut tile = Tile::from_texture(Texture::from_color([20, 50, 70, 255]));
        tile.baked_material_data = vec![vec![1, 2]];
        let id = tile.id;
        project.tiles.insert(id, tile);
        let encoded = encode_project(&project).unwrap();
        let mut source = ZipArchive::new(Cursor::new(encoded)).unwrap();
        let mut corrupt = ZipWriter::new(Cursor::new(Vec::new()));
        for index in 0..source.len() {
            let mut entry = source.by_index(index).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            if entry.name() == tile_path(id) {
                bytes.pop().unwrap();
            }
            corrupt
                .start_file(entry.name(), SimpleFileOptions::default())
                .unwrap();
            corrupt.write_all(&bytes).unwrap();
        }
        let corrupt = corrupt.finish().unwrap().into_inner();
        assert!(
            decode_project(&corrupt)
                .unwrap_err()
                .contains("truncated pixel plane")
        );
    }
}
