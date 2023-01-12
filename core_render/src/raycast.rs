
use crate::prelude::*;
use raycaster::prelude::*;

/// Handles the 2.5D raycaster support.
pub struct Raycast {

    pub raycaster               : Raycaster,

    /// Tilemaps map
    world_maps                  : FxHashMap<Uuid, WorldMap>,
    world_tilemaps              : FxHashMap<Uuid, FxHashMap<Uuid, (usize, usize, usize, usize)>>,
}

impl Raycast {

    pub fn new() -> Self {
        Self {
            raycaster           : Raycaster::new(),

            world_maps          : FxHashMap::default(),
            world_tilemaps      : FxHashMap::default(),
        }
    }

    /// Creates a WorldMap for the given region and passed the required tilemaps.
    pub fn load_region(&mut self, asset: &Asset, region: &GameRegionData) {

        let mut world = WorldMap::new();
        let mut tilemaps : FxHashMap<Uuid, (usize, usize, usize, usize)> = FxHashMap::default();

        // Add the tilemaps needed to draw the region
        // TODO: Only pass in the tilemaps that are actually used by the region
        for (tilemap_id, tilemap) in &asset.tileset.maps {
            let id = world.add_image(tilemap.pixels.clone(), tilemap.width as u32, tilemap.height as u32);
            tilemaps.insert(*tilemap_id, (id, tilemap.settings.grid_size, tilemap.width, tilemap.height));
        }

        let blue = raycaster::Tile::colored([0, 0, 255, 255]);
        world.set_default_ceiling(blue);
        //world.set_fog([100, 100, 100, 255], 10.0);
        self.world_maps.insert(region.id, world);

        // Pass the tiles and add them to worldmap depending on their properties (floor, wall, ceiling?)
        for (pos, tile) in &region.layer1 {
            if let Some(world) = self.world_maps.get_mut(&region.id) {
                if let Some((t_id, size, width, _height)) = tilemaps.get(&tile.tilemap) {
                    let rect = (tile.x_off as usize * size * 4, tile.y_off as usize * width * size * 4, *size, *size);
                    let t = raycaster::Tile::textured(*t_id, rect);
                    world.set_floor(pos.0 as i32, -pos.1 as i32, t);
                    // world.set_floor_tile(t);
                }
            }
        }

        for (pos, tile) in &region.layer2 {
            if let Some(world) = self.world_maps.get_mut(&region.id) {
                if let Some((t_id, size, width, _height)) = tilemaps.get(&tile.tilemap) {
                    let rect =  (tile.x_off as usize * size * 4, tile.y_off as usize * width * size * 4, *size, *size);
                    let t = raycaster::Tile::textured(*t_id, rect);
                    //world.set_wall(pos.0 as i32, -pos.1 as i32, t);
                    world.add_sprite(Sprite::new(pos.0 as f32 + 0.5, -pos.1 as f32, t));
                }
            }
        }

        self.raycaster.face_north();

        self.world_tilemaps.insert(region.id, tilemaps);
    }

    /// Sets the position of the raycaster
    pub fn render(&mut self, frame: &mut [u8], pos: (i32, i32), region: &Uuid, rect: (usize, usize, usize, usize), stride: usize) {
        self.raycaster.set_pos(pos.0 as f32 + 0.5, -pos.1 as f32);

        //println!("pos: {:?}, {}", pos, stride);

        if let Some(world) = self.world_maps.get_mut(region) {
            self.raycaster.render(frame, rect, stride, world);
        }
    }
}