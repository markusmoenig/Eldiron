//! Pure staged map operations. Publication and undo belong to scepter_authoring.
use super::scepter_authoring::{parse_id, region_index};
use super::*;
use eldiron_scepter::*;
use rusterix::{GeometryObject, GeometryObjectKind, PixelSource, WallAssembly, WallOpeningShape};
use serde_json::{Value as Json, json};
use shared::material_composition::{CompositionSurface, MapMaterialBinding};

pub(super) fn point(p: [f32; 3]) -> Result<Vec3<f32>, String> {
    if p.iter().any(|v| !v.is_finite() || v.abs() > 1_000_000.) {
        return Err("World coordinates must be finite and within ±1000000".into());
    }
    Ok(Vec3::from(p))
}
fn name(s: &str) -> Result<(), String> {
    if s.is_empty() || s.len() > 256 {
        Err("Names must be 1..256 bytes".into())
    } else {
        Ok(())
    }
}
pub(super) fn tile(project: &Project, selector: &TileSelector) -> Result<Uuid, String> {
    let matches = project
        .tiles
        .values()
        .filter(|t| {
            selector
                .id
                .as_ref()
                .is_none_or(|id| t.id.to_string() == *id)
                && selector.alias.as_ref().is_none_or(|a| {
                    t.alias
                        .split([',', ';', '\n'])
                        .any(|s| s.trim().eq_ignore_ascii_case(a))
                })
                && selector
                    .role
                    .as_ref()
                    .is_none_or(|r| t.role.to_string().eq_ignore_ascii_case(r))
                && selector
                    .kind
                    .as_ref()
                    .is_none_or(|k| t.procedural.kind.eq_ignore_ascii_case(k))
                && selector
                    .style
                    .as_ref()
                    .is_none_or(|k| t.procedural.style.eq_ignore_ascii_case(k))
                && selector
                    .tags
                    .iter()
                    .all(|tag| t.gameplay_tags.contains(tag))
        })
        .map(|t| t.id)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err("Tile selector must resolve to exactly one tile".into());
    }
    Ok(matches[0])
}
fn source(project: &Project, s: &Option<TileSelector>) -> Result<Option<PixelSource>, String> {
    s.as_ref()
        .map(|s| tile(project, s).map(PixelSource::TileId))
        .transpose()
}
fn fresh_id(project: &Project, id: &str) -> Result<Uuid, String> {
    let id = parse_id(id)?;
    if project.characters.contains_key(&id)
        || project.items.contains_key(&id)
        || project.tiles.contains_key(&id)
        || project.procedural_appearances.contains_key(&id)
        || project.regions.iter().any(|r| {
            r.characters.contains_key(&id)
                || r.items.contains_key(&id)
                || r.map.geometry_objects.iter().any(|o| o.id == id)
                || r.map.wall_assemblies.iter().any(|w| w.id == id)
        })
    {
        return Err("Destination UUID is already in use".into());
    }
    Ok(id)
}
fn polygon(
    map: &mut Map,
    points: &[[f32; 2]],
    name: &str,
    layer: Option<u8>,
) -> Result<u32, String> {
    if !(3..=256).contains(&points.len()) {
        return Err("A sector requires 3..256 vertices".into());
    }
    for p in points {
        point([p[0], 0., p[1]])?;
    }
    let area: f32 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum();
    if area.abs() < 0.0001 {
        return Err("Sector polygon has no area".into());
    }
    // Validate polygon simplicity; crossing edges would create misleading bounds/materials.
    fn cross(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    }
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            let n = points.len();
            if j == i + 1 || i == 0 && j == n - 1 {
                continue;
            }
            let (a, b, c, d) = (
                points[i],
                points[(i + 1) % n],
                points[j],
                points[(j + 1) % n],
            );
            if cross(a, b, c) * cross(a, b, d) <= 0. && cross(c, d, a) * cross(c, d, b) <= 0. {
                return Err("Sector polygon intersects itself".into());
            }
        }
    }
    let id = map
        .find_free_sector_id()
        .ok_or("Sector ID space exhausted")?;
    let mut vertices = Vec::new();
    for [x, z] in points {
        let vid = map
            .find_free_vertex_id()
            .ok_or("Vertex ID space exhausted")?;
        map.vertices.push(rusterix::Vertex::new(vid, *x, *z));
        vertices.push(vid);
    }
    let mut lines = vec![];
    for i in 0..vertices.len() {
        let lid = map
            .find_free_linedef_id()
            .ok_or("Line ID space exhausted")?;
        let mut l = rusterix::Linedef::new(lid, vertices[i], vertices[(i + 1) % vertices.len()]);
        l.sector_ids = vec![id];
        map.linedefs.push(l);
        lines.push(lid);
    }
    let mut sector = rusterix::Sector::new(id, lines);
    sector.name = name.into();
    sector.layer = layer;
    map.sectors.push(sector);
    let mut surface = rusterix::Surface::new(id);
    surface.extrusion.enabled = false;
    surface.calculate_geometry(map);
    map.surfaces.insert(surface.id, surface);
    map.update_surfaces();
    Ok(id)
}
fn bounds(min: [f32; 3], max: [f32; 3]) -> Result<(Vec3<f32>, Vec3<f32>), String> {
    let min = point(min)?;
    let max = point(max)?;
    if (max - min)
        .into_array()
        .iter()
        .any(|v| *v <= 0.0001 || *v > 4096.)
    {
        return Err("Geometry bounds require positive extents up to 4096 units".into());
    }
    Ok((min, max))
}
fn geometry_create(project: &mut Project, c: &GeometryCreate) -> Result<Json, String> {
    name(&c.name)?;
    let i = region_index(project, &c.region)?;
    let id = fresh_id(project, &c.id)?;
    let (min, max) = bounds(c.min, c.max)?;
    let material = source(project, &c.material)?;
    let mut object = match c.shape.as_str() {
        "box" | "floor" | "ceiling" => GeometryObject::box_from_bounds(&c.name, min, max),
        "cylinder" => {
            let segments = c.segments.unwrap_or(16);
            if !(3..=128).contains(&segments) {
                return Err("Cylinder segments must be 3..128".into());
            }
            GeometryObject::cylinder_from_bounds(&c.name, min, max, segments as usize, true)
        }
        _ => return Err("shape must be box, cylinder, floor or ceiling".into()),
    };
    object.id = id;
    for face in &mut object.faces {
        face.tile = material.clone();
    }
    let faces = object
        .faces
        .iter()
        .enumerate()
        .map(|(index, f)| json!({"index":index,"id":f.id}))
        .collect::<Vec<_>>();
    project.regions[i].map.geometry_objects.push(object);
    Ok(json!({"geometry":id,"faces":faces}))
}
fn wall_budget(wall: &WallAssembly) -> Result<(), String> {
    let mut bricks = 0_f32;
    for span in &wall.spans {
        let style = span.style_override.as_ref().unwrap_or(&wall.style);
        let length = wall
            .span_length(span.id)
            .ok_or("Wall contains a degenerate span")?;
        if length > 4096. || !(0.02..=16.).contains(&style.thickness) {
            return Err("Wall span or thickness exceeds the supported bounds".into());
        }
        bricks += (length / style.brick_width.max(0.02)).ceil()
            * (style.height / style.brick_height.max(0.02)).ceil();
    }
    if bricks > 25000. {
        return Err("Wall exceeds 25000 masonry cells; split it into smaller assemblies".into());
    }
    Ok(())
}

fn wall_create(project: &mut Project, c: &WallCreate) -> Result<Json, String> {
    name(&c.name)?;
    let i = region_index(project, &c.region)?;
    let id = fresh_id(project, &c.id)?;
    if !(2..=256).contains(&c.points.len()) || c.closed && c.points.len() < 3 {
        return Err("Walls require 2..256 points; closed walls require at least three".into());
    }
    if !(0.1..=256.).contains(&c.height) || !(0.02..=16.).contains(&c.thickness) {
        return Err("Invalid wall height or thickness".into());
    }
    let mut wall = WallAssembly::new(&c.name);
    wall.id = id;
    wall.style.height = c.height;
    wall.style.thickness = c.thickness;
    wall.style.stone_source = source(project, &c.material)?;
    let mut nodes = vec![];
    for p in &c.points {
        nodes.push(wall.add_node(point(*p)?));
    }
    let n = if c.closed {
        nodes.len()
    } else {
        nodes.len() - 1
    };
    for k in 0..n {
        wall.add_span(nodes[k], nodes[(k + 1) % nodes.len()])?;
    }
    wall_budget(&wall)?;
    let spans = wall.spans.iter().map(|s| s.id).collect::<Vec<_>>();
    let map = &mut project.regions[i].map;
    map.wall_assemblies.push(wall);
    map.rebuild_wall_geometry();
    Ok(json!({"wall":id,"spans":spans}))
}
fn opening_put(project: &mut Project, c: &OpeningPut) -> Result<Json, String> {
    let i = region_index(project, &c.region)?;
    let id = parse_id(&c.id)?;
    let assembly = parse_id(&c.assembly)?;
    let span = parse_id(&c.span)?;
    if ![c.center, c.bottom, c.width, c.height]
        .iter()
        .all(|v| v.is_finite())
        || c.width <= 0.05
        || c.height <= 0.05
        || c.bottom < 0.
    {
        return Err("Invalid opening dimensions".into());
    }
    let map = &mut project.regions[i].map;
    let wall = map
        .wall_assembly_mut(assembly)
        .ok_or("Wall assembly not found")?;
    let length = wall.span_length(span).ok_or("Wall span not found")?;
    let height = wall
        .span(span)
        .and_then(|s| s.style_override.as_ref())
        .unwrap_or(&wall.style)
        .height;
    if c.center - c.width / 2. < 0.
        || c.center + c.width / 2. > length
        || c.bottom + c.height > height
    {
        return Err("Opening must fit completely within its wall span".into());
    }
    if wall
        .spans
        .iter()
        .filter(|s| s.id != span)
        .any(|s| s.openings.iter().any(|o| o.id == id))
    {
        return Err("Opening ID belongs to another span".into());
    }
    wall.remove_opening(span, id);
    let created = wall.add_opening(
        span,
        Vec2::new(c.center - c.width / 2., c.bottom),
        Vec2::new(c.center + c.width / 2., c.bottom + c.height),
        if c.arched {
            WallOpeningShape::Arch
        } else {
            WallOpeningShape::Rectangular
        },
    )?;
    wall.opening_mut(span, created).unwrap().id = id;
    map.rebuild_wall_geometry();
    Ok(json!({"opening":id,"wall":assembly,"span":span}))
}
fn room_create(project: &mut Project, c: &RoomCreate) -> Result<Json, String> {
    let (min, max) = bounds(c.min, c.max)?;
    let height = max.y - min.y;
    if c.thickness * 2. >= (max.x - min.x).min(max.z - min.z) {
        return Err("Room wall thickness exceeds its size".into());
    }
    let wall = wall_create(
        project,
        &WallCreate {
            region: c.region.clone(),
            id: c.id.clone(),
            name: c.name.clone(),
            points: vec![
                [min.x, min.y, min.z],
                [max.x, min.y, min.z],
                [max.x, min.y, max.z],
                [min.x, min.y, max.z],
            ],
            closed: true,
            height,
            thickness: c.thickness,
            material: c.wall.clone(),
        },
    )?;
    let floor_id = Uuid::new_v4();
    geometry_create(
        project,
        &GeometryCreate {
            region: c.region.clone(),
            id: floor_id.to_string(),
            name: format!("{} Floor", c.name),
            shape: "floor".into(),
            min: [min.x, min.y - 0.05, min.z],
            max: [max.x, min.y, max.z],
            material: c.floor.clone(),
            segments: None,
        },
    )?;
    let i = region_index(project, &c.region)?;
    let floor = source(project, &c.floor)?;
    let sector = polygon(
        &mut project.regions[i].map,
        &[
            [min.x, min.z],
            [max.x, min.z],
            [max.x, max.z],
            [min.x, max.z],
        ],
        &c.name,
        Some(1),
    )?;
    let s = project.regions[i].map.find_sector_mut(sector).unwrap();
    s.properties.set("rect", Value::Bool(true));
    if let Some(source) = floor {
        s.properties.set("source", Value::Source(source));
    }
    let ceiling = if c.ceiling.is_some() {
        let id = Uuid::new_v4();
        geometry_create(
            project,
            &GeometryCreate {
                region: c.region.clone(),
                id: id.to_string(),
                name: format!("{} Ceiling", c.name),
                shape: "ceiling".into(),
                min: [min.x, max.y, min.z],
                max: [max.x, max.y + 0.05, max.z],
                material: c.ceiling.clone(),
                segments: None,
            },
        )?;
        Some(id)
    } else {
        None
    };
    let mut doors = vec![];
    if c.doors.len() > 64 {
        return Err("Maximum 64 openings per room".into());
    }
    for door in &c.doors {
        let span = wall["spans"]
            .get(door.wall)
            .and_then(Json::as_str)
            .ok_or("Room door wall index must be 0..3")?;
        doors.push(opening_put(
            project,
            &OpeningPut {
                region: c.region.clone(),
                assembly: c.id.clone(),
                span: span.into(),
                id: Uuid::new_v4().to_string(),
                center: door.center,
                bottom: door.bottom,
                width: door.width,
                height: door.height,
                arched: false,
            },
        )?);
    }
    Ok(
        json!({"wall":wall["wall"],"spans":wall["spans"],"floor":floor_id,"ceiling":ceiling,"sector":sector,"doors":doors}),
    )
}
fn entity_place(project: &mut Project, c: &EntityPlace) -> Result<Json, String> {
    let i = region_index(project, &c.region)?;
    let id = fresh_id(project, &c.id)?;
    let position = point(c.position)?;
    if c.data.as_ref().is_some_and(|d| d.len() > 65536) {
        return Err("Instance TOML exceeds 64 KiB".into());
    }
    match c.kind.as_str() {
        "character" => {
            let t = template_id(project, true, &c.template)?;
            let mut entity = shared::character::Character::default();
            entity.id = id;
            entity.character_id = t;
            entity.position = position;
            entity.name = c
                .name
                .clone()
                .unwrap_or_else(|| project.characters[&t].name.clone());
            name(&entity.name)?;
            entity.data = c.data.clone().unwrap_or_default();
            entity
                .data
                .parse::<toml::Table>()
                .map_err(|e| e.to_string())?;
            project.regions[i].characters.insert(id, entity);
        }
        "item" => {
            let t = template_id(project, false, &c.template)?;
            let mut item = shared::item::Item::default();
            item.id = id;
            item.item_id = t;
            item.position = position;
            item.name = c
                .name
                .clone()
                .unwrap_or_else(|| project.items[&t].name.clone());
            name(&item.name)?;
            item.data = c.data.clone().unwrap_or_default();
            item.data
                .parse::<toml::Table>()
                .map_err(|e| e.to_string())?;
            project.regions[i].items.insert(id, item);
        }
        _ => return Err("Entity kind must be character or item".into()),
    }
    if let Some(active) = c.active {
        let owner = shared::entity_graph::instance_key(project.regions[i].id, &c.kind, id);
        shared::entity_graph::set_owner_attribute(project, &owner, "active", json!(active))?;
    }
    Ok(json!({"id":id,"kind":c.kind,"position":c.position}))
}
fn template_id(project: &Project, character: bool, value: &str) -> Result<Uuid, String> {
    fn matches(id: Uuid, name: &str, data: &str, value: &str) -> bool {
        id.to_string() == value
            || name.eq_ignore_ascii_case(value)
            || data
                .parse::<toml::Table>()
                .ok()
                .and_then(|t| t.get("attributes").and_then(toml::Value::as_table).cloned())
                .is_some_and(|t| {
                    ["ruleset_id", "ruleset_path"]
                        .iter()
                        .any(|k| t.get(*k).and_then(toml::Value::as_str) == Some(value))
                })
    }
    let ids = if character {
        project
            .characters
            .values()
            .filter(|c| matches(c.id, &c.name, &c.data, value))
            .map(|c| c.id)
            .collect::<Vec<_>>()
    } else {
        project
            .items
            .values()
            .filter(|c| matches(c.id, &c.name, &c.data, value))
            .map(|c| c.id)
            .collect::<Vec<_>>()
    };
    if ids.len() != 1 {
        return Err(
            "Template must resolve to one project/ruleset template; use entity.templates".into(),
        );
    }
    Ok(ids[0])
}
fn entity_update(project: &mut Project, c: &EntityUpdate) -> Result<Json, String> {
    let i = region_index(project, &c.region)?;
    let id = parse_id(&c.id)?;
    let pos = c.position.map(point).transpose()?;
    if let Some(n) = &c.name {
        name(n)?;
    }
    if let Some(data) = &c.data {
        if data.len() > 65536 {
            return Err("Instance TOML exceeds 64 KiB".into());
        }
        data.parse::<toml::Table>().map_err(|e| e.to_string())?;
    }
    if let Some([x, z]) = c.orientation {
        if !x.is_finite() || !z.is_finite() || x * x + z * z < 0.00001 {
            return Err("Orientation must be a finite nonzero vector".into());
        }
    }
    match c.kind.as_str() {
        "character" => {
            let e = project.regions[i]
                .characters
                .get_mut(&id)
                .ok_or("Character instance not found")?;
            if let Some(p) = pos {
                e.position = p;
            }
            if let Some(n) = &c.name {
                e.name = n.clone();
            }
            if let Some(d) = &c.data {
                e.data = d.clone();
            }
            if let Some(o) = c.orientation {
                e.orientation = Vec2::from(o).normalized();
            }
        }
        "item" => {
            if c.orientation.is_some() {
                return Err(
                    "Item orientation is authored by its appearance; use instance TOML".into(),
                );
            }
            let e = project.regions[i]
                .items
                .get_mut(&id)
                .ok_or("Item instance not found")?;
            if let Some(p) = pos {
                e.position = p;
            }
            if let Some(n) = &c.name {
                e.name = n.clone();
            }
            if let Some(d) = &c.data {
                e.data = d.clone();
            }
        }
        _ => return Err("Entity kind must be character or item".into()),
    }
    let owner = shared::entity_graph::instance_key(project.regions[i].id, &c.kind, id);
    if let Some(data) = &c.data {
        let rules = project
            .rules_source()?
            .parse::<shared::entity_graph::RulesTable>()
            .map_err(|e| e.to_string())?;
        let doc = shared::entity_graph::import(data, &rules)?;
        project.node_graphs.insert(
            owner.clone(),
            serde_json::to_value(doc).map_err(|e| e.to_string())?,
        );
    }
    if let Some(active) = c.active {
        shared::entity_graph::set_owner_attribute(project, &owner, "active", json!(active))?;
    }
    Ok(json!({"id":id,"kind":c.kind}))
}
pub(super) fn validate_targets(project: &Project, i: usize, t: &MapTargets) -> Result<(), String> {
    if t.sectors.len() + t.geometry.len() + t.walls.len() + t.characters.len() + t.items.len()
        > 4096
    {
        return Err("Maximum 4096 map targets".into());
    }
    for ids in [&t.geometry, &t.walls, &t.characters, &t.items] {
        let mut seen = HashSet::new();
        for id in ids {
            if !seen.insert(parse_id(id)?) {
                return Err("Duplicate map target".into());
            }
        }
    }
    if t.sectors.iter().copied().collect::<HashSet<_>>().len() != t.sectors.len() {
        return Err("Duplicate sector target".into());
    }
    let r = &project.regions[i];
    for id in &t.sectors {
        if r.map.find_sector(*id).is_none() {
            return Err(format!("Sector {id} not found"));
        }
    }
    for id in &t.geometry {
        let id = parse_id(id)?;
        let o = r
            .map
            .geometry_objects
            .iter()
            .find(|o| o.id == id)
            .ok_or("Geometry object not found")?;
        if o.kind == GeometryObjectKind::Generated {
            return Err("Edit generated wall geometry through its wall assembly".into());
        }
    }
    for id in &t.walls {
        if r.map.wall_assembly(parse_id(id)?).is_none() {
            return Err("Wall assembly not found".into());
        }
    }
    for id in &t.characters {
        if !r.characters.contains_key(&parse_id(id)?) {
            return Err("Character instance not found".into());
        }
    }
    for id in &t.items {
        if !r.items.contains_key(&parse_id(id)?) {
            return Err("Item instance not found".into());
        }
    }
    Ok(())
}
fn transform_point(p: Vec3<f32>, c: &MapTransform, vector: bool) -> Vec3<f32> {
    let pivot = if vector {
        Vec3::zero()
    } else {
        Vec3::from(c.pivot)
    };
    let q = (p - pivot) * Vec3::from(c.scale);
    let (s, cos) = c.rotation_y.to_radians().sin_cos();
    Vec3::new(q.x * cos + q.z * s, q.y, -q.x * s + q.z * cos)
        + if vector {
            Vec3::zero()
        } else {
            pivot + Vec3::from(c.translation)
        }
}
fn transform_cell(cell: [i32; 2], c: &MapTransform) -> Result<[i32; 2], String> {
    if (c.scale[0] - 1.).abs() > 0.0001
        || (c.scale[2] - 1.).abs() > 0.0001
        || (c.rotation_y / 90. - (c.rotation_y / 90.).round()).abs() > 0.0001
        || c.translation[0].fract().abs() > 0.0001
        || c.translation[2].fract().abs() > 0.0001
    {
        return Err(
            "Painted cell overrides require unit scale, quarter turns and whole-cell translation"
                .into(),
        );
    }
    let p = transform_point(
        Vec3::new(cell[0] as f32 + 0.5, 0., cell[1] as f32 + 0.5),
        c,
        false,
    );
    point(p.into_array())?;
    if (p.x - 0.5 - (p.x - 0.5).round()).abs() > 0.001
        || (p.z - 0.5 - (p.z - 0.5).round()).abs() > 0.001
    {
        return Err(
            "Painted cell transform must preserve the cell grid; adjust the rotation pivot".into(),
        );
    }
    Ok([p.x.floor() as i32, p.z.floor() as i32])
}
fn remap_cell_overrides(properties: &mut ValueContainer, c: &MapTransform) -> Result<(), String> {
    for key in ["tiles", "blend_tiles"] {
        match properties.get_mut(key) {
            Some(Value::TileOverrides(tiles)) => {
                *tiles = std::mem::take(tiles)
                    .into_iter()
                    .map(|(cell, v)| {
                        let cell = transform_cell([cell.0, cell.1], c)?;
                        Ok(((cell[0], cell[1]), v))
                    })
                    .collect::<Result<_, String>>()?;
            }
            Some(Value::BlendOverrides(tiles)) => {
                *tiles = std::mem::take(tiles)
                    .into_iter()
                    .map(|(cell, v)| {
                        let cell = transform_cell([cell.0, cell.1], c)?;
                        Ok(((cell[0], cell[1]), v))
                    })
                    .collect::<Result<_, String>>()?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn transform_object(object: &mut GeometryObject, c: &MapTransform) {
    for col in 0..4 {
        let p = transform_point(
            Vec3::new(
                object.transform[col][0],
                object.transform[col][1],
                object.transform[col][2],
            ),
            c,
            col != 3,
        );
        for axis in 0..3 {
            object.transform[col][axis] = p[axis];
        }
    }
}
fn transform(project: &mut Project, c: &MapTransform, copy: bool) -> Result<Json, String> {
    let i = region_index(project, &c.region)?;
    validate_targets(project, i, &c.targets)?;
    point(c.translation)?;
    point(c.pivot)?;
    if !c.rotation_y.is_finite()
        || c.scale
            .iter()
            .any(|v| !v.is_finite() || !(0.001..=1024.).contains(v))
    {
        return Err(
            "Transform scale must be 0.001..1024; rotation is degrees around world Y".into(),
        );
    }
    if !c.targets.walls.is_empty() && ((c.scale[0] - c.scale[2]).abs() > 0.0001 || c.scale[1] != 1.)
    {
        return Err("Wall transforms require uniform horizontal scale and unchanged vertical scale; edit height through wall source settings".into());
    }
    let mut result = MapTargets::default();
    let mut binding_copies = vec![];
    let bindings = project
        .map_material_bindings
        .values()
        .filter(|b| b.map == project.regions[i].map.id)
        .map(|b| b.surface.clone())
        .collect::<Vec<_>>();
    // Capture topology first; allocate fresh vertex/line IDs to protect unselected neighbours.
    let sectors = c
        .targets
        .sectors
        .iter()
        .map(|id| {
            let s = project.regions[i].map.find_sector(*id).unwrap().clone();
            let lines = s
                .linedefs
                .iter()
                .map(|id| project.regions[i].map.find_linedef(*id).unwrap().clone())
                .collect::<Vec<_>>();
            (s, lines)
        })
        .collect::<Vec<_>>();
    for (old, lines) in sectors {
        let old_map = project.regions[i].map.clone();
        let old_surface = old_map.get_surface_for_sector_id(old.id).cloned();
        let map = &mut project.regions[i].map;
        if !copy {
            map.delete_elements(&[], &[], &[old.id]);
        }
        let new_id = if copy {
            map.find_free_sector_id()
                .ok_or("Sector ID space exhausted")?
        } else {
            old.id
        };
        let mut vm = std::collections::HashMap::new();
        let mut lm = vec![];
        for l in lines {
            for id in [l.start_vertex, l.end_vertex] {
                if !vm.contains_key(&id) {
                    let old = old_map.find_vertex(id).ok_or("Missing sector vertex")?;
                    let p = transform_point(Vec3::new(old.x, old.z, old.y), c, false);
                    point(p.into_array())?;
                    let n = map
                        .find_free_vertex_id()
                        .ok_or("Vertex ID space exhausted")?;
                    let mut v = old.clone();
                    v.id = n;
                    v.x = p.x;
                    v.y = p.z;
                    v.z = p.y;
                    map.vertices.push(v);
                    vm.insert(id, n);
                }
            }
            let mut next = l.clone();
            next.id = map
                .find_free_linedef_id()
                .ok_or("Line ID space exhausted")?;
            next.creator_id = if copy { Uuid::new_v4() } else { l.creator_id };
            next.start_vertex = vm[&l.start_vertex];
            next.end_vertex = vm[&l.end_vertex];
            next.sector_ids = vec![new_id];
            lm.push(next.id);
            map.linedefs.push(next);
        }
        let mut next = old.clone();
        next.id = new_id;
        next.linedefs = lm;
        if copy {
            next.creator_id = Uuid::new_v4();
            next.name = format!("{} Copy", next.name);
        }
        remap_cell_overrides(&mut next.properties, c)?;
        map.sectors.push(next);
        let mut surface = old_surface.unwrap_or_else(|| {
            let mut s = rusterix::Surface::new(new_id);
            s.extrusion.enabled = false;
            s
        });
        if copy {
            surface.id = Uuid::new_v4();
        }
        surface.sector_id = new_id;
        surface.calculate_geometry(map);
        map.surfaces.insert(surface.id, surface);
        result.sectors.push(new_id);
        if copy {
            for old_binding in &bindings {
                if let CompositionSurface::SectorTile { sector, cell } = old_binding {
                    if *sector == old.id {
                        binding_copies.push((
                            old_binding.clone(),
                            CompositionSurface::SectorTile {
                                sector: new_id,
                                cell: transform_cell(*cell, c)?,
                            },
                        ));
                    }
                }
            }
            binding_copies.push((
                CompositionSurface::RectTile { sector: old.id },
                CompositionSurface::RectTile { sector: new_id },
            ));
            binding_copies.push((
                CompositionSurface::Sector {
                    id: old.id,
                    ceiling: false,
                },
                CompositionSurface::Sector {
                    id: new_id,
                    ceiling: false,
                },
            ));
            binding_copies.push((
                CompositionSurface::Sector {
                    id: old.id,
                    ceiling: true,
                },
                CompositionSurface::Sector {
                    id: new_id,
                    ceiling: true,
                },
            ));
        }
    }
    for id in &c.targets.geometry {
        let id = parse_id(id)?;
        let map = &mut project.regions[i].map;
        let mut object = map
            .geometry_objects
            .iter()
            .find(|o| o.id == id)
            .unwrap()
            .clone();
        transform_object(&mut object, c);
        for v in &object.vertices {
            point(object.transform_point(*v).into_array())?;
        }
        if copy {
            object.id = Uuid::new_v4();
            object.name = format!("{} Copy", object.name);
            for (f, face) in object.faces.iter_mut().enumerate() {
                let old_effect = face.paint_surface_id.unwrap_or(face.id);
                face.id = Uuid::new_v4();
                face.paint_surface_id = None;
                if let Some(e) = map.face_emissions.get(&old_effect).cloned() {
                    map.face_emissions.insert(face.id, e);
                }
                if let Some(e) = map.face_particle_emissions.get(&old_effect).cloned() {
                    map.face_particle_emissions.insert(face.id, e);
                }
                for old_binding in &bindings {
                    if let CompositionSurface::GeometryTile {
                        object: old_object,
                        face: old_face,
                        cell,
                    } = old_binding
                    {
                        if *old_object == id && *old_face == f {
                            binding_copies.push((
                                old_binding.clone(),
                                CompositionSurface::GeometryTile {
                                    object: object.id,
                                    face: f,
                                    cell: *cell,
                                },
                            ));
                        }
                    }
                }
                binding_copies.push((
                    CompositionSurface::GeometryFace {
                        object: id,
                        face: f,
                    },
                    CompositionSurface::GeometryFace {
                        object: object.id,
                        face: f,
                    },
                ));
            }
            result.geometry.push(object.id.to_string());
            map.geometry_objects.push(object);
        } else {
            result.geometry.push(id.to_string());
            *map.geometry_objects
                .iter_mut()
                .find(|o| o.id == id)
                .unwrap() = object;
        }
    }
    for id in &c.targets.walls {
        let id = parse_id(id)?;
        let map = &mut project.regions[i].map;
        let mut wall = map.wall_assembly(id).unwrap().clone();
        for node in &mut wall.nodes {
            node.position = transform_point(node.position, c, false);
        }
        for surface in &mut wall.area_surfaces {
            for p in &mut surface.outline {
                *p = transform_point(*p, c, false);
            }
        }
        if c.scale[0] != 1. {
            wall.style.thickness *= c.scale[0];
            for s in &mut wall.spans {
                s.curve_offset *= c.scale[0];
                if let Some(style) = &mut s.style_override {
                    style.thickness *= c.scale[0];
                }
                for o in &mut s.openings {
                    o.center *= c.scale[0];
                    o.width *= c.scale[0];
                }
            }
        }
        wall_budget(&wall)?;
        if copy {
            wall.id = Uuid::new_v4();
            wall.name = format!("{} Copy", wall.name);
            let mut nodes = std::collections::HashMap::new();
            for n in &mut wall.nodes {
                let old = n.id;
                n.id = Uuid::new_v4();
                nodes.insert(old, n.id);
            }
            let mut spans = std::collections::HashMap::new();
            let mut openings = std::collections::HashMap::new();
            let mut areas = std::collections::HashMap::new();
            for span in &mut wall.spans {
                let old = span.id;
                span.id = Uuid::new_v4();
                spans.insert(old, span.id);
                span.start_node = nodes[&span.start_node];
                span.end_node = nodes[&span.end_node];
                for o in &mut span.openings {
                    let old = o.id;
                    o.id = Uuid::new_v4();
                    openings.insert(old, o.id);
                }
            }
            for area in &mut wall.area_surfaces {
                let old = area.id;
                area.id = Uuid::new_v4();
                areas.insert(old, area.id);
                for edge in &mut area.boundary {
                    edge.span_id = *spans
                        .get(&edge.span_id)
                        .ok_or("Area references a missing wall span")?;
                }
            }
            for binding in &bindings {
                let target = match binding {
                    CompositionSurface::WallArea { assembly, surface } if *assembly == id => {
                        Some(CompositionSurface::WallArea {
                            assembly: wall.id,
                            surface: *areas
                                .get(surface)
                                .ok_or("Area material binding is invalid")?,
                        })
                    }
                    CompositionSurface::WallSlot {
                        assembly,
                        span,
                        opening,
                        slot,
                    } if *assembly == id => Some(CompositionSurface::WallSlot {
                        assembly: wall.id,
                        span: span
                            .map(|s| {
                                spans
                                    .get(&s)
                                    .copied()
                                    .ok_or("Wall binding references a missing span")
                            })
                            .transpose()?,
                        opening: opening
                            .map(|o| {
                                openings
                                    .get(&o)
                                    .copied()
                                    .ok_or("Wall binding references a missing opening")
                            })
                            .transpose()?,
                        slot: *slot,
                    }),
                    _ => None,
                };
                if let Some(target) = target {
                    binding_copies.push((binding.clone(), target));
                }
            }
            result.walls.push(wall.id.to_string());
            map.wall_assemblies.push(wall);
        } else {
            result.walls.push(id.to_string());
            *map.wall_assembly_mut(id).unwrap() = wall;
        }
    }
    for id in &c.targets.characters {
        let id = parse_id(id)?;
        let mut e = project.regions[i].characters[&id].clone();
        e.position = transform_point(e.position, c, false);
        let o = transform_point(Vec3::new(e.orientation.x, 0., e.orientation.y), c, true);
        e.orientation = Vec2::new(o.x, o.z).normalized();
        if copy {
            let old = e.id;
            e.id = Uuid::new_v4();
            e.name = format!("{} Copy", e.name);
            copy_entity_graph(project, i, "character", old, e.id)?;
        }
        result.characters.push(e.id.to_string());
        project.regions[i].characters.insert(e.id, e);
    }
    for id in &c.targets.items {
        let id = parse_id(id)?;
        let mut e = project.regions[i].items[&id].clone();
        e.position = transform_point(e.position, c, false);
        if copy {
            let old = e.id;
            e.id = Uuid::new_v4();
            e.name = format!("{} Copy", e.name);
            copy_entity_graph(project, i, "item", old, e.id)?;
        }
        result.items.push(e.id.to_string());
        project.regions[i].items.insert(e.id, e);
    }
    let map_id = project.regions[i].map.id;
    if !copy && c.rotation_y != 0. {
        let ids = project
            .map_material_bindings
            .iter()
            .filter(|(_, b)| b.map == map_id)
            .filter(|(_, b)| match b.surface {
                CompositionSurface::RectTile { sector }
                | CompositionSurface::SectorTile { sector, .. } => {
                    c.targets.sectors.contains(&sector)
                }
                CompositionSurface::Sector { id, .. } => c.targets.sectors.contains(&id),
                CompositionSurface::GeometryFace { object, .. }
                | CompositionSurface::GeometryTile { object, .. } => {
                    c.targets.geometry.iter().any(|s| parse_id(s) == Ok(object))
                }
                CompositionSurface::WallArea { assembly, .. }
                | CompositionSurface::WallSlot { assembly, .. } => {
                    c.targets.walls.iter().any(|s| parse_id(s) == Ok(assembly))
                }
                _ => false,
            })
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in ids {
            if let Some(comp) = project.material_compositions.get_mut(&id) {
                for layer in &mut comp.layers {
                    layer.angle += c.rotation_y;
                }
            }
        }
    }
    if !copy {
        for binding in project
            .map_material_bindings
            .values_mut()
            .filter(|b| b.map == map_id)
        {
            if let CompositionSurface::SectorTile { sector, cell } = &mut binding.surface {
                if c.targets.sectors.contains(sector) {
                    *cell = transform_cell(*cell, c)?;
                }
            }
        }
    }
    for (old, new) in binding_copies {
        if let Some(source) = project
            .map_material_bindings
            .iter()
            .find(|(_, b)| b.map == map_id && b.surface == old)
            .map(|(id, _)| *id)
        {
            copy_binding(project, i, source, new, c.rotation_y)?;
        }
    }
    project.regions[i].map.update_surfaces();
    project.regions[i].map.rebuild_wall_geometry();
    Ok(json!({"targets":result}))
}
fn copy_entity_graph(
    project: &mut Project,
    i: usize,
    kind: &str,
    old: Uuid,
    new: Uuid,
) -> Result<(), String> {
    let old = shared::entity_graph::instance_key(project.regions[i].id, kind, old);
    let new = shared::entity_graph::instance_key(project.regions[i].id, kind, new);
    if let Some(graph) = project.node_graphs.get(&old).cloned() {
        project.node_graphs.insert(new, graph);
    }
    Ok(())
}
fn copy_binding(
    project: &mut Project,
    i: usize,
    old: Uuid,
    surface: CompositionSurface,
    rotation_y: f32,
) -> Result<(), String> {
    let id = Uuid::new_v4();
    let mut c = project
        .material_compositions
        .get(&old)
        .ok_or("Placed composition definition is missing")?
        .clone();
    for layer in &mut c.layers {
        layer.angle += rotation_y;
    }
    let mut tile = project
        .tiles
        .get(&old)
        .ok_or("Placed composition tile missing")?
        .clone();
    tile.id = id;
    project.tiles.insert(id, tile);
    project.material_compositions.insert(id, c);
    let binding = MapMaterialBinding {
        map: project.regions[i].map.id,
        surface,
    };
    if !crate::procedural_material::map_composition::set_source(
        &mut project.regions[i].map,
        &binding,
        id,
    ) {
        return Err("Could not bind copied composition".into());
    }
    project.map_material_bindings.insert(id, binding);
    Ok(())
}
fn delete(project: &mut Project, c: &MapDelete) -> Result<Json, String> {
    let i = region_index(project, &c.region)?;
    validate_targets(project, i, &c.targets)?;
    let map = &mut project.regions[i].map;
    map.delete_elements(&[], &[], &c.targets.sectors);
    let objects = c
        .targets
        .geometry
        .iter()
        .map(|id| parse_id(id))
        .collect::<Result<HashSet<_>, _>>()?;
    let paint = map
        .geometry_objects
        .iter()
        .filter(|o| objects.contains(&o.id))
        .flat_map(|o| o.faces.iter().map(|f| f.paint_surface_id.unwrap_or(f.id)))
        .collect::<Vec<_>>();
    map.geometry_objects.retain(|o| !objects.contains(&o.id));
    for id in paint {
        map.face_emissions.remove(&id);
        map.face_particle_emissions.remove(&id);
    }
    let walls = c
        .targets
        .walls
        .iter()
        .map(|id| parse_id(id))
        .collect::<Result<HashSet<_>, _>>()?;
    map.wall_assemblies.retain(|w| !walls.contains(&w.id));
    map.rebuild_wall_geometry();
    map.clear_selection();
    for id in &c.targets.characters {
        let id = parse_id(id)?;
        project.regions[i].characters.shift_remove(&id);
        project
            .node_graphs
            .shift_remove(&shared::entity_graph::instance_key(
                project.regions[i].id,
                "character",
                id,
            ));
    }
    for id in &c.targets.items {
        let id = parse_id(id)?;
        project.regions[i].items.shift_remove(&id);
        project
            .node_graphs
            .shift_remove(&shared::entity_graph::instance_key(
                project.regions[i].id,
                "item",
                id,
            ));
    }
    // Keep reusable recipe definitions. Detached surface ownership is removed.
    let map = &project.regions[i].map;
    let ids = project
        .map_material_bindings
        .iter()
        .filter(|(_, b)| b.map == map.id)
        .filter(|(_, b)| match b.surface {
            CompositionSurface::RectTile { sector }
            | CompositionSurface::SectorTile { sector, .. } => c.targets.sectors.contains(&sector),
            CompositionSurface::Sector { id, .. } => c.targets.sectors.contains(&id),
            CompositionSurface::GeometryFace { object, .. }
            | CompositionSurface::GeometryTile { object, .. } => objects.contains(&object),
            CompositionSurface::WallArea { assembly, .. }
            | CompositionSurface::WallSlot { assembly, .. } => walls.contains(&assembly),
            _ => false,
        })
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    for id in ids {
        project.map_material_bindings.shift_remove(&id);
    }
    Ok(json!({"deleted":c.targets}))
}
fn wall_update(project: &mut Project, c: &WallUpdate) -> Result<Json, String> {
    let i = region_index(project, &c.region)?;
    let id = parse_id(&c.id)?;
    if let Some(n) = &c.name {
        name(n)?;
    }
    if c.height.is_some_and(|h| !(0.1..=256.).contains(&h))
        || c.thickness.is_some_and(|h| !(0.02..=16.).contains(&h))
    {
        return Err("Invalid wall dimensions".into());
    }
    if c.nodes.len() + c.spans.len() > 256 {
        return Err("Maximum 256 wall-source edits".into());
    }
    let material = source(project, &c.material)?;
    let map = &mut project.regions[i].map;
    let wall = map.wall_assembly_mut(id).ok_or("Wall not found")?;
    if let Some(n) = &c.name {
        wall.name = n.clone();
    }
    if let Some(h) = c.height {
        wall.style.height = h;
    }
    if let Some(t) = c.thickness {
        wall.style.thickness = t;
    }
    if c.material.is_some() {
        wall.style.stone_source = material;
    }
    for node in &c.nodes {
        wall.set_node_position(parse_id(&node.id)?, point(node.position)?)?;
    }
    for update in &c.spans {
        let span = wall
            .span_mut(parse_id(&update.id)?)
            .ok_or("Wall span not found")?;
        if let Some(offset) = update.curve_offset {
            if !offset.is_finite() || offset.abs() > 4096. {
                return Err("Invalid wall curvature".into());
            }
            span.curve_offset = offset;
        }
        if let Some(segments) = update.curve_segments {
            if !(1..=128).contains(&segments) {
                return Err("Curve segments must be 1..128".into());
            }
            span.curve_segments = segments;
        }
    }
    for span in &wall.spans {
        let height = span.style_override.as_ref().unwrap_or(&wall.style).height;
        let length = wall.span_length(span.id).ok_or("Wall became degenerate")?;
        if span.openings.iter().any(|o| {
            o.bottom + o.height > height
                || o.center - o.width / 2. < 0.
                || o.center + o.width / 2. > length
        }) {
            return Err("Wall edit would clip an existing opening".into());
        }
    }
    wall_budget(wall)?;
    map.rebuild_wall_geometry();
    Ok(json!({"wall":id}))
}
fn sector_update(project: &mut Project, c: &SectorUpdate) -> Result<Json, String> {
    let i = region_index(project, &c.region)?;
    if let Some(n) = &c.name {
        name(n)?;
    }
    if c.height.is_some_and(|h| !(0.01..=256.).contains(&h))
        || c.floor_height
            .is_some_and(|h| !h.is_finite() || h.abs() > 1000000.)
    {
        return Err("Invalid sector height".into());
    }
    let map = &mut project.regions[i].map;
    let s = map.find_sector_mut(c.id).ok_or("Sector not found")?;
    if let Some(n) = &c.name {
        s.name = n.clone();
    }
    if let Some(h) = c.height {
        s.properties.set("wall_height", Value::Float(h));
    }
    if let Some(h) = c.floor_height {
        s.properties.set("floor_height", Value::Float(h));
    }
    if let Some(l) = c.layer {
        s.layer = Some(l);
    }
    let ids = s.linedefs.clone();
    if let Some(h) = c.floor_height {
        let vertices = ids
            .iter()
            .filter_map(|id| map.find_linedef(*id))
            .flat_map(|l| [l.start_vertex, l.end_vertex])
            .collect::<HashSet<_>>();
        if map
            .sectors
            .iter()
            .filter(|s| s.id != c.id)
            .flat_map(|s| &s.linedefs)
            .filter_map(|id| map.find_linedef(*id))
            .any(|l| vertices.contains(&l.start_vertex) || vertices.contains(&l.end_vertex))
        {
            return Err(
                "Move shared sector vertices with map_transform before changing floor height"
                    .into(),
            );
        }
        for id in vertices {
            map.find_vertex_mut(id).unwrap().z = h;
        }
    }
    if let Some(h) = c.height {
        for surface in map.surfaces.values_mut().filter(|s| s.sector_id == c.id) {
            surface.extrusion.enabled = true;
            surface.extrusion.depth = h;
        }
    }
    map.update_surfaces();
    Ok(json!({"sector":c.id}))
}

pub(super) fn erase_cells(project: &mut Project, c: &EraseCells) -> Result<Json, String> {
    let i = region_index(project, &c.region)?;
    if c.cells.is_empty() || c.cells.len() > 4096 {
        return Err("Erase requires 1..4096 cells".into());
    }
    let mut removed = HashSet::new();
    let mut masked = 0;
    for [x, y] in &c.cells {
        point([*x as f32, 0., *y as f32])?;
        let ids = Editor::scepter_find_cell_replacement_sectors(&project.regions[i].map, *x, *y);
        for id in ids {
            let map = &project.regions[i].map;
            let sector = map.find_sector(id).ok_or("Sector disappeared")?;
            if c.layer
                .is_some_and(|layer| sector.layer.unwrap_or(1) != layer)
            {
                continue;
            }
            let b = sector.bounding_box(map);
            if (b.min.x - *x as f32).abs() < 0.001
                && (b.min.y - *y as f32).abs() < 0.001
                && (b.max.x - *x as f32 - 1.).abs() < 0.001
                && (b.max.y - *y as f32 - 1.).abs() < 0.001
            {
                removed.insert(id);
            } else {
                crate::procedural_material::map_composition::set_tile_override(
                    &mut project.regions[i]
                        .map
                        .find_sector_mut(id)
                        .unwrap()
                        .properties,
                    [*x, *y],
                    PixelSource::Off,
                );
                masked += 1;
            }
        }
    }
    let deleted = if removed.is_empty() {
        json!({"sectors":[]})
    } else {
        delete(
            project,
            &MapDelete {
                region: c.region.clone(),
                targets: MapTargets {
                    sectors: removed.into_iter().collect(),
                    ..Default::default()
                },
            },
        )?
    };
    Ok(json!({"deleted":deleted,"masked_cells":masked}))
}

pub(super) fn apply(project: &mut Project, op: &EditOperation) -> Result<Json, String> {
    let (region, result) = match op {
        EditOperation::WallUpdate(c) => (c.region.clone(), wall_update(project, c)?),
        EditOperation::SectorUpdate(c) => (c.region.clone(), sector_update(project, c)?),
        EditOperation::GeometryCreate(c) => (c.region.clone(), geometry_create(project, c)?),
        EditOperation::WallCreate(c) => (c.region.clone(), wall_create(project, c)?),
        EditOperation::RoomCreate(c) => (c.region.clone(), room_create(project, c)?),
        EditOperation::OpeningPut(c) => (c.region.clone(), opening_put(project, c)?),
        EditOperation::OpeningDelete(c) => {
            let i = region_index(project, &c.region)?;
            let map = &mut project.regions[i].map;
            if !map
                .wall_assembly_mut(parse_id(&c.assembly)?)
                .ok_or("Wall not found")?
                .remove_opening(parse_id(&c.span)?, parse_id(&c.id)?)
            {
                return Err("Opening not found".into());
            }
            map.rebuild_wall_geometry();
            (c.region.clone(), json!({"opening":c.id,"deleted":true}))
        }
        EditOperation::EntityPlace(c) => (c.region.clone(), entity_place(project, c)?),
        EditOperation::EntityUpdate(c) => (c.region.clone(), entity_update(project, c)?),
        EditOperation::MapTransform(c) => (c.region.clone(), transform(project, c, false)?),
        EditOperation::MapCopy(c) => (c.region.clone(), transform(project, c, true)?),
        EditOperation::MapDelete(c) => (c.region.clone(), delete(project, c)?),
        EditOperation::EraseCells(c) => (c.region.clone(), erase_cells(project, c)?),
        EditOperation::SectorCreate(c) => {
            name(&c.name)?;
            let i = region_index(project, &c.region)?;
            let floor = source(project, &c.floor)?;
            let ceiling = source(project, &c.ceiling)?;
            if c.height
                .is_some_and(|h| !h.is_finite() || h <= 0. || h > 256.)
            {
                return Err("Sector height must be 0..256".into());
            }
            let map = &mut project.regions[i].map;
            let id = polygon(map, &c.polygon, &c.name, c.layer)?;
            let s = map.find_sector_mut(id).unwrap();
            if let Some(f) = floor {
                s.properties.set("source", Value::Source(f));
            }
            if let Some(f) = ceiling {
                s.properties.set("ceiling_source", Value::Source(f));
            }
            if let Some(height) = c.height {
                s.properties.set("wall_height", Value::Float(height));
                for surface in map.surfaces.values_mut().filter(|s| s.sector_id == id) {
                    surface.extrusion.enabled = true;
                    surface.extrusion.depth = height;
                }
            }
            map.update_surfaces();
            (c.region.clone(), json!({"sector":id}))
        }
        _ => return Err("Not a map operation".into()),
    };
    let i = region_index(project, &region)?;
    project.regions[i].map.changed = project.regions[i].map.changed.wrapping_add(1);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::super::scepter_authoring::stage;
    use super::*;
    pub(super) fn fixture() -> Project {
        let mut p = Project::default();
        p.regions[0].map.geometry_objects.clear();
        p.regions[0].map.sectors.clear();
        p.regions[0].map.vertices.clear();
        p.regions[0].map.linedefs.clear();
        let mut tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(4, 4)]);
        tile.alias = "Test terrain".into();
        p.tiles.insert(tile.id, tile);
        for template in shared::rulesets::ruleset_item_templates_from_source(
            shared::rulesets::latest_official_ruleset(),
        )
        .unwrap()
        {
            let item = shared::item::Item {
                name: template.name,
                data: template.data,
                source: template.source,
                ..Default::default()
            };
            p.items.insert(item.id, item);
        }
        let c = shared::character::Character {
            name: "Guard".into(),
            data: "[attributes]\nrace = 'Human'\nstart_equipped_items = ['Linen Shirt']\n".into(),
            ..Default::default()
        };
        p.characters.insert(c.id, c);
        p
    }
    pub(super) fn room(p: &Project, id: Uuid) -> RoomCreate {
        RoomCreate {
            region: RegionRef::id(p.regions[0].id.to_string()),
            id: id.to_string(),
            name: "Shared workshop".into(),
            min: [0., 0., 0.],
            max: [6., 3., 5.],
            thickness: 0.25,
            floor: None,
            wall: None,
            ceiling: None,
            doors: vec![RoomDoor {
                wall: 0,
                center: 3.,
                width: 1.2,
                height: 2.2,
                bottom: 0.,
            }],
        }
    }
    fn run(p: &Project, operations: Vec<EditOperation>) -> Result<(Project, Vec<Json>), String> {
        stage(
            p,
            &Transaction {
                operations,
                dry_run: false,
            },
        )
    }
    #[test]
    fn scepter_room_instances_and_states_are_one_staged_edit() {
        let p = fixture();
        let original = serde_json::to_string(&p).unwrap();
        let r = RegionRef::id(p.regions[0].id.to_string());
        let wall = Uuid::new_v4();
        let npc = Uuid::new_v4();
        let torch = Uuid::new_v4();
        let (next, results) = run(
            &p,
            vec![
                EditOperation::RoomCreate(room(&p, wall)),
                EditOperation::EntityPlace(EntityPlace {
                    region: r.clone(),
                    id: npc.to_string(),
                    kind: "character".into(),
                    template: "Guard".into(),
                    position: [2., 0., 2.],
                    name: Some("Workshop guard".into()),
                    data: None,
                    active: None,
                }),
                EditOperation::EntityPlace(EntityPlace {
                    region: r.clone(),
                    id: torch.to_string(),
                    kind: "item".into(),
                    template: "Torch".into(),
                    position: [1., 0., 1.],
                    name: None,
                    data: None,
                    active: Some(false),
                }),
            ],
        )
        .unwrap();
        assert_eq!(serde_json::to_string(&p).unwrap(), original);
        assert_eq!(results.len(), 3);
        let map = &next.regions[0].map;
        assert_eq!(map.wall_assemblies[0].spans.len(), 4);
        assert_eq!(map.wall_assemblies[0].spans[0].openings.len(), 1);
        assert!(
            map.geometry_objects
                .iter()
                .any(|o| o.name == "Shared workshop Floor")
        );
        assert_eq!(
            next.regions[0].characters[&npc].character_id,
            *p.characters.keys().next().unwrap()
        );
        assert!(
            next.regions[0].items[&torch]
                .data
                .contains("active = false")
        );
        assert_eq!(map.entities.len(), 1);
        assert_eq!(map.items.len(), 1);
        let owner = shared::entity_graph::instance_key(next.regions[0].id, "item", torch);
        assert!(next.node_graphs.contains_key(&owner));
        let (updated, _) = run(
            &next,
            vec![EditOperation::EntityUpdate(EntityUpdate {
                region: r,
                id: torch.to_string(),
                kind: "item".into(),
                position: Some([3., 0., 2.]),
                orientation: None,
                name: Some("Entry light".into()),
                data: Some("[attributes]\nactive = true\n".into()),
                active: Some(false),
            })],
        )
        .unwrap();
        assert!(
            updated.regions[0].items[&torch]
                .data
                .contains("active = false")
        );
        assert_eq!(
            updated.regions[0].items[&torch].position,
            Vec3::new(3., 0., 2.)
        );
    }
    #[test]
    fn scepter_failed_placement_rolls_back_the_entire_room() {
        let p = fixture();
        let before = serde_json::to_string(&p).unwrap();
        let r = RegionRef::id(p.regions[0].id.to_string());
        assert!(
            run(
                &p,
                vec![
                    EditOperation::RoomCreate(room(&p, Uuid::new_v4())),
                    EditOperation::EntityPlace(EntityPlace {
                        region: r,
                        id: Uuid::new_v4().to_string(),
                        kind: "item".into(),
                        template: "Missing template".into(),
                        position: [0.; 3],
                        name: None,
                        data: None,
                        active: None
                    })
                ]
            )
            .is_err()
        );
        assert_eq!(serde_json::to_string(&p).unwrap(), before);
    }
    #[test]
    fn scepter_copy_transform_delete_preserves_wall_graph_and_entity_setup() {
        let p = fixture();
        let r = RegionRef::id(p.regions[0].id.to_string());
        let wall = Uuid::new_v4();
        let item = Uuid::new_v4();
        let (p, _) = run(
            &p,
            vec![
                EditOperation::RoomCreate(room(&p, wall)),
                EditOperation::EntityPlace(EntityPlace {
                    region: r.clone(),
                    id: item.to_string(),
                    kind: "item".into(),
                    template: "Torch".into(),
                    position: [1., 0., 1.],
                    name: None,
                    data: None,
                    active: Some(false),
                }),
            ],
        )
        .unwrap();
        let old = p.regions[0].map.wall_assembly(wall).unwrap();
        let old_span = old.spans[0].id;
        let old_open = old.spans[0].openings[0].id;
        let targets = MapTargets {
            walls: vec![wall.to_string()],
            items: vec![item.to_string()],
            ..Default::default()
        };
        let (p, results) = run(
            &p,
            vec![EditOperation::MapCopy(MapTransform {
                region: r.clone(),
                targets,
                translation: [10., 0., 0.],
                pivot: [0.; 3],
                rotation_y: 90.,
                scale: [1.; 3],
            })],
        )
        .unwrap();
        let new_wall = parse_id(results[0]["targets"]["walls"][0].as_str().unwrap()).unwrap();
        let w = p.regions[0].map.wall_assembly(new_wall).unwrap();
        assert_ne!(w.spans[0].id, old_span);
        assert_ne!(w.spans[0].openings[0].id, old_open);
        assert_eq!(w.spans[0].openings.len(), 1);
        let copy_id = parse_id(results[0]["targets"]["items"][0].as_str().unwrap()).unwrap();
        assert_ne!(copy_id, item);
        assert!(p.regions[0].items[&copy_id].data.contains("active = false"));
        let (p, _) = run(
            &p,
            vec![EditOperation::MapDelete(MapDelete {
                region: r,
                targets: MapTargets {
                    walls: vec![new_wall.to_string()],
                    items: vec![copy_id.to_string()],
                    ..Default::default()
                },
            })],
        )
        .unwrap();
        assert!(p.regions[0].map.wall_assembly(wall).is_some());
        assert!(p.regions[0].map.wall_assembly(new_wall).is_none());
        assert!(p.regions[0].items.contains_key(&item));
        assert!(!p.regions[0].items.contains_key(&copy_id));
    }
    #[test]
    fn scepter_sector_move_does_not_drag_neighbours_and_partial_erase_keeps_floor() {
        let p = fixture();
        let r = RegionRef::id(p.regions[0].id.to_string());
        let (p, result) = run(
            &p,
            vec![EditOperation::SectorCreate(SectorCreate {
                region: r.clone(),
                name: "Floor".into(),
                polygon: vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
                floor: Some(TileSelector::alias("Test terrain")),
                ceiling: None,
                height: None,
                layer: Some(1),
            })],
        )
        .unwrap();
        let sector = result[0]["sector"].as_u64().unwrap() as u32;
        let (p, _) = run(
            &p,
            vec![EditOperation::EraseCells(EraseCells {
                region: r.clone(),
                cells: vec![[1, 1]],
                layer: Some(1),
            })],
        )
        .unwrap();
        assert!(p.regions[0].map.find_sector(sector).is_some());
        assert!(matches!(
            p.regions[0]
                .map
                .find_sector(sector)
                .unwrap()
                .properties
                .get("tiles"),
            Some(Value::TileOverrides(_))
        ));
        let (next, results) = run(
            &p,
            vec![EditOperation::MapCopy(MapTransform {
                region: r,
                targets: MapTargets {
                    sectors: vec![sector],
                    ..Default::default()
                },
                translation: [8., 0., 0.],
                pivot: [0.; 3],
                rotation_y: 0.,
                scale: [1.; 3],
            })],
        )
        .unwrap();
        let copied = results[0]["targets"]["sectors"][0].as_u64().unwrap() as u32;
        assert_ne!(copied, sector);
        assert!(
            matches!(next.regions[0].map.find_sector(copied).unwrap().properties.get("tiles"),Some(Value::TileOverrides(tiles)) if tiles.contains_key(&(9,1))&&!tiles.contains_key(&(1,1)))
        );
        assert_eq!(
            next.regions[0]
                .map
                .find_sector(sector)
                .unwrap()
                .bounding_box(&next.regions[0].map)
                .min
                .x,
            0.
        );
        assert_eq!(
            next.regions[0]
                .map
                .find_sector(copied)
                .unwrap()
                .bounding_box(&next.regions[0].map)
                .min
                .x,
            8.
        );
    }
    #[test]
    fn scepter_opening_and_wall_edits_validate_before_publication() {
        let p = fixture();
        let r = RegionRef::id(p.regions[0].id.to_string());
        let id = Uuid::new_v4();
        let (p, results) = run(&p, vec![EditOperation::RoomCreate(room(&p, id))]).unwrap();
        let span = results[0]["spans"][0].as_str().unwrap().to_string();
        let original = serde_json::to_string(&p).unwrap();
        assert!(
            run(
                &p,
                vec![EditOperation::OpeningPut(OpeningPut {
                    region: r.clone(),
                    assembly: id.to_string(),
                    span,
                    id: Uuid::new_v4().to_string(),
                    center: 0.,
                    bottom: 0.,
                    width: 2.,
                    height: 2.,
                    arched: true
                })]
            )
            .is_err()
        );
        assert!(
            run(
                &p,
                vec![EditOperation::WallUpdate(WallUpdate {
                    region: r,
                    id: id.to_string(),
                    name: None,
                    height: Some(1.),
                    thickness: None,
                    material: None,
                    nodes: vec![],
                    spans: vec![]
                })]
            )
            .is_err()
        );
        assert_eq!(serde_json::to_string(&p).unwrap(), original);
    }
}

#[cfg(test)]
mod acceptance {
    use super::*;
    use crate::scepter::jobs::Job;
    use std::sync::Arc;
    fn invoke(
        editor: &mut Editor,
        command: ScepterCommand,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Json {
        let job = Arc::new(Job::new());
        job.begin();
        let request = ScepterRequest {
            request_id: Some(Uuid::new_v4().to_string()),
            session: Some(editor.scepter_session()),
            revision: Some(editor.scepter_revision()),
            command,
        };
        editor.scepter_execute(request, job.clone(), ui, ctx);
        job.response(std::time::Duration::ZERO)
    }
    fn snapshot(project: &Project) -> u64 {
        let mut copy = project.clone();
        for r in &mut copy.regions {
            r.map.entities.clear();
            r.map.items.clear();
            r.map.clear_selection();
        }
        let mut h = DefaultHasher::new();
        serde_json::to_vec(&copy).unwrap().hash(&mut h);
        h.finish()
    }
    #[test]
    #[ignore = "requires native GPU and local project fixtures"]
    fn scepter_world_workflow_in_hideout_and_3d_starter() {
        let output = std::path::Path::new("/tmp/eldiron-scepter-world");
        std::fs::create_dir_all(output).unwrap();
        for fixture in ["Hideout2D-MZ", "3DStarter"] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../test_projects")
                .join(format!("{fixture}.eldiron"));
            let mut project =
                shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
            project.migrate_default_ruleset();
            project.sync_ruleset_items().unwrap();
            crate::procedural_material::starters::install(&mut project).unwrap();
            shared::rusterix_utils::insert_content_into_maps(&mut project);
            let preset = |label: &str| {
                project
                    .tiles
                    .values()
                    .find(|t| t.alias == label && t.id.as_u128() >> 64 == 0x454c444d41544552)
                    .unwrap()
                    .id
            };
            let grass = preset("Grass");
            let road = preset("Street");
            let walls = preset("Walls");
            let bounds = project.regions[0].map.bbox();
            let x = bounds.max.x.ceil() + 4.;
            let z = bounds.min.y.floor();
            let reference = RegionRef::id(project.regions[0].id.to_string());
            let character = project.characters.values().next().unwrap().id.to_string();
            let torch = project
                .items
                .values()
                .find(|i| i.name.eq_ignore_ascii_case("Torch"))
                .unwrap()
                .id
                .to_string();
            let wall = Uuid::new_v4();
            let npc = Uuid::new_v4();
            let light = Uuid::new_v4();
            let mix = Uuid::new_v4();
            let mut layer = shared::material_composition::CompositionLayer::split(road);
            layer.shape = shared::material_composition::CompositionShape::Road;
            layer.angle = 90.;
            layer.position = 0.8;
            layer.feather = 0.06;
            layer.wobble = 0.02;
            let selector = |id: Uuid| TileSelector {
                id: Some(id.to_string()),
                ..Default::default()
            };
            let operations = vec![
                EditOperation::CompositionPut {
                    id: mix.to_string(),
                    composition: json!(shared::material_composition::MaterialComposition {
                        base: grass,
                        layers: vec![layer],
                        tiling: Default::default()
                    }),
                },
                EditOperation::PaintRect(RegionPaintRect {
                    region: reference.clone(),
                    tile: selector(grass),
                    rect: [x as i32 - 1, z as i32 - 5, 8, 11],
                    layer: Some("1".into()),
                    select: None,
                    replace_existing: Some(true),
                }),
                EditOperation::GeometryCreate(GeometryCreate {
                    region: reference.clone(),
                    id: Uuid::new_v4().to_string(),
                    name: "Workshop lawn".into(),
                    shape: "floor".into(),
                    min: [x - 1., -0.1, z - 5.],
                    max: [x + 7., -0.05, z + 6.],
                    material: Some(selector(grass)),
                    segments: None,
                }),
                EditOperation::RoomCreate(RoomCreate {
                    region: reference.clone(),
                    id: wall.to_string(),
                    name: "Scepter workshop".into(),
                    min: [x, 0., z],
                    max: [x + 6., 3., z + 5.],
                    thickness: 0.25,
                    floor: Some(selector(road)),
                    wall: Some(selector(walls)),
                    ceiling: None,
                    doors: vec![RoomDoor {
                        wall: 0,
                        center: 3.,
                        width: 1.2,
                        height: 2.2,
                        bottom: 0.,
                    }],
                }),
                EditOperation::PaintRect(RegionPaintRect {
                    region: reference.clone(),
                    tile: selector(mix),
                    rect: [x as i32 + 3, z as i32 - 4, 1, 4],
                    layer: Some("1".into()),
                    select: None,
                    replace_existing: Some(true),
                }),
                EditOperation::GeometryCreate(GeometryCreate {
                    region: reference.clone(),
                    id: Uuid::new_v4().to_string(),
                    name: "Workshop approach".into(),
                    shape: "floor".into(),
                    min: [x + 3., -0.04, z - 4.],
                    max: [x + 4., -0.02, z],
                    material: Some(selector(mix)),
                    segments: None,
                }),
                EditOperation::EntityPlace(EntityPlace {
                    region: reference.clone(),
                    id: npc.to_string(),
                    kind: "character".into(),
                    template: character,
                    position: [x + 3., 0., z + 2.],
                    name: Some("Workshop guard".into()),
                    data: Some("[attributes]\nplayer = false\n".into()),
                    active: None,
                }),
                EditOperation::EntityPlace(EntityPlace {
                    region: reference.clone(),
                    id: light.to_string(),
                    kind: "item".into(),
                    template: torch,
                    position: [x + 2., 0., z - 0.8],
                    name: Some("Entry torch".into()),
                    data: None,
                    active: Some(true),
                }),
            ];
            let mut editor = Editor::new();
            editor.project = project;
            editor.server_ctx.curr_region = editor.project.regions[0].id;
            editor.server_ctx.pc = ProjectContext::Region(editor.server_ctx.curr_region);
            *UNDOMANAGER.write().unwrap() = UndoManager::default();
            let mut ui = TheUI::new();
            let mut ctx = TheContext::new(1024, 768, 1.);
            let original = snapshot(&editor.project);
            let result = invoke(
                &mut editor,
                ScepterCommand::Transaction(Transaction {
                    operations,
                    dry_run: false,
                }),
                &mut ui,
                &mut ctx,
            );
            assert_eq!(result["ok"], true, "{result}");
            assert_eq!(result["undo_steps"], 1);
            assert!(editor.project.regions[0].characters.contains_key(&npc));
            assert!(editor.project.regions[0].items.contains_key(&light));
            assert!(
                editor.project.regions[0]
                    .map
                    .wall_assembly(wall)
                    .unwrap()
                    .spans[0]
                    .openings
                    .len()
                    == 1
            );
            let saved = shared::project_io::encode_project(&editor.project).unwrap();
            let reopened = shared::project_io::decode_project(&saved).unwrap();
            assert!(reopened.regions[0].characters.contains_key(&npc));
            assert!(reopened.material_compositions.contains_key(&mix));
            std::fs::write(output.join(format!("{fixture}-workshop.eldiron")), saved).unwrap();
            for view in ["2d", "iso"] {
                let request:RegionRenderPreview=serde_json::from_value(json!({"region":{"id":editor.project.regions[0].id},"view":view,"size":[768,640],"bounds":[x as i32-1,z as i32-5,8,11],"camera":{"center":[x+3.,0.,z+1.],"scale":7.,"elevation":55.}})).unwrap();
                let image =
                    super::super::scepter_preview::render(editor.project.clone(), 0, &request)
                        .unwrap();
                assert_eq!(image["ready"], true);
                std::fs::write(
                    output.join(format!("{fixture}-{view}.base64")),
                    image["png_base64"].as_str().unwrap(),
                )
                .unwrap();
            }
            let region_id = editor.project.regions[0].id;
            let room_result = invoke(
                &mut editor,
                ScepterCommand::EditorSelect(
                    serde_json::from_value(
                        json!({"region":{"id":region_id},"targets":{"walls":[wall]}}),
                    )
                    .unwrap(),
                ),
                &mut ui,
                &mut ctx,
            );
            assert_eq!(room_result["ok"], true, "{room_result}");
            // View changes do not add undo steps.
            let undo = invoke(&mut editor, ScepterCommand::ProjectUndo, &mut ui, &mut ctx);
            assert_eq!(undo["ok"], true);
            assert!(!editor.project.regions[0].characters.contains_key(&npc));
            assert!(!UNDOMANAGER.read().unwrap().has_undo());
            // Selection is view state and can differ after explicit selection; clear it before comparing authoring state.
            for r in &mut editor.project.regions {
                r.map.clear_selection();
            }
            let restored = snapshot(&editor.project);
            assert_eq!(
                restored, original,
                "Undo must restore the complete authored project"
            );
            let redo = invoke(&mut editor, ScepterCommand::ProjectRedo, &mut ui, &mut ctx);
            assert_eq!(redo["ok"], true);
            assert!(editor.project.regions[0].characters.contains_key(&npc));
            println!(
                "Scepter {fixture}: room, road, doorway, ruleset NPC/torch, save/reopen, 2D/3D previews, one-step undo/redo passed"
            );
        }
    }
}
