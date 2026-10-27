//! Read-only conversion of retired tile groups to ordinary tile assets.
//! Group types never enter the live project or renderer model.
use indexmap::IndexMap;
use rusterix::{Texture, Tile};
use serde::Deserialize;
use serde_json::{Value, json};
use theframework::prelude::Uuid;

#[derive(Deserialize)]
struct LegacyMember {
    tile_id: Uuid,
    x: u16,
    y: u16,
}

#[derive(Deserialize)]
struct LegacyGroup {
    #[serde(default)]
    name: String,
    width: u16,
    height: u16,
    #[serde(default)]
    members: Vec<LegacyMember>,
}

/// Used by project decoding and local collection import before typed decoding.
pub fn migrate(value: &mut Value) -> Result<(), String> {
    let Some(root) = value.as_object_mut() else {
        return Ok(());
    };
    let groups = root.remove("tile_groups").unwrap_or_else(|| json!({}));
    let groups: IndexMap<Uuid, LegacyGroup> =
        serde_json::from_value(groups).map_err(|err| err.to_string())?;
    if groups.is_empty() {
        root.remove("tile_board_groups");
        return Ok(());
    }
    let tiles = root.entry("tiles").or_insert_with(|| json!({}));
    let tiles: IndexMap<Uuid, Tile> =
        serde_json::from_value(tiles.clone()).map_err(|err| err.to_string())?;
    let output = root
        .get_mut("tiles")
        .and_then(Value::as_object_mut)
        .ok_or("Invalid legacy tile assets.")?;
    for (id, group) in &groups {
        if tiles.contains_key(id) {
            return Err(format!("Legacy tile group {id} conflicts with a tile ID."));
        }
        let tile = flatten_group(*id, group, &tiles)?;
        output.insert(
            id.to_string(),
            serde_json::to_value(tile).map_err(|err| err.to_string())?,
        );
    }
    migrate_board(root);
    if let Some(collections) = root
        .get_mut("tile_collections")
        .and_then(Value::as_object_mut)
    {
        for collection in collections.values_mut() {
            migrate_collection(collection, &groups)?;
        }
    }
    if let Some(collection) = root.get_mut("collection") {
        migrate_collection(collection, &groups)?;
    }
    rewrite_sources(value, &groups)
}

fn migrate_board(object: &mut serde_json::Map<String, Value>) {
    if let Some(Value::Object(positions)) = object.remove("tile_board_groups") {
        if let Some(tiles) = object
            .entry("tile_board_tiles")
            .or_insert_with(|| json!({}))
            .as_object_mut()
        {
            for (id, pos) in positions {
                tiles.entry(id).or_insert(pos);
            }
        }
    }
}

fn migrate_collection(
    value: &mut Value,
    groups: &IndexMap<Uuid, LegacyGroup>,
) -> Result<(), String> {
    let Some(collection) = value.as_object_mut() else {
        return Ok(());
    };
    // A flattened group remains one ordinary image tile. Its members also stay
    // available as independent assets, without continuing group navigation.
    if let Some(entries) = collection.get_mut("entries").and_then(Value::as_array_mut) {
        for entry in entries {
            if let Some(id) = entry.get("TileGroup").and_then(Value::as_str) {
                let id: Uuid = id.parse::<Uuid>().map_err(|err| err.to_string())?;
                if !groups.contains_key(&id) {
                    return Err(format!("Missing legacy tile group {id}."));
                }
                *entry = json!({"SingleTile": id});
            }
        }
    }
    migrate_board(collection);
    Ok(())
}

fn rewrite_sources(value: &mut Value, groups: &IndexMap<Uuid, LegacyGroup>) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            if object.len() == 1 {
                if let Some(id) = object.get("TileGroup").and_then(Value::as_str) {
                    let id: Uuid = id.parse::<Uuid>().map_err(|err| err.to_string())?;
                    if !groups.contains_key(&id) {
                        return Err(format!("Missing legacy tile group {id}."));
                    }
                    *value = json!({"TileId": id});
                    return Ok(());
                }
                if let Some(member) = object.get("TileGroupMember") {
                    let id: Uuid = serde_json::from_value(member["group_id"].clone())
                        .map_err(|err| err.to_string())?;
                    let index = member["member_index"]
                        .as_u64()
                        .ok_or("Invalid legacy group member index.")?;
                    let tile = groups
                        .get(&id)
                        .and_then(|group| group.members.get(index as usize))
                        .ok_or_else(|| {
                            format!("Missing member {index} in legacy tile group {id}.")
                        })?;
                    *value = json!({"TileId": tile.tile_id});
                    return Ok(());
                }
            }
            for child in object.values_mut() {
                rewrite_sources(child, groups)?;
            }
        }
        Value::Array(array) => {
            for child in array {
                rewrite_sources(child, groups)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn flatten_group(
    id: Uuid,
    group: &LegacyGroup,
    tiles: &IndexMap<Uuid, Tile>,
) -> Result<Tile, String> {
    let members = group
        .members
        .iter()
        .map(|member| {
            tiles
                .get(&member.tile_id)
                .map(|tile| (member, tile))
                .ok_or_else(|| format!("Missing tile {} in legacy group {id}.", member.tile_id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let cell_width = members
        .iter()
        .flat_map(|(_, tile)| &tile.textures)
        .map(|t| t.width)
        .max()
        .unwrap_or(1)
        .max(1);
    let cell_height = members
        .iter()
        .flat_map(|(_, tile)| &tile.textures)
        .map(|t| t.height)
        .max()
        .unwrap_or(1)
        .max(1);
    let width = cell_width
        .checked_mul(usize::from(group.width.max(1)))
        .ok_or_else(|| format!("Legacy tile group {id} is too large to convert."))?;
    let height = cell_height
        .checked_mul(usize::from(group.height.max(1)))
        .ok_or_else(|| format!("Legacy tile group {id} is too large to convert."))?;
    // Preserve the full animation period when members have different lengths.
    let mut frames = 1usize;
    for (_, tile) in &members {
        let count = tile.textures.len().max(1);
        let (mut a, mut b) = (frames, count);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        frames = (frames / a)
            .checked_mul(count)
            .ok_or_else(|| format!("Legacy tile group {id} has an excessive animation period."))?;
    }
    if width > 8192
        || height > 8192
        || width
            .checked_mul(height)
            .and_then(|n| n.checked_mul(frames))
            .is_none_or(|n| n > 64 * 1024 * 1024)
    {
        return Err(format!("Legacy tile group {id} is too large to convert."));
    }
    let mut textures = Vec::with_capacity(frames);
    for frame in 0..frames {
        let mut texture = Texture::alloc(width, height);
        for (member, tile) in &members {
            if member.x >= group.width || member.y >= group.height {
                return Err(format!("Member outside legacy tile group {id}."));
            }
            if tile.textures.is_empty() {
                continue;
            }
            let index = frame % tile.textures.len();
            let mut source = tile.textures[index].clone();
            if source.width == 0
                || source.height == 0
                || source
                    .width
                    .checked_mul(source.height)
                    .and_then(|n| n.checked_mul(4))
                    != Some(source.data.len())
                || source
                    .data_ext
                    .as_ref()
                    .is_some_and(|ext| ext.len() != source.data.len())
            {
                return Err(format!("Invalid texture in legacy tile group {id}."));
            }
            if let Some(bytes) = tile.baked_material_data.get(index) {
                if !source.apply_material_bytes(bytes) {
                    return Err(format!("Invalid baked material in legacy tile group {id}."));
                }
            }
            let mut frame_tile = Tile::from_texture(source.clone());
            frame_tile.id = tile.id;
            frame_tile.material = tile.material.clone();
            frame_tile.baked_material_data = tile
                .baked_material_data
                .get(index)
                .cloned()
                .into_iter()
                .collect();
            source.data_ext = frame_tile.to_material_array().into_iter().next();
            for y in 0..cell_height {
                for x in 0..cell_width {
                    let src = ((y * source.height / cell_height) * source.width
                        + x * source.width / cell_width)
                        * 4;
                    let dst = ((usize::from(member.y) * cell_height + y) * width
                        + usize::from(member.x) * cell_width
                        + x)
                        * 4;
                    texture.data[dst..dst + 4].copy_from_slice(&source.data[src..src + 4]);
                    if let Some(ext) = &source.data_ext {
                        let output = texture
                            .data_ext
                            .get_or_insert_with(|| [0x88, 0x0f, 128, 128].repeat(width * height));
                        output[dst..dst + 4].copy_from_slice(&ext[src..src + 4]);
                    }
                }
            }
        }
        textures.push(texture);
    }
    let mut tile = Tile::from_textures(textures);
    tile.baked_material_data = tile.textures.iter().map(Texture::material_bytes).collect();
    tile.id = id;
    tile.alias = group.name.clone();
    if let Some((_, first)) = members.first() {
        tile.role = first.role;
    }
    tile.procedural.coverage = [
        u32::from(group.width.max(1)),
        u32::from(group.height.max(1)),
    ];
    Ok(tile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Project, TileCollectionAsset, TileCollectionEntry};
    use rusterix::{PixelSource, Value as MapValue};
    use theframework::prelude::Vec2;

    fn grouped_project() -> (Value, Uuid, Uuid, Uuid) {
        let mut project = Project::new();
        let left = Tile::from_textures(vec![
            Texture::from_color([255, 0, 0, 255]),
            Texture::from_color([0, 255, 0, 255]),
        ]);
        let mut right = Tile::from_texture(Texture::from_color([0, 0, 255, 255]));
        right.textures[0].set_materials_all(0.2, 0.4, 0.8, 0.6);
        right.textures[0].set_normal(0, 0, 0.25, -0.25);
        right.baked_material_data = vec![right.textures[0].material_bytes()];
        let (left_id, right_id) = (left.id, right.id);
        project.tiles.insert(left_id, left);
        project.tiles.insert(right_id, right);
        let group = Uuid::new_v4();
        project.regions[0]
            .map
            .properties
            .set("legacy", MapValue::Source(PixelSource::TileId(group)));
        let collection = TileCollectionAsset::new("Legacy".into());
        let collection_id = collection.id;
        project.tile_collections.insert(collection_id, collection);
        let mut value = serde_json::to_value(project).unwrap();
        value["tile_groups"] = json!({group.to_string(): {
            "id": group, "name": "Old layout", "width": 3, "height": 1,
            "members": [{"tile_id": left_id, "x": 0, "y": 0}, {"tile_id": right_id, "x": 2, "y": 0}]
        }});
        value["tile_board_groups"] = json!({group.to_string(): {"x": 4, "y": 2}});
        value["tile_collections"][collection_id.to_string()]["entries"] =
            json!([{"TileGroup": group}]);
        value["tile_collections"][collection_id.to_string()]["tile_board_groups"] =
            json!({group.to_string(): {"x": 1, "y": 2}});
        value["regions"][0]["map"]["properties"]["values"]["legacy"] =
            json!({"Source": {"TileGroup": group}});
        value["world_module"] =
            json!({"nested": [{"TileGroupMember": {"group_id": group, "member_index": 1}}]});
        (value, group, left_id, right_id)
    }

    #[test]
    fn legacy_groups_become_regular_tiles_and_survive_save_reload() {
        let (value, group, left, right) = grouped_project();
        let project: Project = serde_json::from_value(value).unwrap();
        assert!(project.tiles.contains_key(&left) && project.tiles.contains_key(&right));
        let tile = &project.tiles[&group];
        assert_eq!(tile.alias, "Old layout");
        assert_eq!(tile.textures.len(), 2);
        assert_eq!(
            tile.textures[0].data,
            [255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 255, 255]
        );
        assert_eq!(&tile.textures[1].data[..4], &[0, 255, 0, 255]);
        assert_eq!(project.tile_board_tiles[&group], Vec2::new(4, 2));
        assert_eq!(
            project.regions[0].map.properties.get_source("legacy"),
            Some(&PixelSource::TileId(group))
        );
        assert_eq!(project.world_module["nested"][0], json!({"TileId": right}));
        assert_eq!(
            project.tile_collections.values().next().unwrap().entries,
            vec![TileCollectionEntry::SingleTile(group)]
        );
        let serialized = serde_json::to_value(&project).unwrap();
        let text = serialized.to_string();
        assert!(!text.contains("TileGroup"));
        assert!(!text.contains("tile_groups"));
        assert!(!text.contains("tile_board_groups"));
        let restored: Project = serde_json::from_value(serialized).unwrap();
        assert_eq!(restored.tiles[&group], project.tiles[&group]);
        let mut expected = project.tiles[&right].textures[0].clone();
        expected.apply_material_bytes(&project.tiles[&right].baked_material_data[0]);
        let mut actual = restored.tiles[&group].textures[0].clone();
        actual.apply_material_bytes(&restored.tiles[&group].baked_material_data[0]);
        assert_eq!(actual.get_materials(2, 0), expected.get_materials(0, 0));
        assert_eq!(actual.get_normal(2, 0), expected.get_normal(0, 0));
    }

    #[test]
    fn invalid_legacy_member_is_rejected_instead_of_dropping_artwork() {
        let (mut value, group, _, _) = grouped_project();
        value["world_module"] = json!({"TileGroupMember": {"group_id": group, "member_index": 99}});
        let error = serde_json::from_value::<Project>(value).unwrap_err();
        assert!(error.to_string().contains("Missing member 99"));
    }

    #[test]
    fn different_member_animation_lengths_keep_their_full_period() {
        let (mut value, group, _, right) = grouped_project();
        let textures = value["tiles"][right.to_string()]["textures"]
            .as_array_mut()
            .unwrap();
        let frame = textures[0].clone();
        textures.extend([frame.clone(), frame]);
        // Material data in this fixture belongs to the first frame only.
        value["tiles"][right.to_string()]["baked_material_data"] = json!([]);
        let project: Project = serde_json::from_value(value).unwrap();
        assert_eq!(project.tiles[&group].textures.len(), 6);
        assert_eq!(
            &project.tiles[&group].textures[4].data[..4],
            &[255, 0, 0, 255]
        );
        assert_eq!(
            &project.tiles[&group].textures[5].data[..4],
            &[0, 255, 0, 255]
        );
    }
}
