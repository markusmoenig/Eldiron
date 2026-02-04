use crate::vm::node::nodeop::NodeOp;
use rustc_hash::FxHashMap;

/// Simple registry of built-in functions (name -> (arity, op)).
#[derive(Clone)]
pub struct Builtins {
    map: FxHashMap<String, (u8, NodeOp)>,
}

impl Builtins {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
        }
    }

    pub fn insert<S: Into<String>>(&mut self, name: S, arity: u8, op: NodeOp) {
        self.map.insert(name.into(), (arity, op));
    }

    pub fn get(&self, name: &str) -> Option<(u8, NodeOp)> {
        self.map.get(name).cloned()
    }

    pub fn entries(&self) -> impl Iterator<Item = (&String, &(u8, NodeOp))> {
        self.map.iter()
    }
}

impl Default for Builtins {
    fn default() -> Self {
        let mut b = Builtins::new();
        b.insert("length", 1, NodeOp::Length);
        b.insert("length2", 1, NodeOp::Length2);
        b.insert("length3", 1, NodeOp::Length3);
        b.insert("abs", 1, NodeOp::Abs);
        b.insert("sin", 1, NodeOp::Sin);
        b.insert("sin1", 1, NodeOp::Sin1);
        b.insert("sin2", 1, NodeOp::Sin2);
        b.insert("cos", 1, NodeOp::Cos);
        b.insert("cos1", 1, NodeOp::Cos1);
        b.insert("cos2", 1, NodeOp::Cos2);
        b.insert("normalize", 1, NodeOp::Normalize);
        b.insert("tan", 1, NodeOp::Tan);
        b.insert("atan", 1, NodeOp::Atan);
        b.insert("atan2", 2, NodeOp::Atan2);
        b.insert("rotate2d", 2, NodeOp::Rotate2D);
        b.insert("dot", 2, NodeOp::Dot);
        b.insert("dot2", 2, NodeOp::Dot2);
        b.insert("dot3", 2, NodeOp::Dot3);
        b.insert("cross", 2, NodeOp::Cross);
        b.insert("floor", 1, NodeOp::Floor);
        b.insert("ceil", 1, NodeOp::Ceil);
        b.insert("round", 1, NodeOp::Round);
        b.insert("fract", 1, NodeOp::Fract);
        b.insert("mod", 2, NodeOp::Mod);
        b.insert("degrees", 1, NodeOp::Degrees);
        b.insert("radians", 1, NodeOp::Radians);
        b.insert("min", 2, NodeOp::Min);
        b.insert("max", 2, NodeOp::Max);
        b.insert("mix", 3, NodeOp::Mix);
        b.insert("smoothstep", 3, NodeOp::Smoothstep);
        b.insert("step", 2, NodeOp::Step);
        b.insert("clamp", 3, NodeOp::Clamp);
        b.insert("sqrt", 1, NodeOp::Sqrt);
        b.insert("log", 1, NodeOp::Log);
        b.insert("pow", 2, NodeOp::Pow);
        // print is variadic; arity handled in compiler
        b.insert("print", 0, NodeOp::Print(0));
        b.insert(
            "set_debug_loc",
            3,
            NodeOp::HostCall {
                name: "set_debug_loc".into(),
                argc: 3,
            },
        );
        b.insert(
            "set_player_camera",
            1,
            NodeOp::HostCall {
                name: "set_player_camera".into(),
                argc: 1,
            },
        );
        b.insert(
            "action",
            1,
            NodeOp::HostCall {
                name: "action".into(),
                argc: 1,
            },
        );
        b.insert(
            "intent",
            1,
            NodeOp::HostCall {
                name: "intent".into(),
                argc: 1,
            },
        );
        b.insert(
            "message",
            3,
            NodeOp::HostCall {
                name: "message".into(),
                argc: 3,
            },
        );
        // Host-only ops
        b.insert(
            "set_tile",
            1,
            NodeOp::HostCall {
                name: "set_tile".into(),
                argc: 1,
            },
        );
        b.insert(
            "set_emit_light",
            1,
            NodeOp::HostCall {
                name: "set_emit_light".into(),
                argc: 1,
            },
        );
        b.insert(
            "set_rig_sequence",
            0,
            NodeOp::HostCall {
                name: "set_rig_sequence".into(),
                argc: 0,
            },
        );
        b.insert(
            "take",
            1,
            NodeOp::HostCall {
                name: "take".into(),
                argc: 1,
            },
        );
        b.insert(
            "equip",
            1,
            NodeOp::HostCall {
                name: "equip".into(),
                argc: 1,
            },
        );
        b.insert(
            "get_attr_of",
            2,
            NodeOp::HostCall {
                name: "get_attr_of".into(),
                argc: 2,
            },
        );
        b.insert(
            "get_attr",
            1,
            NodeOp::HostCall {
                name: "get_attr".into(),
                argc: 1,
            },
        );
        b.insert(
            "set_attr",
            2,
            NodeOp::HostCall {
                name: "set_attr".into(),
                argc: 2,
            },
        );
        b.insert(
            "toggle_attr",
            1,
            NodeOp::HostCall {
                name: "toggle_attr".into(),
                argc: 1,
            },
        );
        b.insert(
            "random",
            2,
            NodeOp::HostCall {
                name: "random".into(),
                argc: 2,
            },
        );
        b.insert(
            "notify_in",
            2,
            NodeOp::HostCall {
                name: "notify_in".into(),
                argc: 2,
            },
        );
        b.insert(
            "random_walk",
            3,
            NodeOp::HostCall {
                name: "random_walk".into(),
                argc: 3,
            },
        );
        b.insert(
            "random_walk_in_sector",
            3,
            NodeOp::HostCall {
                name: "random_walk_in_sector".into(),
                argc: 3,
            },
        );
        b.insert(
            "debug",
            1,
            NodeOp::HostCall {
                name: "debug".into(),
                argc: 1,
            },
        );
        b.insert(
            "inventory_items",
            1,
            NodeOp::HostCall {
                name: "inventory_items".into(),
                argc: 1,
            },
        );
        b.insert(
            "inventory_items_of",
            2,
            NodeOp::HostCall {
                name: "inventory_items_of".into(),
                argc: 2,
            },
        );
        b.insert(
            "entities_in_radius",
            0,
            NodeOp::HostCall {
                name: "entities_in_radius".into(),
                argc: 0,
            },
        );
        b.insert(
            "list_get",
            2,
            NodeOp::HostCall {
                name: "list_get".into(),
                argc: 2,
            },
        );
        b.insert(
            "is_item",
            1,
            NodeOp::HostCall {
                name: "is_item".into(),
                argc: 1,
            },
        );
        b.insert(
            "is_entity",
            1,
            NodeOp::HostCall {
                name: "is_entity".into(),
                argc: 1,
            },
        );
        b.insert(
            "distance_to",
            1,
            NodeOp::HostCall {
                name: "distance_to".into(),
                argc: 1,
            },
        );
        b.insert(
            "set_proximity_tracking",
            2,
            NodeOp::HostCall {
                name: "set_proximity_tracking".into(),
                argc: 2,
            },
        );
        b.insert(
            "deal_damage",
            2,
            NodeOp::HostCall {
                name: "deal_damage".into(),
                argc: 2,
            },
        );
        b.insert(
            "took_damage",
            2,
            NodeOp::HostCall {
                name: "took_damage".into(),
                argc: 2,
            },
        );
        b.insert(
            "block_events",
            2,
            NodeOp::HostCall {
                name: "block_events".into(),
                argc: 2,
            },
        );
        b.insert(
            "add_item",
            1,
            NodeOp::HostCall {
                name: "add_item".into(),
                argc: 1,
            },
        );
        b.insert(
            "drop_items",
            1,
            NodeOp::HostCall {
                name: "drop_items".into(),
                argc: 1,
            },
        );
        b.insert(
            "offer_inventory",
            2,
            NodeOp::HostCall {
                name: "offer_inventory".into(),
                argc: 2,
            },
        );
        b.insert(
            "drop",
            1,
            NodeOp::HostCall {
                name: "drop".into(),
                argc: 1,
            },
        );
        b.insert(
            "teleport",
            2,
            NodeOp::HostCall {
                name: "teleport".into(),
                argc: 2,
            },
        );
        b.insert(
            "goto",
            2,
            NodeOp::HostCall {
                name: "goto".into(),
                argc: 2,
            },
        );
        b.insert(
            "close_in",
            3,
            NodeOp::HostCall {
                name: "close_in".into(),
                argc: 3,
            },
        );
        b.insert(
            "id",
            0,
            NodeOp::HostCall {
                name: "id".into(),
                argc: 0,
            },
        );
        // format is variadic; arity handled specially in compiler.
        b.insert("format", 0, NodeOp::Format(0));
        b
    }
}
