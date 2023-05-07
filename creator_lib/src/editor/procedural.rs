use crate::prelude::*;
use rhai::{ Engine, Dynamic };

pub fn generate_region(region: &mut GameRegion, _asset: &Asset) {
    //if region.procedural.is_none() { return; }

    if let Some(procedural) = &mut region.procedural {

        let nodes = procedural.data.nodes.clone();

        for (id, node) in &nodes {
            if node.behavior_type == BehaviorNodeType::Cellular {
                create_cellular(region, (id, node));
                break;
            } else
            if node.behavior_type == BehaviorNodeType::DrunkardsWalk {
                drunkards_walk(region, (id, node));
                break;
            }
        }
    }
}

/// Random walk
fn drunkards_walk(region: &mut GameRegion, node: (&Uuid, &BehaviorNode)) {

    let mut engine = setup_engine();

    let mut size = 80;
    let mut distance = 50;

    if let Some(d) = get_node_script_int_value(&mut engine, node.1, "distance".into()) {
        distance = d;
    }

    if let Some(s) = get_node_script_int_value(&mut engine, node.1, "size".into()) {
        size = s;
    }

    let mut rng = thread_rng();

    let mut layer1 = FxHashMap::default();
    let mut layer2 = FxHashMap::default();
    let layer3 = FxHashMap::default();
    let layer4 = FxHashMap::default();

    let needs_to_cover = (size * size) / 3;

    if let Some(f) = get_node_value(node.1, "floor".into()) {
        if let Some(w) = get_node_value(node.1, "wall".into()) {

            if f.to_tile_data().is_none() || w.to_tile_data().is_none() {
                return;
            }

            let floor = f.to_tile_data().unwrap();
            let wall = w.to_tile_data().unwrap();

            let range_s = 0_isize;
            let range_e = size as isize;

            // Fill the area with walls

            for y in range_s..range_e {
                for x in range_s..range_e {
                    if wall.usage == TileUsage::EnvBlocking {
                        layer2.insert((x, y), wall.clone());
                    } else {
                        layer1.insert((x, y), wall.clone());
                    }
                }
            }

            fn is_valid(x: isize, y: isize, start: isize, end: isize) -> bool {
                if x >= start && x < end && y >= start && y < end {
                    true
                } else {
                    false
                }
            }

            let mut i = 0;

            loop {
                // Place a miner

                let mut d = 0;
                let mut x = range_e / 2;//rng.gen_range(range_s..range_e);
                let mut y = range_e - 1;//rng.gen_range(range_s..range_e);

                layer1.insert((x, y), floor.clone());
                layer2.remove(&(x,y));

                for _ in 0..distance {

                    match rng.gen_range(0..4) {
                        0 => { x -= 1; },
                        1 => { x += 1; },
                        2 => { y -= 1; },
                        _ => { y += 1; }
                    }

                    if is_valid(x, y, range_s,range_e) {
                        layer1.insert((x, y), floor.clone());
                        layer2.remove(&(x,y));
                        d += 1;
                    } else {
                        break;
                    }

                    if d >= distance {
                        break;
                    }
                }

                // Calc how much we cover already

                let mut covers = 0;
                for y in range_s..range_e {
                    for x in range_s..range_e {
                        if layer1.contains_key(&(x, y)) {
                            covers += 1;
                        }
                    }
                }
                if covers >= needs_to_cover {
                    break;
                }

                // Safeguard
                i += 1;
                if i >= 1000 {
                    break;
                }
            }

            // Close the edges
            for y in range_s..range_e {
                for x in range_s..range_e {
                    if x == range_s || y == range_s || x == range_e -1 || y == range_e - 1 {
                        if wall.usage == TileUsage::EnvBlocking {
                            layer2.insert((x, y), wall.clone());
                        } else {
                            layer1.insert((x, y), wall.clone());
                        }
                    }
                }
            }
        }
    }

    region.data.layer1 = layer1;
    region.data.layer2 = layer2;
    region.data.layer3 = layer3;
    region.data.layer4 = layer4;

    region.calc_dimensions();
}

/// Cellular creation
fn create_cellular(region: &mut GameRegion, node: (&Uuid, &BehaviorNode)) {

    let mut engine = setup_engine();

    let mut size = 80;
    let mut steps = 1;

    if let Some(s) = get_node_script_int_value(&mut engine, node.1, "steps".into()) {
        steps = s;
    }

    if let Some(s) = get_node_script_int_value(&mut engine, node.1, "size".into()) {
        size = s;
    }

    steps = steps.clamp(0, 100);

    let mut rng = thread_rng();

    //let half_size = (size / 2) as isize;
    let range_s = 0_isize;
    let range_e = size as isize;

    let mut random_layer : FxHashMap<(isize, isize), i32> = FxHashMap::default();

    for y in range_s..range_e {
        for x in range_s..range_e {
            let random = rng.gen_range(0..=100);
            random_layer.insert((x, y), if random > 55 { 0 } else { 1 });
        }
    }

    fn count_neighbours(map: &FxHashMap<(isize, isize), i32>, pos: (isize, isize)) -> i32 {
        let mut neighbours = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 { continue; }
                if let Some(v) = map.get(&(pos.0 + dx, pos.1 + dy)) {
                    if *v == 1 {
                        neighbours += 1;
                    }
                }
            }
        }
        neighbours
    }

    for _ in 0..steps {
        let mut new_layer = FxHashMap::default();
        for y in range_s..range_e {
            for x in range_s..range_e {
                let neighbours = count_neighbours(&random_layer, (x, y));
                if neighbours > 4 || neighbours == 0 {
                    new_layer.insert((x, y), 1);
                } else {
                    new_layer.insert((x, y), 0);
                }
            }
        }
        random_layer = new_layer;
    }

    if let Some(f) = get_node_value(node.1, "floor".into()) {
        if let Some(w) = get_node_value(node.1, "wall".into()) {

            if f.to_tile_data().is_none() || w.to_tile_data().is_none() {
                return;
            }

            let floor = f.to_tile_data().unwrap();
            let wall = w.to_tile_data().unwrap();

            let mut layer1 = FxHashMap::default();
            let mut layer2 = FxHashMap::default();
            let layer3 = FxHashMap::default();
            let layer4 = FxHashMap::default();

            for y in range_s..range_e {
                for x in range_s..range_e {
                    if let Some(r) = random_layer.get(&(x, y)) {
                        if *r == 1 {
                            if wall.usage == TileUsage::EnvBlocking {
                                layer2.insert((x, y), wall.clone());
                            } else {
                                layer1.insert((x, y), wall.clone());
                            }
                        } else {
                            layer1.insert((x, y), floor.clone());
                        }
                    }
                }
            }

            // Close the edges
            for y in range_s..range_e {
                for x in range_s..range_e {
                    if x == range_s || y == range_s || x == range_e -1 || y == range_e - 1 {
                        if wall.usage == TileUsage::EnvBlocking {
                            layer2.insert((x, y), wall.clone());
                        } else {
                            layer1.insert((x, y), wall.clone());
                        }
                    }
                }
            }

            region.data.layer1 = layer1;
            region.data.layer2 = layer2;
            region.data.layer3 = layer3;
            region.data.layer4 = layer4;

            region.calc_dimensions();
        }
    }
}

/// Get the script int value of the given node
fn get_node_script_int_value(engine: &Engine, node: &BehaviorNode, name: String) -> Option<i32> {
    if let Some(v) = get_node_value(node, name) {
        if let Some(s) = v.to_string() {
            if let Some(rc) = engine.eval::<Dynamic>(s.as_str()).ok() {
                if let Some(int) = rc.as_int().ok() {
                    return Some(int);
                }
            }
        }
    }
    None
}

/// Get a value of the given node
fn get_node_value(node: &BehaviorNode, name: String) -> Option<Value> {
    for (id, value) in &node.values {
        if *id == name {
            return Some(value.clone());
        }
    }
    None
}

//
fn setup_engine() -> Engine {
    let mut engine = Engine::new();

    #[allow(deprecated)]
    engine.on_var(|name, _index, _context| {

        if name.starts_with("d") {
            let mut s = name.to_string();
            s.remove(0);
            if let Some(n) = s.parse::<i32>().ok() {
                let mut rng = thread_rng();
                let random = rng.gen_range(1..=n);
                return Ok(Some(random.into()));
            }
        }
        Ok(None)
    });

    engine
}