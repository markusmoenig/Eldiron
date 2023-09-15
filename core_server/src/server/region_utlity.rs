extern crate ref_thread_local;
use crate::prelude::*;
use ref_thread_local::RefThreadLocal;

pub struct RegionUtility {
    pub rng: ThreadRng,

    pub roll_regex: regex::Regex,
}

impl RegionUtility {
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),

            roll_regex: regex::Regex::new(r"^(\d+)?d(\d+)([+-]\d+)?$").unwrap(),
        }
    }

    pub fn roll(&mut self, dice_expression: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let caps = self
            .roll_regex
            .captures(dice_expression)
            .ok_or("Invalid dice expression")?;

        let num_dice = caps
            .get(1)
            .map_or(1, |m| m.as_str().parse::<u32>().unwrap());
        let num_sides = caps.get(2).unwrap().as_str().parse::<u32>()?;
        let modifier = caps
            .get(3)
            .map_or(0, |m| m.as_str().parse::<i32>().unwrap());

        if num_sides <= 0 {
            return Err("Number of sides must be at least 1".into());
        }

        let total: u32 = (0..num_dice)
            .map(|_| self.rng.gen_range(1..=num_sides))
            .sum();
        Ok(total as i32 + modifier)
    }
}

/// Executes the given node and follows the connection chain
pub fn execute_area_node(
    region_id: Uuid,
    area_index: usize,
    node_id: Uuid,
) -> Option<BehaviorNodeConnector> {
    let mut connectors: Vec<BehaviorNodeConnector> = vec![];
    let mut connected_node_ids: Vec<Uuid> = vec![];

    let mut node_call: Option<NodeDataCall> = None;

    {
        let data = &mut REGION_DATA.borrow_mut()[*CURR_INST.borrow()];
        data.curr_area_index = area_index;
        if let Some(node) = data.region_area_behavior[area_index].nodes.get(&node_id) {
            if let Some(nc) = data.nodes.get(&node.behavior_type) {
                node_call = Some(nc.clone());
            }
        }
    }

    let mut rc: Option<BehaviorNodeConnector> = None;
    if let Some(node_call) = node_call {
        let connector: BehaviorNodeConnector =
            node_call((region_id, node_id), &mut FxHashMap::default());
        rc = Some(connector);
        connectors.push(connector);
    } else {
        connectors.push(BehaviorNodeConnector::Bottom);
    }

    // Search the connections to check if we can find an ongoing node connection
    for connector in connectors {
        let data = &REGION_DATA.borrow()[*CURR_INST.borrow()];
        for c in &data.region_area_behavior[area_index].connections {
            if c.0 == node_id && c.1 == connector {
                connected_node_ids.push(c.2);
                //self.executed_connections.push((BehaviorType::Regions, c.0, c.1));
            }
        }
    }

    // And if yes execute it
    for (_index, connected_node_id) in connected_node_ids.iter().enumerate() {
        execute_area_node(region_id, area_index, *connected_node_id);
    }

    rc
}
