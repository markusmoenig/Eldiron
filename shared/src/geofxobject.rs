use crate::prelude::*;
use theframework::prelude::*;

use GeoFXNodeRole::*;

#[derive(PartialEq, Clone, Debug)]
pub struct FTBuilderContext {
    pub id_counter: i32,
    pub out: String,
    pub geometry: Vec<String>,
    pub material_id: Option<String>,
}

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXObject {
    pub id: Uuid,
    pub material_id: Uuid,

    pub nodes: Vec<GeoFXNode>,

    /// The node connections: Source node index, source terminal, dest node index, dest terminal
    pub connections: Vec<(u16, u8, u16, u8)>,
    pub zoom: f32,
    pub selected_node: Option<usize>,

    pub scroll_offset: Vec2i,

    pub area: Vec<Vec2i>,

    #[serde(default)]
    pub height: i32,
    pub level: i32,
}

impl Default for GeoFXObject {
    fn default() -> Self {
        Self::new()
    }
}

impl GeoFXObject {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            material_id: Uuid::nil(),

            nodes: Vec::new(),
            connections: Vec::new(),
            zoom: 1.0,
            selected_node: None,

            scroll_offset: Vec2i::zero(),

            area: Vec::new(),

            height: 0,
            level: 0,
        }
    }

    /// Gives a chance to each node to update its parameters in case things changed.
    pub fn update_parameters(&mut self) {
        for n in &mut self.nodes {
            n.update_parameters();
        }
    }

    /// Loads the parameters of the nodes into memory for faster access.
    pub fn load_parameters(&self, time: &TheTime) -> Vec<Vec<f32>> {
        let mut data = vec![];

        for n in &self.nodes {
            data.push(n.load_parameters(time));
        }
        data
    }

    /// Generates the source code of the face.
    pub fn build(&self, palette: &ThePalette, textures: &FxHashMap<Uuid, TheRGBATile>) -> String {
        let mut ctx = FTBuilderContext {
            out: String::new(),
            id_counter: 0,
            geometry: vec![],
            material_id: None,
        };

        self.build_trail(0, palette, textures, &mut ctx);
        ctx.out
    }

    /// After exiting a geometry node follow the trail of material nodes to compute the material.
    pub fn build_trail(
        &self,
        node: usize,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        ctx: &mut FTBuilderContext,
    ) {
        println!("build_trail: {:?}", self.nodes[node].role);

        // Check for the material of a shape
        if self.nodes[node].is_shape() {
            if let Some((n, _)) = self.find_connected_input_node(node, 1) {
                self.build_trail(n as usize, palette, textures, ctx);
            }
        }

        // Build pattern content
        match &self.nodes[node].role {
            Repeat => {
                for terminal in 1..7 {
                    if let Some((n, _)) = self.find_connected_input_node(node, terminal) {
                        self.build_trail(n as usize, palette, textures, ctx);
                    }
                }
            }
            Stack => {
                let mut geometry = vec![];
                for terminal in 1..7 {
                    if let Some((n, _)) = self.find_connected_input_node(node, terminal) {
                        self.build_trail(n as usize, palette, textures, ctx);
                        geometry.append(&mut ctx.geometry);
                    }
                }
                ctx.geometry = geometry;
            }
            _ => {}
        }

        // Follow the trail at output terminal 0
        if let Some((n, _)) = self.find_connected_input_node(node, 0) {
            self.build_trail(n as usize, palette, textures, ctx);
        }

        self.nodes[node].build(palette, textures, ctx);
    }

    /// Returns the distance to the object nodes, the distance and the index of the closes node is returned.
    pub fn distance(
        &self,
        time: &TheTime,
        p: Vec2f,
        scale: f32,
        hit: &mut Option<&mut Hit>,
    ) -> (f32, usize) {
        let mut min_distance = f32::INFINITY;
        let mut index = 10000;

        for (i, geo) in self.nodes.iter().enumerate() {
            let distance = geo.distance(time, p, scale, hit);
            if distance < min_distance {
                min_distance = distance;
                index = i;
            }
        }

        (min_distance, index)
    }

    pub fn distance_3d(
        &self,
        time: &TheTime,
        p: Vec3f,
        hit: &mut Option<&mut Hit>,
        params: &[Vec<f32>],
    ) -> (f32, usize) {
        let mut min_distance = f32::INFINITY;
        let mut index = 10000;

        for (i, geo) in self.nodes.iter().enumerate() {
            let distance = geo.distance_3d(time, p, hit, &params[i]);
            if distance < min_distance {
                min_distance = distance;
                index = i;
            }
        }

        (min_distance, index)
    }

    pub fn normal(&self, time: &TheTime, p: Vec3f, params: &[Vec<f32>]) -> Vec3f {
        let scale = 0.5773 * 0.0005;
        let e = vec2f(1.0 * scale, -1.0 * scale);

        // IQs normal function

        let e1 = vec3f(e.x, e.y, e.y);
        let e2 = vec3f(e.y, e.y, e.x);
        let e3 = vec3f(e.y, e.x, e.y);
        let e4 = vec3f(e.x, e.x, e.x);

        let n = e1 * self.distance_3d(time, p + e1, &mut None, params).0
            + e2 * self.distance_3d(time, p + e2, &mut None, params).0
            + e3 * self.distance_3d(time, p + e3, &mut None, params).0
            + e4 * self.distance_3d(time, p + e4, &mut None, params).0;
        normalize(n)
    }

    pub fn update_area(&mut self) {
        self.area.clear();
        // let mut area = AABB2D::zero();
        // for geo in &self.nodes {
        //     if let Some(aabb) = geo.aabb(&TheTime::default()) {
        //         area.grow(aabb);
        //     }
        // }
        // self.area = area.to_tiles();

        // for geo in &self.nodes {
        //     let p = geo.position();
        //     let pp = vec2i(p.x as i32, p.y as i32);
        //     if !self.area.contains(&pp) {
        //         self.area.push(pp);
        //     }
        // }

        for geo in &self.nodes {
            let area = geo.area();
            self.height = geo.height().ceil() as i32;
            for p in area {
                if !self.area.contains(&p) {
                    self.area.push(p);
                }
            }
        }
    }

    /// Checks if this tile is blocking
    pub fn is_blocking(&self) -> bool {
        for node in &self.nodes {
            if node.is_blocking() {
                return true;
            }
        }
        false
    }

    /// Update the nodes with defaults.
    pub fn init(&mut self) {
        let mut role = GeoFXNodeRole::MiddleWallH;
        if let Some(geo) = self.nodes.first() {
            role = geo.role.clone();
        }
        match &role {
            LeftWall | TopWall | RightWall | BottomWall | MiddleWallH | MiddleWallV => {
                let mut bricks = GeoFXNode::new(Box);
                bricks.position = vec2i(200, 40);
                self.nodes.push(bricks);
                self.connections.push((0, 0, 1, 0));
            }
            _ => {}
        }
    }

    /// Returns the layer role (Ground, Wall etc) for this object.
    pub fn get_layer_role(&self) -> Option<Layer2DRole> {
        if let Some(geo) = self.nodes.first() {
            return Some(geo.get_layer_role());
        }

        None
    }

    /// Get the length of the node.
    pub fn get_length(&self) -> f32 {
        if let Some(geo) = self.nodes.first() {
            geo.length()
        } else {
            1.0
        }
    }

    /// Get the height of the node.
    pub fn get_height(&self) -> f32 {
        if let Some(geo) = self.nodes.first() {
            geo.height()
        } else {
            1.0
        }
    }

    /// Get the thickness of the node.
    pub fn get_thickness(&self) -> f32 {
        if let Some(geo) = self.nodes.first() {
            geo.thickness()
        } else {
            0.2
        }
    }

    /// Get the tile position of the node.
    pub fn get_position(&self) -> Vec2f {
        if let Some(geo) = self.nodes.first() {
            let collection = geo.collection();
            geo.position(&collection)
        } else {
            Vec2f::zero()
        }
    }

    /// Set the tile position of the node.
    pub fn set_position(&mut self, pos: Vec2f) {
        if let Some(geo) = self.nodes.first_mut() {
            geo.set_position(pos);
        }
    }

    /// Convert the model to a node canvas.
    pub fn to_canvas(&mut self) -> TheNodeCanvas {
        let mut canvas = TheNodeCanvas {
            node_width: 136,
            selected_node: self.selected_node,
            offset: self.scroll_offset,
            ..Default::default()
        };

        //let preview_size = 100;

        for (index, node) in self.nodes.iter().enumerate() {
            // if i >= self.node_previews.len() {
            //     self.node_previews.resize(i + 1, None);
            // }

            // Remove preview buffer if size has changed
            // if let Some(preview_buffer) = &self.node_previews[i] {
            //     if preview_buffer.dim().width != preview_size
            //         && preview_buffer.dim().height != preview_size
            //     {
            //         self.node_previews[i] = None;
            //     }
            // }

            // Create preview if it doesn't exist
            // if self.node_previews[i].is_none() {
            //     let preview_buffer = TheRGBABuffer::new(TheDim::sized(preview_size, preview_size));
            //     //self.render_node_preview(&mut preview_buffer, i, palette);
            //     self.node_previews[i] = Some(preview_buffer);
            // }

            let n = TheNode {
                name: node.name(),
                position: node.position,
                inputs: node.inputs(),
                outputs: node.outputs(),
                preview: node.preview.clone(),
                supports_preview: node.supports_preview,
                preview_is_open: node.preview_is_open,
                can_be_deleted: index != 0,
            };
            canvas.nodes.push(n);
        }
        canvas.connections.clone_from(&self.connections);
        canvas.zoom = self.zoom;
        canvas.selected_node = self.selected_node;

        canvas
    }

    /// Returns the connected input node and terminal for the given output node and terminal.
    pub fn find_connected_input_node(
        &self,
        node: usize,
        terminal_index: usize,
    ) -> Option<(u16, u8)> {
        for (o, ot, i, it) in &self.connections {
            if *o == node as u16 && *ot == terminal_index as u8 {
                return Some((*i, *it));
            }
        }
        None
    }

    /// Load a model from a JSON string.
    pub fn from_json(json: &str) -> Self {
        let object: GeoFXObject = serde_json::from_str(json).unwrap_or_default();
        object
    }

    /// Convert the model to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}
