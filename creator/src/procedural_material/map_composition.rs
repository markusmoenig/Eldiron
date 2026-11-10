//! Map-surface ownership and authoring entry point for material compositions.
use super::*;
use rusterix::{PixelSource, ValueContainer};
use shared::material_composition::{
    CompositionLayer, CompositionSurface, MapMaterialBinding, MaterialComposition,
};

pub(crate) const OPEN: &str = "Compose Map Materials";

fn rect_target_exists(map: &Map, target: &CompositionSurface) -> bool {
    match target {
        CompositionSurface::RectTile { sector } | CompositionSurface::SectorTile { sector, .. } => {
            map.find_sector(*sector).is_some()
        }
        CompositionSurface::GeometryTile { object, face, .. } => map
            .geometry_objects
            .iter()
            .find(|o| o.id == *object)
            .and_then(|o| o.faces.get(*face))
            .is_some(),
        CompositionSurface::TerrainTile { .. } => {
            map.properties.get_bool_default("terrain_enabled", false)
        }
        _ => false,
    }
}
fn is_rect_target(target: &CompositionSurface) -> bool {
    matches!(
        target,
        CompositionSurface::RectTile { .. }
            | CompositionSurface::SectorTile { .. }
            | CompositionSurface::GeometryTile { .. }
            | CompositionSurface::TerrainTile { .. }
    )
}
/// Rect's eyedropper can paint a composition elsewhere. Detach even the original
/// owner's tile before editing when its cache is referenced by another placement.
fn rect_reference_count(map: &Map, id: Uuid) -> usize {
    fn properties(properties: &ValueContainer, id: Uuid) -> usize {
        properties
            .values()
            .map(|value| match value {
                Value::Source(PixelSource::TileId(tile)) => usize::from(*tile == id),
                Value::TileOverrides(tiles) => tiles
                    .values()
                    .filter(|source| matches!(source,PixelSource::TileId(tile) if *tile==id))
                    .count(),
                Value::BlendOverrides(tiles) => tiles
                    .values()
                    .filter(|(_, source)| matches!(source,PixelSource::TileId(tile) if *tile==id))
                    .count(),
                _ => 0,
            })
            .sum()
    }
    properties(&map.properties, id)
        + map
            .sectors
            .iter()
            .map(|sector| properties(&sector.properties, id))
            .sum::<usize>()
        + map
            .geometry_objects
            .iter()
            .flat_map(|object| &object.faces)
            .map(|face| {
                usize::from(matches!(face.tile,Some(PixelSource::TileId(tile)) if tile==id))
                    + face
                        .tiles
                        .values()
                        .filter(|source| matches!(source,PixelSource::TileId(tile) if *tile==id))
                        .count()
            })
            .sum::<usize>()
        + map
            .profiles
            .values()
            .map(|profile| rect_reference_count(profile, id))
            .sum::<usize>()
}
fn tile_override(properties: &ValueContainer, cell: [i32; 2]) -> Option<PixelSource> {
    let key = (cell[0], cell[1]);
    if let Some(Value::TileOverrides(tiles)) = properties.get("tiles")
        && let Some(source) = tiles.get(&key)
    {
        return Some(source.clone());
    }
    if let Some(Value::BlendOverrides(tiles)) = properties.get("blend_tiles")
        && let Some((_, source)) = tiles.get(&key)
    {
        return Some(source.clone());
    }
    None
}
pub(crate) fn set_tile_override(
    properties: &mut ValueContainer,
    cell: [i32; 2],
    source: PixelSource,
) {
    let key = (cell[0], cell[1]);
    if let Some(Value::TileOverrides(tiles)) = properties.get_mut("tiles")
        && tiles.contains_key(&key)
    {
        tiles.insert(key, source);
        return;
    }
    if let Some(Value::BlendOverrides(tiles)) = properties.get_mut("blend_tiles")
        && let Some((_, old)) = tiles.get_mut(&key)
    {
        *old = source;
        return;
    }
    if let Some(Value::TileOverrides(tiles)) = properties.get_mut("tiles") {
        tiles.insert(key, source);
    } else {
        properties.set(
            "tiles",
            Value::TileOverrides([(key, source)].into_iter().collect()),
        );
    }
}

/// Resolve exactly one editable surface and the active material slot. Faces have
/// precedence over object/sector selection, as in the existing Apply Tile path.
pub(crate) fn selection(map: &Map, server: &ServerContext) -> Option<MapMaterialBinding> {
    if server.pc.is_region()
        && server.curr_map_tool_type == MapToolType::Rect
        && server.rect_compose_mode
    {
        return server
            .rect_composition_target
            .as_ref()
            .filter(|binding| binding.map == map.id && rect_target_exists(map, &binding.surface))
            .cloned();
    }
    let slot = server.selected_hud_icon_index;
    let surface = if !map.selected_geometry_faces.is_empty() {
        if map.selected_geometry_faces.len() != 1 {
            return None;
        }
        let (object, face) = map.selected_geometry_faces[0];
        map.geometry_objects
            .iter()
            .find(|o| o.id == object)?
            .faces
            .get(face)?;
        CompositionSurface::GeometryFace { object, face }
    } else if !map.selected_geometry_objects.is_empty() {
        // A whole object can have many distinct materials. Select a face first.
        return None;
    } else if server.curr_map_tool_type == MapToolType::Linedef {
        if map.selected_linedefs.len() != 1 || !(0..=3).contains(&slot) {
            return None;
        }
        CompositionSurface::Linedef {
            id: map.selected_linedefs[0],
            row: slot as u8 + 1,
        }
    } else {
        if map.selected_sectors.len() != 1 || !(0..=1).contains(&slot) {
            return None;
        }
        CompositionSurface::Sector {
            id: map.selected_sectors[0],
            ceiling: slot == 1,
        }
    };
    match surface {
        CompositionSurface::Sector { id, .. } => {
            map.find_sector(id)?;
        }
        CompositionSurface::Linedef { id, .. } => {
            map.find_linedef(id)?;
        }
        _ => {}
    }
    Some(MapMaterialBinding {
        map: map.id,
        surface,
    })
}

pub(crate) fn source(map: &Map, binding: &MapMaterialBinding) -> Option<PixelSource> {
    if binding.map != map.id {
        return None;
    }
    match binding.surface {
        CompositionSurface::RectTile { sector } => map
            .find_sector(sector)?
            .properties
            .get_default_source()
            .cloned(),
        CompositionSurface::SectorTile { sector, cell } => {
            let properties = &map.find_sector(sector)?.properties;
            tile_override(properties, cell).or_else(|| properties.get_default_source().cloned())
        }
        CompositionSurface::TerrainTile { cell } => tile_override(&map.properties, cell)
            .or_else(|| map.properties.get_source("default_terrain_tile").cloned()),
        CompositionSurface::GeometryTile { object, face, cell } => {
            let face = map
                .geometry_objects
                .iter()
                .find(|o| o.id == object)?
                .faces
                .get(face)?;
            face.tiles
                .get(&(cell[0], cell[1]))
                .cloned()
                .or_else(|| face.tile.clone())
        }
        CompositionSurface::Sector { id, ceiling } => map
            .find_sector(id)?
            .properties
            .get_source(if ceiling { "ceiling_source" } else { "source" })
            .cloned(),
        CompositionSurface::Linedef { id, row } => map
            .find_linedef(id)?
            .properties
            .get_source(&format!("row{row}_source"))
            .cloned(),
        CompositionSurface::GeometryFace { object, face } => {
            let face = map
                .geometry_objects
                .iter()
                .find(|o| o.id == object)?
                .faces
                .get(face)?;
            face.tile
                .clone()
                .or_else(|| face.tiles.values().next().cloned())
        }
    }
}
pub(crate) fn set_source(map: &mut Map, binding: &MapMaterialBinding, id: Uuid) -> bool {
    if map.id != binding.map {
        return false;
    }
    let source = PixelSource::TileId(id);
    match binding.surface {
        CompositionSurface::RectTile { sector } => {
            let Some(sector) = map.find_sector_mut(sector) else {
                return false;
            };
            sector.properties.set("source", Value::Source(source));
        }
        CompositionSurface::SectorTile { sector, cell } => {
            let Some(sector) = map.find_sector_mut(sector) else {
                return false;
            };
            set_tile_override(&mut sector.properties, cell, source);
        }
        CompositionSurface::TerrainTile { cell } => {
            if !map.properties.get_bool_default("terrain_enabled", false) {
                return false;
            }
            set_tile_override(&mut map.properties, cell, source);
        }
        CompositionSurface::GeometryTile { object, face, cell } => {
            let Some(face) = map
                .geometry_objects
                .iter_mut()
                .find(|o| o.id == object)
                .and_then(|o| o.faces.get_mut(face))
            else {
                return false;
            };
            face.tiles.insert((cell[0], cell[1]), source);
        }
        CompositionSurface::Sector { id, ceiling } => {
            let Some(sector) = map.find_sector_mut(id) else {
                return false;
            };
            sector.properties.set(
                if ceiling { "ceiling_source" } else { "source" },
                Value::Source(source),
            );
        }
        CompositionSurface::Linedef { id, row } => {
            let Some(line) = map.find_linedef_mut(id) else {
                return false;
            };
            line.properties
                .set(&format!("row{row}_source"), Value::Source(source));
        }
        CompositionSurface::GeometryFace { object, face } => {
            let Some(object) = map.geometry_objects.iter_mut().find(|o| o.id == object) else {
                return false;
            };
            let Some(face) = object.faces.get_mut(face) else {
                return false;
            };
            face.tile = Some(source);
            face.tiles.clear();
        }
    }
    map.update_surfaces();
    true
}
fn tile_id(source: &PixelSource) -> Option<Uuid> {
    match source {
        PixelSource::TileId(id) => Some(*id),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct CreatedMapComposition {
    pub tile: rusterix::Tile,
    pub composition: MaterialComposition,
    pub bundle: String,
    pub binding: MapMaterialBinding,
}
impl CreatedMapComposition {
    pub(crate) fn install(&self, project: &mut Project) {
        let id = self.tile.id;
        project.tiles.insert(id, self.tile.clone());
        project
            .material_compositions
            .insert(id, self.composition.clone());
        project.procedural_materials.insert(id, self.bundle.clone());
        project
            .map_material_bindings
            .insert(id, self.binding.clone());
        project.procedural_material_cache_keys.shift_remove(&id);
    }
}

impl ProceduralMaterialEditor {
    pub(super) fn open_map_composition(
        &mut self,
        binding: MapMaterialBinding,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut ServerContext,
    ) {
        let Some(map) = project.get_map(server).filter(|map| map.id == binding.map) else {
            Self::status(
                ui,
                ctx,
                "The selected map changed. Select a surface and try again.",
            );
            return;
        };
        let assigned = source(map, &binding);
        let base = if let Some(assigned) = assigned.as_ref() {
            let Some(id) = tile_id(assigned) else {
                Self::status(ui, ctx, "The surface needs a tile as its background.");
                return;
            };
            id
        } else {
            Self::status(ui, ctx, fl!("mz_blend_paint_first"));
            return;
        };
        if project.map_material_bindings.get(&base) == Some(&binding)
            && project.material_compositions.contains_key(&base)
            && (!is_rect_target(&binding.surface)
                || project
                    .regions
                    .iter()
                    .map(|region| rect_reference_count(&region.map, base))
                    .sum::<usize>()
                    <= 1)
        {
            self.open(base, ui, ctx, project);
            self.map_binding = Some(binding);
            return;
        }
        if !project
            .tiles
            .get(&base)
            .is_some_and(|tile| !tile.textures.is_empty())
        {
            Self::status(
                ui,
                ctx,
                "The selected surface needs a tile as its background.",
            );
            return;
        }
        let old_map = map.clone();
        // A copied surface's local stack becomes independent when editing starts.
        let composition = if project.material_compositions.contains_key(&base) {
            let Some(composition) = project.material_compositions.get(&base) else {
                Self::status(
                    ui,
                    ctx,
                    "The surface composition is missing its editable layers.",
                );
                return;
            };
            composition.clone()
        } else {
            let overlay = server.curr_tile_id.filter(|id| {
                *id != base
                    && project
                        .tiles
                        .get(id)
                        .is_some_and(|tile| !tile.textures.is_empty())
                    && !project.map_material_bindings.contains_key(id)
            });
            MaterialComposition {
                tiling: Default::default(),
                base,
                layers: overlay.map(CompositionLayer::split).into_iter().collect(),
            }
        };
        let size = pixel_style::bake_size(pixel_style::density(&project.config));
        let mut tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(size, size)]);
        tile.alias = "Surface composition".into();
        tile.set_default_materials();
        let id = tile.id;
        let bundle = match composition::bundle_for_surface(id, project, &composition, &binding) {
            Ok(bundle) => bundle,
            Err(error) => {
                Self::status(ui, ctx, error);
                return;
            }
        };
        let mut new_map = old_map.clone();
        if !set_source(&mut new_map, &binding, id) {
            Self::status(ui, ctx, "This material slot cannot be composed.");
            return;
        }
        let created = CreatedMapComposition {
            tile,
            composition,
            bundle,
            binding: binding.clone(),
        };
        created.install(project);
        *project.get_map_mut(server).unwrap() = new_map.clone();
        UNDOMANAGER.write().unwrap().add_undo(
            ProjectUndoAtom::MapMaterialCompositionCreate(
                server.pc,
                Box::new(old_map),
                Box::new(new_map),
                Box::new(created),
            ),
            ctx,
        );
        self.open(id, ui, ctx, project);
        self.map_binding = Some(binding);
        // Scene refresh is serviced by the host event loop, after the complete
        // surface/asset transaction is installed.
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Procedural Tiles Restored"),
            TheValue::Empty,
        ));
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Action List"),
            TheValue::Empty,
        ));
    }
    pub(super) fn follow_map_composition(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server: &ServerContext,
    ) {
        let selected = project.get_map(server).and_then(|map| {
            selection(map, server).and_then(|binding| {
                let id = source(map, &binding).as_ref().and_then(tile_id)?;
                (project.map_material_bindings.get(&id) == Some(&binding)).then_some((id, binding))
            })
        });
        if let Some((id, binding)) = selected {
            if self.tile != Some(id) || self.map_binding.as_ref() != Some(&binding) {
                self.open(id, ui, ctx, project);
                self.map_binding = Some(binding);
            }
        } else if self.map_binding.is_some() {
            self.clear_selection(ui, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Project, ServerContext, Uuid, Uuid) {
        let mut p = Project::default();
        let mut ids = vec![];
        for alias in ["Grass", "Road"] {
            let mut tile = rusterix::Tile::from_textures(vec![rusterix::Texture::alloc(4, 4)]);
            tile.alias = alias.into();
            let id = tile.id;
            p.procedural_materials
                .insert(id, starter(id).unwrap().save().unwrap());
            p.tiles.insert(id, tile);
            ids.push(id);
        }
        let mut region = Region::new();
        region.map = Map::default();
        for id in [1, 2] {
            let mut sector = rusterix::Sector::new(id, vec![]);
            sector
                .properties
                .set("source", Value::Source(PixelSource::TileId(ids[0])));
            region.map.sectors.push(sector);
        }
        region.map.selected_sectors = vec![1];
        let mut server = ServerContext::default();
        server.pc = ProjectContext::Region(region.id);
        server.curr_region = region.id;
        server.curr_map_tool_type = MapToolType::Sector;
        server.selected_hud_icon_index = 0;
        server.curr_tile_id = Some(ids[1]);
        server.curr_tile_source = Some(rusterix::TileSource::SingleTile(ids[1]));
        p.regions = vec![region];
        (p, server, ids[0], ids[1])
    }
    fn ui() -> (TheUI, TheContext) {
        super::super::tests::workspace_ui(1200, 850)
    }
    #[test]
    fn map_compose_accepts_ordinary_tiles_without_converting_the_sources() {
        let (mut p, mut server, a, b) = fixture();
        p.procedural_materials.clear();
        p.tiles.get_mut(&a).unwrap().textures[0] =
            rusterix::Texture::from_color([180, 80, 30, 255]);
        p.tiles.get_mut(&b).unwrap().textures[0] =
            rusterix::Texture::from_color([30, 180, 80, 128]);
        let originals = (p.tiles[&a].clone(), p.tiles[&b].clone());
        let (mut ui, mut ctx) = ui();
        let mut host = ProceduralMaterialEditor::default();
        let binding = selection(p.get_map(&server).unwrap(), &server).unwrap();
        host.open_map_composition(binding, &mut ui, &mut ctx, &mut p, &mut server);
        let id = host.tile.unwrap();
        assert_eq!(p.material_compositions[&id].base, a);
        assert_eq!(p.material_compositions[&id].layers[0].material, b);
        assert_eq!((&p.tiles[&a], &p.tiles[&b]), (&originals.0, &originals.1));
        assert!(
            !p.procedural_materials.contains_key(&a) && !p.procedural_materials.contains_key(&b)
        );
        EditableMaterialAsset::reopen(&p.procedural_materials[&id])
            .unwrap()
            .validate()
            .unwrap();
    }
    #[test]
    fn map_composition_is_local_persists_and_follows_map_selection() {
        let (mut p, mut server, grass, road) = fixture();
        let (mut ui, mut ctx) = ui();
        let mut host = ProceduralMaterialEditor::default();
        let binding = selection(p.get_map(&server).unwrap(), &server).unwrap();
        let grass_bundle = p.procedural_materials[&grass].clone();
        host.open_map_composition(binding.clone(), &mut ui, &mut ctx, &mut p, &mut server);
        let id = host.tile.unwrap();
        assert_ne!(id, grass);
        assert_ne!(id, road);
        assert_eq!(
            source(p.get_map(&server).unwrap(), &binding),
            Some(PixelSource::TileId(id))
        );
        assert_eq!(
            p.regions[0]
                .map
                .find_sector(2)
                .unwrap()
                .properties
                .get_default_source(),
            Some(&PixelSource::TileId(grass))
        );
        assert_eq!(p.material_compositions[&id].base, grass);
        assert_eq!(p.material_compositions[&id].layers[0].material, road);
        assert_eq!(server.curr_tile_id, Some(road));
        assert!(!p.tile_board_tiles.contains_key(&id));
        assert!(p.map_material_bindings.contains_key(&id));
        host.composition_view(&mut ui, &mut ctx, &p);
        assert!(ui.get_widget("Composition Surface").is_some());
        host.handle_event(
            &TheEvent::ValueChanged(
                TheId::named("Composition Position"),
                TheValue::Text("0.3".into()),
            ),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(p.material_compositions[&id].layers[0].position, 0.3);
        assert_eq!(p.procedural_materials[&grass], grass_bundle);
        server.curr_tile_id = Some(grass);
        host.handle_event(
            &TheEvent::Custom(TheId::named("Tile Picked"), TheValue::Id(grass)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(
            host.tile,
            Some(id),
            "Picking a source must keep the map composition open"
        );
        host.handle_event(
            &TheEvent::StateChanged(
                TheId::named("Composition Use Picked"),
                TheWidgetState::Clicked,
            ),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(p.material_compositions[&id].layers[0].material, grass);
        p.get_map_mut(&server).unwrap().selected_sectors = vec![2];
        host.follow_map_composition(&mut ui, &mut ctx, &p, &server);
        assert!(host.map_binding.is_none());
        p.get_map_mut(&server).unwrap().selected_sectors = vec![1];
        host.follow_map_composition(&mut ui, &mut ctx, &p, &server);
        assert_eq!(host.tile, Some(id));
        let restored: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(restored.map_material_bindings[&id], binding);
        assert_eq!(restored.material_compositions[&id].layers[0].position, 0.3);
        assert_eq!(ui.get_stack_layout(PREVIEW_STACK).unwrap().index(), 1);
    }
    #[test]
    #[ignore = "requires the scene renderer's GPU device for map undo/redo"]
    fn creation_undo_restores_surface_and_generated_assets_atomically() {
        let (mut p, mut server, grass, _) = fixture();
        let (mut ui, mut ctx) = ui();
        let mut host = ProceduralMaterialEditor::default();
        let old = p.get_map(&server).unwrap().clone();
        let binding = MapMaterialBinding {
            map: old.id,
            surface: CompositionSurface::RectTile { sector: 1 },
        };
        server.curr_map_tool_type = MapToolType::Rect;
        server.rect_compose_mode = true;
        server.rect_composition_target = Some(binding.clone());
        host.open_map_composition(binding.clone(), &mut ui, &mut ctx, &mut p, &mut server);
        let id = host.tile.unwrap();
        let new = p.get_map(&server).unwrap().clone();
        let created = CreatedMapComposition {
            tile: p.tiles[&id].clone(),
            composition: p.material_compositions[&id].clone(),
            bundle: p.procedural_materials[&id].clone(),
            binding: binding.clone(),
        };
        let atom = ProjectUndoAtom::MapMaterialCompositionCreate(
            server.pc,
            Box::new(old),
            Box::new(new),
            Box::new(created),
        );
        p.name = "Keep this rename".into();
        atom.undo(&mut p, &mut ui, &mut ctx, &mut server);
        assert!(!p.tiles.contains_key(&id));
        assert!(!p.material_compositions.contains_key(&id));
        assert!(!p.map_material_bindings.contains_key(&id));
        assert_eq!(
            source(p.get_map(&server).unwrap(), &binding),
            Some(PixelSource::TileId(grass))
        );
        assert_eq!(p.name, "Keep this rename");
        atom.redo(&mut p, &mut ui, &mut ctx, &mut server);
        assert_eq!(
            source(p.get_map(&server).unwrap(), &binding),
            Some(PixelSource::TileId(id))
        );
        assert!(p.map_material_bindings.contains_key(&id));
        assert!(!p.tile_board_tiles.contains_key(&id));
        assert_eq!(p.name, "Keep this rename");
        host.open_map_composition(binding.clone(), &mut ui, &mut ctx, &mut p, &mut server);
        host.handle_event(
            &TheEvent::Custom(TheId::named("Procedural Tiles Restored"), TheValue::Empty),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !p.procedural_material_cache_keys.contains_key(&id)
            && std::time::Instant::now() < deadline
        {
            host.update(&mut ui, &mut ctx, &mut p, &mut server);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            p.procedural_material_cache_keys.contains_key(&id),
            "The map-local composition must be baked by the GPU worker"
        );
        assert!(p.tiles[&id].textures[0].data.iter().any(|value| *value > 0));
        assert_eq!(
            source(p.get_map(&server).unwrap(), &binding),
            Some(PixelSource::TileId(id))
        );
    }
    #[test]
    fn copied_surface_becomes_independent_and_stale_requests_do_not_edit_maps() {
        let (mut p, mut server, _, _) = fixture();
        let (mut ui, mut ctx) = ui();
        let mut host = ProceduralMaterialEditor::default();
        let binding = selection(p.get_map(&server).unwrap(), &server).unwrap();
        host.open_map_composition(binding, &mut ui, &mut ctx, &mut p, &mut server);
        let first = host.tile.unwrap();
        p.get_map_mut(&server)
            .unwrap()
            .find_sector_mut(2)
            .unwrap()
            .properties
            .set("source", Value::Source(PixelSource::TileId(first)));
        p.get_map_mut(&server).unwrap().selected_sectors = vec![2];
        let binding = selection(p.get_map(&server).unwrap(), &server).unwrap();
        host.open_map_composition(binding.clone(), &mut ui, &mut ctx, &mut p, &mut server);
        let second = host.tile.unwrap();
        assert_ne!(first, second);
        assert_eq!(
            p.material_compositions[&first],
            p.material_compositions[&second]
        );
        host.handle_event(
            &TheEvent::ValueChanged(TheId::named("Composition Position"), TheValue::Float(0.8)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(p.material_compositions[&first].layers[0].position, 0.5);
        assert_eq!(p.material_compositions[&second].layers[0].position, 0.8);
        let count = p.tiles.len();
        let mut stale = binding;
        stale.map = Uuid::new_v4();
        host.open_map_composition(stale, &mut ui, &mut ctx, &mut p, &mut server);
        assert_eq!(p.tiles.len(), count);
    }
    #[test]
    fn surface_targets_respect_face_priority_wall_rows_and_ceiling() {
        let (mut p, mut server, grass, _) = fixture();
        let map = p.get_map_mut(&server).unwrap();
        let mut object = rusterix::GeometryObject::box_("Box", Vec3::zero(), Vec3::one());
        for face in &mut object.faces {
            face.tile = Some(PixelSource::TileId(grass));
        }
        let object_id = object.id;
        map.geometry_objects.push(object);
        map.selected_geometry_faces = vec![(object_id, 2)];
        map.selected_geometry_objects = vec![object_id];
        let binding = selection(map, &server).unwrap();
        let id = Uuid::new_v4();
        assert!(set_source(map, &binding, id));
        for (index, face) in map.geometry_objects[0].faces.iter().enumerate() {
            assert_eq!(
                face.tile,
                Some(PixelSource::TileId(if index == 2 { id } else { grass }))
            );
        }
        map.selected_geometry_faces.push((object_id, 3));
        assert!(selection(map, &server).is_none());
        map.selected_geometry_faces.clear();
        map.selected_geometry_objects.clear();
        server.selected_hud_icon_index = 1;
        let binding = selection(map, &server).unwrap();
        assert!(set_source(map, &binding, id));
        assert_eq!(
            map.find_sector(1).unwrap().properties.get_default_source(),
            Some(&PixelSource::TileId(grass))
        );
        assert_eq!(
            map.find_sector(1)
                .unwrap()
                .properties
                .get_source("ceiling_source"),
            Some(&PixelSource::TileId(id))
        );
        map.linedefs.push(rusterix::Linedef::new(7, 0, 1));
        map.selected_linedefs = vec![7];
        server.curr_map_tool_type = MapToolType::Linedef;
        server.selected_hud_icon_index = 2;
        let binding = selection(map, &server).unwrap();
        assert!(set_source(map, &binding, id));
        assert_eq!(
            map.find_linedef(7)
                .unwrap()
                .properties
                .get_source("row3_source"),
            Some(&PixelSource::TileId(id))
        );
        assert!(
            map.find_linedef(7)
                .unwrap()
                .properties
                .get_source("row1_source")
                .is_none()
        );
    }

    #[test]
    fn rect_geometry_tile_composition_preserves_face_and_neighbour_cells() {
        let (mut p, mut server, grass, road) = fixture();
        let (mut ui, mut ctx) = ui();
        let mut object =
            rusterix::GeometryObject::box_("Wall", Vec3::zero(), Vec3::new(3., 2., 1.));
        let object_id = object.id;
        object.faces[0].tile = Some(PixelSource::TileId(grass));
        object.faces[0]
            .tiles
            .insert((1, 0), PixelSource::TileId(road));
        p.get_map_mut(&server)
            .unwrap()
            .geometry_objects
            .push(object);
        let binding = MapMaterialBinding {
            map: p.get_map(&server).unwrap().id,
            surface: CompositionSurface::GeometryTile {
                object: object_id,
                face: 0,
                cell: [0, 0],
            },
        };
        server.curr_map_tool_type = MapToolType::Rect;
        server.rect_compose_mode = true;
        server.rect_composition_target = Some(binding.clone());
        let mut host = ProceduralMaterialEditor::default();
        host.open_map_composition(binding.clone(), &mut ui, &mut ctx, &mut p, &mut server);
        let id = host.tile.unwrap();
        let face = &p.get_map(&server).unwrap().geometry_objects[0].faces[0];
        assert_eq!(face.tile, Some(PixelSource::TileId(grass)));
        assert_eq!(face.tiles.get(&(1, 0)), Some(&PixelSource::TileId(road)));
        assert_eq!(face.tiles.get(&(0, 0)), Some(&PixelSource::TileId(id)));
        assert_eq!(p.material_compositions[&id].base, grass);
        server.rect_tile_id_3d = (2, 1);
        host.follow_map_composition(&mut ui, &mut ctx, &p, &server);
        assert_eq!(
            host.tile,
            Some(id),
            "Hovering another cell must not change the clicked editing target"
        );
        host.handle_event(
            &TheEvent::ValueChanged(TheId::named("Composition Position"), TheValue::Float(0.2)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(p.material_compositions[&id].layers[0].position, 0.2);
        let restored: Project = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(restored.map_material_bindings[&id], binding);
        server.rect_composition_target = Some(MapMaterialBinding {
            map: binding.map,
            surface: CompositionSurface::GeometryTile {
                object: object_id,
                face: 0,
                cell: [1, 0],
            },
        });
        host.follow_map_composition(&mut ui, &mut ctx, &p, &server);
        assert!(host.map_binding.is_none());
    }
    #[test]
    fn terrain_and_sector_tile_overrides_preserve_other_cells_and_blend_shapes() {
        let (mut p, server, grass, road) = fixture();
        let map = p.get_map_mut(&server).unwrap();
        let new = Uuid::new_v4();
        let preset = rusterix::VertexBlendPreset::from_index(1).unwrap();
        let properties = &mut map.find_sector_mut(1).unwrap().properties;
        properties.set(
            "tiles",
            Value::TileOverrides([((1, 0), PixelSource::TileId(road))].into_iter().collect()),
        );
        properties.set(
            "blend_tiles",
            Value::BlendOverrides(
                [((0, 0), (preset, PixelSource::TileId(grass)))]
                    .into_iter()
                    .collect(),
            ),
        );
        let binding = MapMaterialBinding {
            map: map.id,
            surface: CompositionSurface::SectorTile {
                sector: 1,
                cell: [0, 0],
            },
        };
        assert_eq!(source(map, &binding), Some(PixelSource::TileId(grass)));
        assert!(set_source(map, &binding, new));
        let properties = &map.find_sector(1).unwrap().properties;
        if let Some(Value::BlendOverrides(tiles)) = properties.get("blend_tiles") {
            assert_eq!(tiles[&(0, 0)], (preset, PixelSource::TileId(new)));
        } else {
            panic!("Existing paint blend coverage must be retained");
        }
        assert_eq!(
            tile_override(properties, [1, 0]),
            Some(PixelSource::TileId(road))
        );
        assert_eq!(
            properties.get_default_source(),
            Some(&PixelSource::TileId(grass))
        );
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set(
            "default_terrain_tile",
            Value::Source(PixelSource::TileId(grass)),
        );
        map.properties.set(
            "tiles",
            Value::TileOverrides([((1, 0), PixelSource::TileId(road))].into_iter().collect()),
        );
        let binding = MapMaterialBinding {
            map: map.id,
            surface: CompositionSurface::TerrainTile { cell: [-2, 3] },
        };
        assert!(set_source(map, &binding, new));
        assert_eq!(source(map, &binding), Some(PixelSource::TileId(new)));
        assert_eq!(
            tile_override(&map.properties, [1, 0]),
            Some(PixelSource::TileId(road))
        );
        assert_eq!(
            map.properties.get_source("default_terrain_tile"),
            Some(&PixelSource::TileId(grass))
        );
    }
    #[test]
    fn rect_2d_tile_stack_is_independent_of_other_painted_tiles() {
        let (mut p, mut server, grass, _) = fixture();
        let (mut ui, mut ctx) = ui();
        let binding = MapMaterialBinding {
            map: p.get_map(&server).unwrap().id,
            surface: CompositionSurface::RectTile { sector: 1 },
        };
        server.curr_map_tool_type = MapToolType::Rect;
        server.rect_compose_mode = true;
        server.rect_composition_target = Some(binding.clone());
        let mut host = ProceduralMaterialEditor::default();
        host.open_map_composition(binding, &mut ui, &mut ctx, &mut p, &mut server);
        let id = host.tile.unwrap();
        EditableMaterialAsset::reopen(&p.procedural_materials[&id])
            .unwrap()
            .validate()
            .unwrap();
        assert_eq!(
            p.get_map(&server)
                .unwrap()
                .find_sector(1)
                .unwrap()
                .properties
                .get_default_source(),
            Some(&PixelSource::TileId(id))
        );
        assert_eq!(
            p.get_map(&server)
                .unwrap()
                .find_sector(2)
                .unwrap()
                .properties
                .get_default_source(),
            Some(&PixelSource::TileId(grass))
        );
        host.handle_event(
            &TheEvent::ValueChanged(TheId::named("Composition Position"), TheValue::Float(0.75)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(p.material_compositions[&id].layers[0].position, 0.75);
        assert!(p.map_material_bindings.contains_key(&id));
        assert!(!p.tile_board_tiles.contains_key(&id));
    }

    #[test]
    fn editing_original_rect_tile_detaches_an_eyedropper_copy() {
        let (mut p, mut server, _, _) = fixture();
        let (mut ui, mut ctx) = ui();
        let mut host = ProceduralMaterialEditor::default();
        let binding = MapMaterialBinding {
            map: p.get_map(&server).unwrap().id,
            surface: CompositionSurface::RectTile { sector: 1 },
        };
        server.curr_map_tool_type = MapToolType::Rect;
        server.rect_compose_mode = true;
        server.rect_composition_target = Some(binding.clone());
        host.open_map_composition(binding.clone(), &mut ui, &mut ctx, &mut p, &mut server);
        let first = host.tile.unwrap();
        p.get_map_mut(&server)
            .unwrap()
            .find_sector_mut(2)
            .unwrap()
            .properties
            .set("source", Value::Source(PixelSource::TileId(first)));
        host.open_map_composition(binding.clone(), &mut ui, &mut ctx, &mut p, &mut server);
        let detached = host.tile.unwrap();
        assert_ne!(detached, first);
        host.handle_event(
            &TheEvent::ValueChanged(TheId::named("Composition Position"), TheValue::Float(0.1)),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        assert_eq!(p.material_compositions[&first].layers[0].position, 0.5);
        assert_eq!(p.material_compositions[&detached].layers[0].position, 0.1);
        assert_eq!(
            p.get_map(&server)
                .unwrap()
                .find_sector(2)
                .unwrap()
                .properties
                .get_default_source(),
            Some(&PixelSource::TileId(first))
        );
    }
    #[test]
    #[ignore = "requires a GPU device for Rect tile composition baking and minimap"]
    fn rect_3d_cell_gpu_bakes_and_previews_without_replacing_the_face() {
        let (mut p, mut server, grass, road) = fixture();
        let (mut ui, mut ctx) = ui();
        ui.init(&mut ctx);
        let mut object =
            rusterix::GeometryObject::box_("Wall", Vec3::zero(), Vec3::new(3., 2., 1.));
        let object_id = object.id;
        object.faces[0].tile = Some(PixelSource::TileId(grass));
        object.faces[0]
            .tiles
            .insert((1, 0), PixelSource::TileId(road));
        p.get_map_mut(&server)
            .unwrap()
            .geometry_objects
            .push(object);
        let binding = MapMaterialBinding {
            map: p.get_map(&server).unwrap().id,
            surface: CompositionSurface::GeometryTile {
                object: object_id,
                face: 0,
                cell: [0, 0],
            },
        };
        server.curr_map_tool_type = MapToolType::Rect;
        server.rect_compose_mode = true;
        server.editor_view_mode = EditorViewMode::FirstP;
        server.rect_composition_target = Some(binding.clone());
        let mut host = ProceduralMaterialEditor::default();
        host.open_map_composition(binding.clone(), &mut ui, &mut ctx, &mut p, &mut server);
        let id = host.tile.unwrap();
        host.handle_event(
            &TheEvent::Custom(TheId::named("Procedural Tiles Restored"), TheValue::Empty),
            &mut ui,
            &mut ctx,
            &mut p,
            &mut server,
        );
        let mut pixels = vec![0; 1200 * 850 * 4];
        ui.draw(&mut pixels, &mut ctx);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while host.frame.is_none() && std::time::Instant::now() < deadline {
            host.update(&mut ui, &mut ctx, &mut p, &mut server);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(host.frame.is_some());
        assert!(p.procedural_material_cache_keys.contains_key(&id));
        assert!(!p.tiles[&id].baked_material_data.is_empty());
        host.preview_dirty = true;
        host.draw_preview(&mut ui, 0);
        ui.draw(&mut pixels, &mut ctx);
        let d = *ui.get_render_view(PREVIEW).unwrap().dim();
        let at = ((d.y + d.height / 2) as usize * 1200 + (d.x + d.width / 2) as usize) * 4;
        assert!(pixels[at..at + 3].iter().any(|c| *c > 100));
        let face = &p.get_map(&server).unwrap().geometry_objects[0].faces[0];
        assert_eq!(face.tile, Some(PixelSource::TileId(grass)));
        assert_eq!(face.tiles.get(&(1, 0)), Some(&PixelSource::TileId(road)));
        assert_eq!(face.tiles.get(&(0, 0)), Some(&PixelSource::TileId(id)));
    }
}
