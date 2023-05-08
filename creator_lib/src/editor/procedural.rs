use crate::prelude::*;
use rhai::{ Engine, Dynamic, Scope };

pub fn generate_region(region: &mut GameRegion, _asset: &Asset) {
    region.delete_areas();

    fn build_chain(data: &GameBehaviorData, uuid: Uuid) -> Vec<BehaviorNode> {
        let mut chain =  vec![];

        let mut c = uuid;

        loop {
            let mut d : Option<Uuid> = None;

            for (s1, s2, d1, d2) in &data.connections {
                if *s1 == c {
                    d = Some(*d1);
                }
            }

            if let Some(d) = d {
                for (id, node) in &data.nodes {
                    if *id == d {
                        chain.push(node.clone());
                        c = d;
                        break;
                    }
                }
            } else {
                break;
            }
        }

        chain
    }

    if let Some(procedural) = &mut region.procedural {

        let data = procedural.data.clone();

        for (id, node) in &data.nodes {
            if node.behavior_type == BehaviorNodeType::Cellular {
                let chain = build_chain(&data, node.id);
                create_cellular(region, (id, node), chain);
                break;
            } else
            if node.behavior_type == BehaviorNodeType::DrunkardsWalk {
                let chain = build_chain(&data, node.id);
                drunkards_walk(region, (id, node), chain);
                break;
            }
        }
    }
}

/// Random walk
fn drunkards_walk(region: &mut GameRegion, node: (&Uuid, &BehaviorNode), chain: Vec<BehaviorNode>) {

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
                if x > start && x < end - 1 && y > start && y < end - 1 {
                    true
                } else {
                    false
                }
            }

            let mut i = 0;

            let mut sx = range_e / 2;
            let mut sy = range_e / 2;

            if let Some(pos) = get_start_area(&engine, size, &chain) {
                sx = pos.0;
                sy = pos.1;
                region.create_area(pos.2);
                if let Some(area) = region.data.areas.last_mut() {
                    area.area.push((pos.0, pos.1));
                    region.save_data();
                }
            }

            loop {
                // Place a miner

                let mut d = 0;
                let mut x = sx;
                let mut y = sy;

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

            /*
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
            }*/
        }
    }

    region.data.layer1 = layer1;
    region.data.layer2 = layer2;
    region.data.layer3 = layer3;
    region.data.layer4 = layer4;

    region.calc_dimensions();
}

/// Cellular creation
fn create_cellular(region: &mut GameRegion, node: (&Uuid, &BehaviorNode), chain: Vec<BehaviorNode>) {

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
fn get_node_script_dynamic_value(engine: &Engine, scope: &mut Scope, node: &BehaviorNode, name: String) -> Option<Dynamic> {
    if let Some(v) = get_node_value(node, name) {
        if let Some(s) = v.to_string() {
            let rc = engine.eval_with_scope::<Dynamic>(scope, s.as_str());
            if let Some(rc) = rc.ok() {
                return Some(rc);
            }
        }
    }
    None
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

/// Setup the engine
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

    ScriptPosition::register(&mut engine);

    engine
}

/// Extract the
fn get_start_area(engine: &Engine, size: i32, chain: &Vec<BehaviorNode>) -> Option<(isize, isize, String)> {

    let mut scope = Scope::new();
    scope.set_value("size", size);

    for n in chain {
        if n.behavior_type == BehaviorNodeType::StartArea {
            if let Some(p) = get_node_script_dynamic_value(engine, &mut scope, &n, "start".to_string()) {
                if let Some(pos) = p.read_lock::<ScriptPosition>() {
                    if let Some(name) = get_node_value(&n, "name".to_string()) {
                        if let Some(name) = name.to_string() {
                            return Some((pos.pos_signed.0, pos.pos_signed.1, name));
                        }
                    }
                }
            }
        }
    }
    None
}

// --- ScriptPosition

#[derive(PartialEq, Debug, Clone)]
pub struct ScriptPosition {
    pub pos             : (usize, usize),
    pub pos_signed      : (isize, isize)
}

impl ScriptPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            pos         : (x as usize, y as usize),
            pos_signed  : (x as isize, y as isize),
        }
    }

    pub fn x(&mut self) -> i32 {
        self.pos.0 as i32
    }

    pub fn y(&mut self) -> i32 {
        self.pos.1 as i32
    }

    pub fn register(engine: &mut Engine) {
        engine.register_type_with_name::<ScriptPosition>("Position")
            .register_get("x", ScriptPosition::x)
            .register_get("y", ScriptPosition::y)
            .register_fn("pos", ScriptPosition::new);
    }
}