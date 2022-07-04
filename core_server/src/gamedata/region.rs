use core_shared::property::Property;
use core_shared::property::PropertySink;
use rand::prelude::*;

use std::fs;
use std::path;
use std::path::PathBuf;

use std::collections::HashMap;

use core_shared::regiondata::{ GameRegionData, RegionArea };
use core_shared::asset::tileset::TileUsage;
use core_shared::asset::Asset;

use super::behavior::GameBehavior;

#[cfg(feature = "embed_binaries")]
use core_embed_binaries::Embedded;

pub struct GameRegion {
    pub name            : String,
    pub path            : PathBuf,
    pub region_path     : PathBuf,
    pub data            : GameRegionData,
    pub behaviors       : Vec<GameBehavior>,
    pub displacements   : HashMap<(isize, isize), (usize, usize, usize, TileUsage)>,
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
                    layer1      : HashMap::new(),
                    layer2      : HashMap::new(),
                    layer3      : HashMap::new(),
                    layer4      : HashMap::new(),
                    id          : thread_rng().gen_range(1..=u32::MAX) as usize,
                    curr_pos    : (0,0),
                    min_pos     : (10000,10000),
                    max_pos     : (-10000, -10000),
                    areas       : vec![],
                    settings    : PropertySink::new(),
                });

        update_region_sink(&mut data.settings);

        // Read the behaviors
        let mut behaviors : Vec<GameBehavior> = vec![];

        let file_name = path.file_stem().unwrap().to_str().unwrap().to_string();

        for a in &data.areas {
            let name = format!("game/regions/{}/area_{}.json", file_name, a.id);
            let path = std::path::Path::new(&name).to_path_buf();
            let behavior = GameBehavior::load_from_path(&path, &path);
            behaviors.push(behavior);
        }

        Self {
            name                : name.to_string(),
            path                : path.clone(),
            region_path         : region_path.clone(),
            data,
            behaviors,
            displacements       : HashMap::new(),
        }
    }

    #[cfg(feature = "embed_binaries")]
    pub fn new_from_embedded(file_name: &str) -> Self {

        let mut data = GameRegionData {
            layer1      : HashMap::new(),
            layer2      : HashMap::new(),
            layer3      : HashMap::new(),
            layer4      : HashMap::new(),
            id          : thread_rng().gen_range(1..=u32::MAX) as usize,
            curr_pos    : (0,0),
            min_pos     : (10000,10000),
            max_pos     : (-10000, -10000),
            areas       : vec![],
            settings    : PropertySink::new(),
        };

        update_region_sink(&mut data.settings);

        if let Some(bytes) = Embedded::get(file_name) {
            if let Some(string) = std::str::from_utf8(bytes.data.as_ref()).ok() {
                data = serde_json::from_str(&string).unwrap();
            }
        }

        // Read the behaviors
        let mut behaviors : Vec<GameBehavior> = vec![];

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
            displacements       : HashMap::new(),
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

    /// Returns the layered tiles at the given position and checks for displacements
    pub fn get_value(&self, pos: (isize, isize)) -> Vec<(usize, usize, usize, TileUsage)> {
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
            if let Some(t) = self.data.layer4.get(&pos) {
                rc.push(t.clone());
            }
        }
        rc
    }


    /// Returns the layered tiles at the given position
    pub fn get_value_without_displacements(&self, pos: (isize, isize)) -> Vec<(usize, usize, usize, TileUsage)> {
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
        if let Some(t) = self.data.layer4.get(&pos) {
            rc.push(t.clone());
        }
        rc
    }

    /// Sets a value at the given position
    pub fn set_value(&mut self, layer: usize, pos: (isize, isize), value: (usize, usize, usize, TileUsage)) {

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

    /// Calculates the min / max positions
    pub fn calc_dimensions(&mut self) {
        let mut min_pos = (10000, 10000);
        let mut max_pos = (-10000, -10000);

        for (pos, _tile)  in &self.data.layer1 {
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
        let mut tiles : HashMap<(isize, isize), (usize, usize, usize, TileUsage)> = HashMap::new();
        let ids: Vec<&(isize, isize)> = self.data.layer1.keys().collect();
        for id in &ids {
            let value = &self.data.layer1[id];
            let tile = asset.get_tile(&(value.0, value.1, value.2));

            tiles.insert(**id, (value.0, value.1, value.2, tile.usage));
        }
        self.data.layer1 = tiles;
        self.save_data();
    }

    /// Create area
    pub fn create_area(&mut self) -> usize {

        let area_id = thread_rng().gen_range(1..=u32::MAX) as usize;
        let mut path = self.path.clone();
        path.push(format!("area_{}.json", area_id));

        let behavior = GameBehavior::load_from_path(&path, &path);
        let behavior_id = behavior.data.id.clone();
        behavior.save_data();
        self.behaviors.push(behavior);

        let area = RegionArea { name: "New Area".to_string(), area: vec![], behavior: behavior_id.clone(), id: area_id };
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

        if let Some(behavior_index) = behavior_index {
            self.data.areas.remove(index);
            let _ = std::fs::remove_file(self.behaviors[behavior_index].path.clone());
            self.behaviors.remove(behavior_index);
        }
        self.save_data();
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

}

// Generate region sink

pub fn update_region_sink(sink: &mut PropertySink) {

    if sink.contains("background") == false {
        sink.properties.insert(0,Property::new_color("background".to_string(), "#000000".to_string()));
    }

    if sink.contains("lighting") == false {
        sink.push(Property::new_string("lighting".to_string(), "off".to_string()));
    }
}

pub fn generate_region_sink_descriptions() -> HashMap<String, Vec<String>> {
    let mut map : HashMap<String, Vec<String>> = HashMap::new();

    map.insert("background".to_string(), vec!["The background color of the region".to_string()]);
    map.insert("lighting".to_string(), vec!["The lighting mode. Use \"off\" for no lighting.".to_string()]);

    map
}
