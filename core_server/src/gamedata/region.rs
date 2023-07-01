use core_shared::prelude::*;
use crate::prelude::*;

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

pub struct GameRegion {
    pub name            : String,
    pub path            : PathBuf,
    pub region_path     : PathBuf,
    pub data            : GameRegionData,
    pub behaviors       : Vec<GameBehavior>,
    pub displacements   : FxHashMap<(isize, isize), TileData>,

    pub procedural      : Option<GameBehavior>,

    pub undo            : UndoStack,
}

impl GameRegion {
    pub fn new(path: &PathBuf, region_path: &PathBuf) -> Self {
        let name = path::Path::new(&path).file_stem().unwrap().to_str().unwrap();

        // Gets the content of the settings file
        let level1_path = path.join( format!("{}{}", "level1", ".json"));

        let contents = fs::read_to_string( level1_path )
                .unwrap_or("".to_string());

        let mut data = serde_json::from_str(&contents)
                .unwrap_or(GameRegionData {
                    layer1          : FxHashMap::default(),
                    layer2          : FxHashMap::default(),
                    layer3          : FxHashMap::default(),
                    layer4          : FxHashMap::default(),
                    id              : Uuid::new_v4(),
                    curr_pos        : (0,0),
                    min_pos         : (10000,10000),
                    max_pos         : (-10000, -10000),
                    areas           : vec![],
                    editor_offset   : None,
                    settings        : PropertySink::new(),
                });

        update_region_sink(&mut data.settings);

        // Read the behaviors
        let mut behaviors : Vec<GameBehavior> = vec![];

        for a in &data.areas {
            let mut area_path = path.clone();
            let name = format!("area_{}.json", a.id);
            let path = std::path::Path::new(&name).to_path_buf();
            area_path.push(path.clone());
            let behavior = GameBehavior::load_from_path(&area_path, &area_path);
            behaviors.push(behavior);
        }

        let procedural_path = path.join(std::path::Path::new(&"procedural.json").to_path_buf());
        let procedural = Some(GameBehavior::load_from_path(&procedural_path, &procedural_path));

        Self {
            name                : name.to_string(),
            path                : path.clone(),
            region_path         : region_path.clone(),
            data,
            behaviors,
            displacements       : FxHashMap::default(),
            procedural,

            undo                : UndoStack::new(),
        }
    }

    #[cfg(feature = "embed_binaries")]
    pub fn new_from_embedded(file_name: &str) -> Self {

        let mut data = GameRegionData {
            layer1          : FxHashMap::default(),
            layer2          : FxHashMap::default(),
            layer3          : FxHashMap::default(),
            layer4          : FxHashMap::default(),
            id              : Uuid::new_v4(),
            curr_pos        : (0,0),
            min_pos         : (10000,10000),
            max_pos         : (-10000, -10000),
            areas           : vec![],
            editor_offset   : None,
            settings        : PropertySink::new(),
        };

        update_region_sink(&mut data.settings);

        if let Some(bytes) = Embedded::get(file_name) {
            if let Some(string) = std::str::from_utf8(bytes.data.as_ref()).ok() {
                data = serde_json::from_str(&string).unwrap();
            }
        }

        // Read the behaviors
        let mut behaviors : Vec<GameBehavior> = vec![];
        let procedural : Option<GameBehavior> = None;

        let mut spl = file_name.split("/");
        spl.next();
        spl.next();
        let name = spl.next().unwrap();

        for a in &data.areas {
            let name = format!("game/regions/{}/area_{}.json", name, a.id);

            let behavior = GameBehavior::load_from_embedded(name.as_str());
            behaviors.push(behavior);
        }

        Self {
            name                : name.to_string(),
            path                : std::path::PathBuf::new(),
            region_path         : std::path::PathBuf::new(),
            data,
            behaviors,
            displacements       : FxHashMap::default(),
            procedural,

            undo                : UndoStack::new(),
        }
    }

    /// Save the region to file
    pub fn save_data(&self) {
        let json_path = self.path.join( format!("{}{}", "level1", ".json"));
        if let Some(json) = serde_json::to_string(&self.data).ok() {
            fs::write(json_path, json)
            .expect("Unable to write region file");
        }
    }

    /// Returns which layer has a tile for this position
    pub fn get_layer_mask(&self, pos: (isize, isize)) -> Vec<Option<TileData>> {
        let mut rc = vec![];

        if let Some(t) = self.data.layer1.get(&pos) {
            rc.push(Some(t.clone()));
        } else {
            rc.push(None)
        }
        if let Some(t) = self.data.layer2.get(&pos) {
            rc.push(Some(t.clone()));
        }  else {
            rc.push(None)
        }
        if let Some(t) = self.data.layer3.get(&pos) {
            rc.push(Some(t.clone()));
        } else {
            rc.push(None)
        }
        if let Some(t) = self.data.layer4.get(&pos) {
            rc.push(Some(t.clone()));
        } else {
            rc.push(None)
        }

        rc
    }

    /// Returns the layered tiles at the given position and checks for displacements
    pub fn get_value(&self, pos: (isize, isize)) -> Vec<TileData> {
        let mut rc = vec![];

        if let Some(t) = self.displacements.get(&pos) {
            rc.push(t.clone());
        } else {
            if let Some(t) = self.data.layer1.get(&pos) {
                rc.push(t.clone());
            }
            if let Some(t) = self.data.layer2.get(&pos) {
                rc.push(t.clone());
            }
            if let Some(t) = self.data.layer3.get(&pos) {
                rc.push(t.clone());
            }
        }
        rc
    }

    /// Returns the layered tiles at the given position including the overlay. This is for preview purposes only.
    pub fn get_value_overlay(&self, pos: (isize, isize)) -> Vec<TileData> {
        let mut rc = vec![];

        if let Some(t) = self.data.layer4.get(&pos) {
            rc.push(t.clone());
        } else {
            if let Some(t) = self.data.layer1.get(&pos) {
                rc.push(t.clone());
            }
            if let Some(t) = self.data.layer2.get(&pos) {
                rc.push(t.clone());
            }
            if let Some(t) = self.data.layer3.get(&pos) {
                rc.push(t.clone());
            }
        }

        rc
    }

    /// Returns the layered tiles at the given position
    pub fn get_value_without_displacements(&self, pos: (isize, isize)) -> Vec<TileData> {
        let mut rc = vec![];

        if let Some(t) = self.data.layer1.get(&pos) {
            rc.push(t.clone());
        }
        if let Some(t) = self.data.layer2.get(&pos) {
            rc.push(t.clone());
        }
        if let Some(t) = self.data.layer3.get(&pos) {
            rc.push(t.clone());
        }
        rc
    }

    /// Sets a value at the given position
    pub fn set_value(&mut self, layer: usize, pos: (isize, isize), value: TileData) {
        if layer == 1 {
            self.data.layer1.insert(pos, value);
        } else
        if layer == 2 {
            self.data.layer2.insert(pos, value);
        } else
        if layer == 3 {
            self.data.layer3.insert(pos, value);
        } else
        if layer == 4 {
            self.data.layer4.insert(pos, value);
        }

        if self.data.min_pos.0 > pos.0 {
            self.data.min_pos.0 = pos.0;
        }
        if self.data.min_pos.1 > pos.1 {
            self.data.min_pos.1 = pos.1;
        }
        if self.data.max_pos.0 < pos.0 {
            self.data.max_pos.0 = pos.0;
        }
        if self.data.max_pos.1 < pos.1 {
            self.data.max_pos.1 = pos.1;
        }
    }

    /// Clears the value at the given position
    pub fn clear_value(&mut self, pos: (isize, isize)) {
        self.data.layer1.remove(&pos);
        self.data.layer2.remove(&pos);
        self.data.layer3.remove(&pos);
        self.data.layer4.remove(&pos);
    }

    /// Clears the value at the given position for the given layer
    pub fn clear_layer_value(&mut self, layer: usize, pos: (isize, isize)) {
        if layer == 1 {
            self.data.layer1.remove(&pos);
        } else
        if layer == 2 {
            self.data.layer2.remove(&pos);
        } else
        if layer == 3 {
            self.data.layer3.remove(&pos);
        } else
        if layer == 4 {
            self.data.layer4.remove(&pos);
        }
    }

    /// Calculates the min / max positions
    pub fn calc_dimensions(&mut self) {
        let mut min_pos = (10000, 10000);
        let mut max_pos = (-10000, -10000);

        //let mut to_clear = vec![];
        for (pos, _tile)  in &self.data.layer1 {

            // Temp cleaning due to an old, fixed bug
            //if pos.0 > 100000 || pos.1 > 100000 {
            //    to_clear.push(pos.clone());
            //}

            if min_pos.0 > pos.0 {
                min_pos.0 = pos.0;
            }
            if min_pos.1 > pos.1 {
                min_pos.1 = pos.1;
            }
            if max_pos.0 < pos.0 {
                max_pos.0 = pos.0;
            }
            if max_pos.1 < pos.1 {
                max_pos.1 = pos.1;
            }
        }

        // for p in to_clear {
        //     self.clear_value(p);
        //     println!("cleared {:?}", p);
        //     self.save_data();
        // }

        self.data.min_pos = min_pos;
        self.data.max_pos = max_pos;
    }

    /// Calculates the offset for center of the area for the given visible size
    pub fn get_center_offset_for_visible_size(&self, visible_tiles: (usize, usize)) -> (isize, isize) {
        let x = self.data.min_pos.0 + (self.data.max_pos.0 - self.data.min_pos.0) / 2 - visible_tiles.0 as isize / 2;
        let y = self.data.min_pos.1 + (self.data.max_pos.1 - self.data.min_pos.1) / 2 - visible_tiles.1 as isize / 2;
        (x, y)
    }

    /// Remaps the TileUsage field of the tiles
    pub fn remap(&mut self, asset: &mut Asset) {
        let mut tiles : FxHashMap<(isize, isize), TileData> = HashMap::default();

        // Layer 1
        let ids: Vec<&(isize, isize)> = self.data.layer1.keys().collect();
        for id in &ids {
            let value = &self.data.layer1[id];
            if let Some(tile) = asset.get_tile(&TileId::new(value.tilemap, value.x_off, value.y_off)) {
                tiles.insert(**id, TileData {
                    tilemap     : value.tilemap,
                    x_off       : value.x_off,
                    y_off       : value.y_off,
                    usage       : tile.usage.clone(),
                    size        : None,
                });
            }
        }
        self.data.layer1 = tiles;

        // Layer 2
        tiles = HashMap::default();
        let ids: Vec<&(isize, isize)> = self.data.layer2.keys().collect();
        for id in &ids {
            let value = &self.data.layer2[id];
            if let Some(tile) = asset.get_tile(&TileId::new(value.tilemap, value.x_off, value.y_off)) {
                tiles.insert(**id, TileData {
                    tilemap     : value.tilemap,
                    x_off       : value.x_off,
                    y_off       : value.y_off,
                    usage       : tile.usage.clone(),
                    size        : None,
                });
            }
        }
        self.data.layer2 = tiles;

        // Layer 3
        tiles = HashMap::default();
        let ids: Vec<&(isize, isize)> = self.data.layer3.keys().collect();
        for id in &ids {
            let value = &self.data.layer3[id];
            if let Some(tile) = asset.get_tile(&TileId::new(value.tilemap, value.x_off, value.y_off)) {
                tiles.insert(**id, TileData {
                    tilemap     : value.tilemap,
                    x_off       : value.x_off,
                    y_off       : value.y_off,
                    usage       : tile.usage.clone(),
                    size        : None,
                });
            }
        }
        self.data.layer3 = tiles;

        // Layer 4
        tiles = HashMap::default();
        let ids: Vec<&(isize, isize)> = self.data.layer4.keys().collect();
        for id in &ids {
            let value = &self.data.layer4[id];
            if let Some(tile) = asset.get_tile(&TileId::new(value.tilemap, value.x_off, value.y_off)) {
                tiles.insert(**id, TileData {
                    tilemap     : value.tilemap,
                    x_off       : value.x_off,
                    y_off       : value.y_off,
                    usage       : tile.usage.clone(),
                    size        : None,
                });
            }
        }
        self.data.layer4 = tiles;

        self.save_data();
    }

    /// Create area
    pub fn create_area(&mut self, name: String) -> Uuid {

        let area_id = Uuid::new_v4();
        let mut path = self.path.clone();
        path.push(format!("area_{}.json", area_id));

        let behavior = GameBehavior::load_from_path(&path, &path);
        let behavior_id = behavior.data.id.clone();
        behavior.save_data();
        self.behaviors.push(behavior);

        let area = RegionArea { name, area: vec![], behavior: behavior_id.clone(), id: area_id };
        self.data.areas.push(area);

        self.save_data();

        behavior_id
    }

    /// Deletes the given area
    pub fn delete_area(&mut self, index: usize) {

        let behavior_id = self.data.areas[index].behavior;
        let mut behavior_index : Option<usize> = None;

        for (index, b) in self.behaviors.iter().enumerate() {
            if b.data.id == behavior_id {
                behavior_index = Some(index);
                break;
            }
        }

        self.data.areas.remove(index);
        if let Some(behavior_index) = behavior_index {
            let _ = std::fs::remove_file(self.behaviors[behavior_index].path.clone());
            self.behaviors.remove(behavior_index);
        }
        self.save_data();
    }

    /// Deletes all areas
    pub fn delete_areas(&mut self) {
        while self.data.areas.is_empty() == false {
            self.delete_area(0);
        }
    }

    /// Get area names
    pub fn get_area_names(&self) -> Vec<String> {
        let mut names : Vec<String> = vec![];
        for area in &self.data.areas {
            names.push(area.name.clone());
        }
        names
    }

    /// Rename the region
    pub fn rename(&mut self, name: String) {
        self.name = name.clone();
        if std::fs::rename(self.path.clone(), self.region_path.join(name.clone())).is_ok() {
            _ = std::fs::remove_file(self.path.clone());
            self.path = self.region_path.join(name);
        }
    }

    // Undo / Redo

    pub fn is_undo_available(&self) -> bool {
        self.undo.has_undo()
    }

    pub fn is_redo_available(&self) -> bool {
        self.undo.has_redo()
    }

    pub fn undo(&mut self) {
        let undo = self.undo.undo();
        self.data = serde_json::from_str(&undo).unwrap();
        self.save_data();
    }

    pub fn redo(&mut self) {
        let redo = self.undo.redo();
        self.data = serde_json::from_str(&redo).unwrap();
        self.save_data();
    }

    /// Get the region data as string
    pub fn get_data(&self) -> String {
        if let Some(json) = serde_json::to_string(&self.data).ok() {
            json
        } else {
            "".to_string()
        }
    }
}

// Generate region sink

pub fn update_region_sink(sink: &mut PropertySink) {

    if sink.contains("background") == false {
        sink.properties.insert(0,Property::new_color("background".to_string(), "#000000".to_string()));
    }

    if sink.contains("movement") == false {
        sink.properties.insert(1,Property::new_string("movement".to_string(), "tile".to_string()));
    }

    if sink.contains("lighting") == false {
        sink.push(Property::new_string("lighting".to_string(), "timeofday".into()));
    }

    if sink.contains("base_lighting") == false {
        sink.push(Property::new_float("base_lighting".to_string(), 0.5));
    }

    if sink.contains("visibility") == false {
        sink.push(Property::new_string("visibility".to_string(), "full".to_string()));
    }

    if sink.contains("visible_distance") == false {
        sink.push(Property::new_int("visible_distance".to_string(), 10));
    }

    if sink.contains("supports_3d") == false {
        sink.push(Property::new_bool("supports_3d".to_string(), false));
    }
}

pub fn generate_region_sink_descriptions() -> FxHashMap<String, Vec<String>> {
    let mut map : FxHashMap<String, Vec<String>> = HashMap::default();

    map.insert("background".to_string(), vec!["The background color of the region".to_string()]);
    map.insert("movement".to_string(), vec!["Use \"tile\" for tile based movement or \"pixel\" for sub-tile movement.".to_string()]);
    map.insert("lighting".to_string(), vec!["The lighting mode. \"timeofday\" for base_lighting + auto time of day lighting or \"basic\" for only using the base lighting (dungeons etc.).".to_string()]);
    map.insert("base_lighting".to_string(), vec!["The base lighting of the region. 0.0 for fully black and 1.0 for fully lit.".to_string()]);
    map.insert("visibility".to_string(), vec!["Use \"full\" for unlimited visibility or \"limited\" to enable the parameters below.".to_string()]);
    map.insert("visible_distance".to_string(), vec!["The visible distance in tiles. \"visibility\" has to be set to \"limited\".".to_string()]);
    map.insert("supports_3d".to_string(), vec!["Does this region support 3D ?".to_string()]);

    map
}
