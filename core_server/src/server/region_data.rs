use crate::prelude::*;

pub struct RegionData {
    pub sheets                      : Vec<Sheet>,
    pub nodes                       : FxHashMap<BehaviorNodeType, NodeDataCall>,

    pub curr_index                  : usize,
}

impl RegionData {
    pub fn new() -> Self {

        let mut nodes : FxHashMap<BehaviorNodeType, NodeDataCall> = FxHashMap::default();
        nodes.insert(BehaviorNodeType::Script, node_script);

        Self {
            sheets                  : vec![],
            nodes,

            curr_index              : 0,
        }
    }
}