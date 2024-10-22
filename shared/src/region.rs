use crate::prelude::*;
use theframework::prelude::*;

fn default_min_brightness() -> f32 {
    0.3
}

fn default_max_brightness() -> f32 {
    1.0
}

fn default_pathtracer_samples() -> i32 {
    30
}

fn default_tile_size() -> i32 {
    36
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum RegionType {
    Region2D,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum CameraMode {
    Pinhole,
    Orthogonal,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Region {
    pub id: Uuid,
    pub region_type: RegionType,

    pub name: String,

    #[serde(default)]
    pub regionfx: RegionFXObject,

    #[serde(with = "vectorize")]
    pub tiles: FxHashMap<(i32, i32), RegionTile>,

    #[serde(default)]
    pub geometry: FxHashMap<Uuid, GeoFXObject>,

    #[serde(skip)]
    pub compiled_geometry: FxHashMap<Uuid, FTContext>,

    #[serde(default)]
    #[serde(with = "vectorize")]
    pub geometry_areas: FxHashMap<Vec3i, Vec<Uuid>>,

    #[serde(default)]
    #[serde(with = "vectorize")]
    pub effects: FxHashMap<Vec3i, TileFXObject>,

    #[serde(default)]
    pub areas: FxHashMap<Uuid, Area>,

    #[serde(default)]
    pub characters: FxHashMap<Uuid, Character>,

    #[serde(default)]
    pub items: FxHashMap<Uuid, Item>,

    #[serde(default)]
    pub heightmap: Heightmap,

    #[serde(default)]
    pub prerendered: PreRendered,

    pub width: i32,
    pub height: i32,
    pub grid_size: i32,
    #[serde(default = "default_tile_size")]
    pub tile_size: i32,
    pub scroll_offset: Vec2i,
    pub zoom: f32,

    #[serde(default = "default_pathtracer_samples")]
    pub pathtracer_samples: i32,

    #[serde(default)]
    pub editing_position_3d: Vec3f,

    #[serde(default = "default_min_brightness")]
    pub min_brightness: f32,

    #[serde(default = "default_max_brightness")]
    pub max_brightness: f32,

    #[serde(default)]
    pub property_1: String,
    #[serde(default)]
    pub property_2: String,
    #[serde(default)]
    pub property_3: String,
    #[serde(default)]
    pub property_4: String,
}

impl Default for Region {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Region {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Region {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            region_type: RegionType::Region2D,

            name: "New Region".to_string(),

            regionfx: RegionFXObject::default(),

            tiles: FxHashMap::default(),

            geometry: FxHashMap::default(),
            compiled_geometry: FxHashMap::default(),
            geometry_areas: FxHashMap::default(),

            effects: FxHashMap::default(),

            areas: FxHashMap::default(),
            characters: FxHashMap::default(),
            items: FxHashMap::default(),

            heightmap: Heightmap::default(),

            prerendered: PreRendered::default(),

            width: 80,
            height: 80,
            grid_size: 24,
            tile_size: 36,
            scroll_offset: Vec2i::zero(),
            zoom: 1.0,

            pathtracer_samples: default_pathtracer_samples(),

            editing_position_3d: Vec3f::zero(),

            min_brightness: 0.3,
            max_brightness: 1.0,

            property_1: String::default(),
            property_2: String::default(),
            property_3: String::default(),
            property_4: String::default(),
        }
    }

    /// Calculate the min / max positions of the tiles.
    pub fn min_max(&self) -> Option<(Vec2i, Vec2i)> {
        if self.tiles.is_empty() {
            return None;
        }

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        for &(x, y) in self.tiles.keys() {
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }

        Some((Vec2i::new(min_x, min_y), Vec2i::new(max_x, max_y)))
    }

    /// Set the tile of the given position and role.
    pub fn set_tile(&mut self, pos: (i32, i32), role: Layer2DRole, tile: Option<Uuid>) {
        if role == Layer2DRole::FX {
            return;
        }
        if let Some(t) = self.tiles.get_mut(&pos) {
            t.layers[role as usize] = tile;
        } else {
            let mut region_tile = RegionTile::default();
            region_tile.layers[role as usize] = tile;
            self.tiles.insert(pos, region_tile);
        }
    }

    /// Add a geometry node.
    pub fn add_geo_node(&mut self, geo: GeoFXNode) -> Uuid {
        let mut geo_obj = GeoFXObject::default();
        geo_obj.nodes.push(geo);
        geo_obj.update_area();
        let geo_obj_id = geo_obj.id;
        self.geometry.insert(geo_obj_id, geo_obj);

        self.update_geometry_areas();

        geo_obj_id
    }

    /// Get the closest geometry to the given 2D point.
    pub fn get_closest_geometry(&self, p: Vec2f, role: Layer2DRole) -> Option<(Uuid, usize)> {
        let pi = Vec2i::from(p);
        let mut rc = None;

        for geo_obj in self.geometry.values() {
            if geo_obj.area.contains(&pi) && geo_obj.get_layer_role() == Some(role) {
                rc = Some((geo_obj.id, 0));
            }
        }

        rc
    }

    /// Collects the area which needs to be rerendered if the given material changes.
    pub fn get_material_area(&self, material_id: Uuid, material_index: usize) -> Vec<Vec2i> {
        let mut areas = FxHashSet::default();
        for (_, geo_obj) in self.geometry.iter() {
            if geo_obj.material_id == material_id {
                for p2d in &geo_obj.area {
                    areas.insert(*p2d);
                }
            }
        }
        // Iterate the heightfield material masks and check for the material.
        let u_id = (material_index + 1) as u8;
        for (key, buffer) in &self.heightmap.material_mask {
            let rgb_slices: Vec<&[u8]> = buffer
                .pixels()
                .chunks(3)
                .filter(|chunk| chunk.len() == 3)
                .collect();
            for chunk in rgb_slices {
                if chunk[0] == u_id || chunk[1] == u_id {
                    areas.insert(vec2i(key.0, key.1));
                    break;
                }
            }
        }
        areas.into_iter().collect()
    }

    /// Update the geometry areas.
    pub fn update_geometry_areas(&mut self) {
        self.geometry_areas.clear();
        for (id, geo_obj) in self.geometry.iter_mut() {
            let height = max(geo_obj.height, 1);
            geo_obj.update_area();
            for p2d in &geo_obj.area {
                for h in 0..height {
                    let p3d = Vec3i::new(p2d.x, geo_obj.level + h, p2d.y);

                    if let Some(list) = self.geometry_areas.get_mut(&p3d) {
                        list.push(*id);
                    } else {
                        self.geometry_areas.insert(p3d, vec![*id]);
                    }
                }
            }
        }
    }

    /// Finds the geo node of the given id.
    pub fn find_geo_node(&mut self, id: Uuid) -> Option<(&mut GeoFXObject, usize)> {
        for geo_obj in self.geometry.values_mut() {
            for (index, geo) in geo_obj.nodes.iter().enumerate() {
                if geo.id == id {
                    return Some((geo_obj, index));
                }
            }
        }
        None
    }

    /// Returns true if the character can move to the given position.
    pub fn can_move_to(
        &self,
        pos: Vec2i,
        tiles: &FxHashMap<Uuid, TheRGBATile>,
        update: &RegionUpdate,
    ) -> bool {
        let mut can_move = true;

        if pos.x < 0 || pos.y < 0 {
            return false;
        }

        if pos.x >= self.width || pos.y >= self.height {
            return false;
        }

        if let Some(tile) = self.tiles.get(&(pos.x, pos.y)) {
            for index in 0..tile.layers.len() {
                if let Some(layer) = tile.layers[index] {
                    if let Some(t) = tiles.get(&layer) {
                        if t.blocking && index == Layer2DRole::Wall as usize {
                            can_move = false;

                            if let Some(wallfx) = update.wallfx.get(&(pos.x, pos.y)) {
                                if wallfx.fx != WallFX::Normal {
                                    can_move = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check if the geometry for the tile is blocking
        if can_move {
            for geo_obj in self.geometry.values() {
                if geo_obj.area.contains(&pos) && geo_obj.is_blocking() {
                    can_move = false;
                    break;
                }
            }
        }

        can_move
    }

    pub fn distance(&self, x: Vec2i, y: Vec2i) -> f32 {
        distance(Vec2f::from(x), Vec2f::from(y))
    }

    /// Fills a code level with the blocking tiles of the region.
    pub fn fill_code_level(
        &self,
        level: &mut Level,
        tiles: &FxHashMap<Uuid, TheRGBATile>,
        update: &RegionUpdate,
        region: &Region,
    ) {
        level.clear();

        for y in 0..self.height {
            for x in 0..self.width {
                let mut can_move = true;
                let pos = vec2i(x, y);

                if let Some(tile) = self.tiles.get(&(pos.x, pos.y)) {
                    for index in 0..tile.layers.len() {
                        if let Some(layer) = tile.layers[index] {
                            if let Some(t) = tiles.get(&layer) {
                                if t.blocking && index == Layer2DRole::Wall as usize {
                                    can_move = false;

                                    if let Some(wallfx) = update.wallfx.get(&(pos.x, pos.y)) {
                                        if wallfx.fx != WallFX::Normal {
                                            can_move = true;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // // If the tile contains a light, add it.
                    // if let Some(timeline) = &tile.tilefx {
                    //     if timeline.contains_collection("Light Emitter") {
                    //         let light = TileFX::new_fx("Light Emitter", None);
                    //         let mut l = light.collection_cloned();
                    //         timeline.fill(&level.time, &mut l);
                    //         level.add_light(pos, l);
                    //     }
                    // }
                }

                if can_move {
                    for geo_obj in self.geometry.values() {
                        if geo_obj.area.contains(&pos) && geo_obj.is_blocking() {
                            can_move = false;
                        }
                    }
                }

                if !can_move {
                    level.set_blocking((x, y));
                }

                if let Some(fx) = region.effects.get(&vec3i(x, 0, y)) {
                    if let Some(props) = fx.get_light_collection() {
                        level.add_light(pos, props);
                    }
                }
            }
        }
    }

    /// Compile all ForgedTiles based geometry nodes.
    pub fn compile_geo_all(
        &mut self,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
    ) {
        let ft = ForgedTiles::default();
        for geo_obj in &mut self.geometry.values() {
            let source = geo_obj.build(palette, textures);
            match ft.compile_code(source) {
                Ok(ctx) => {
                    self.compiled_geometry.insert(geo_obj.id, ctx);
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            };
        }
    }

    /// Compile this specific geometry.
    pub fn compile_geo(
        &mut self,
        id: Uuid,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
    ) {
        let ft = ForgedTiles::default();
        if let Some(geo_obj) = self.geometry.get(&id) {
            let source = geo_obj.build(palette, textures);
            match ft.compile_code(source) {
                Ok(ctx) => {
                    self.compiled_geometry.insert(geo_obj.id, ctx);
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            };
        }
    }

    /// Create a region from json.
    pub fn from_json(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or(Region::new())
    }

    /// Convert the region to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum Layer2DRole {
    Ground,
    Wall,
    Ceiling,
    FX,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegionTile {
    // Tile layers
    pub layers: Vec<Option<Uuid>>,
}

impl Default for RegionTile {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionTile {
    pub fn new() -> Self {
        Self {
            layers: vec![None, None, None],
        }
    }
}
