use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

use GeoFXNodeRole::*;

#[derive(PartialEq, Clone, Debug)]
pub struct FTBuilderContext {
    pub id_counter: i32,
    pub out: String,
    pub geometry: Vec<String>,
    pub material_id: Option<String>,
    pub cut_out: Option<String>,
}

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXObject {
    pub id: Uuid,

    #[serde(default)]
    pub name: String,

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
            name: String::new(),

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

    /// Generates the source code of the face.
    pub fn build(&self, palette: &ThePalette, textures: &FxHashMap<Uuid, TheRGBATile>) -> String {
        let mut ctx = FTBuilderContext {
            out: String::new(),
            id_counter: 0,
            geometry: vec![],
            material_id: None,
            cut_out: None,
        };

        for (index, node) in self.nodes.iter().enumerate() {
            if node.role == MetaMaterial || node.role == MetaDelete {
                self.build_trail(index, palette, textures, &mut ctx);
            }
        }

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
        //println!("build_trail: {:?}", self.nodes[node].role);

        // Check for the material of a shape
        if self.nodes[node].is_shape() {
            if let Some((n, _)) = self.find_connected_input_node(node, 1) {
                self.build_trail(n as usize, palette, textures, ctx);
            }
        }

        // Build pattern content
        match &self.nodes[node].role {
            Repeat => {
                for terminal in 0..4 {
                    if let Some((n, _)) = self.find_connected_input_node(node, terminal) {
                        self.build_trail(n as usize, palette, textures, ctx);
                    }
                }
            }
            Stack => {
                let mut geometry = vec![];
                for terminal in 0..4 {
                    if let Some((n, _)) = self.find_connected_input_node(node, terminal) {
                        self.build_trail(n as usize, palette, textures, ctx);
                        geometry.append(&mut ctx.geometry);
                    }
                }
                ctx.geometry = geometry;
            }
            Group => {
                if let Some((n, _)) = self.find_connected_input_node(node, 0) {
                    self.build_trail(n as usize, palette, textures, ctx);
                    if !ctx.geometry.is_empty() {
                        ctx.cut_out = Some(ctx.geometry[0].clone());
                    }
                }
                ctx.geometry = vec![];
                let mut geometry = vec![];
                for terminal in 1..4 {
                    if let Some((n, _)) = self.find_connected_input_node(node, terminal) {
                        self.build_trail(n as usize, palette, textures, ctx);
                        geometry.append(&mut ctx.geometry);
                    }
                }
                ctx.geometry = geometry;
            }
            LeftWall | MiddleWallV | RightWall | BackWall | MiddleWallH | FrontWall => {
                let mut geometry = vec![];
                for terminal in 0..6 {
                    if let Some((n, _)) = self.find_connected_input_node(node, terminal) {
                        self.build_trail(n as usize, palette, textures, ctx);
                        geometry.append(&mut ctx.geometry);
                    }
                }
                ctx.geometry = geometry;
            }
            _ => {
                // Follow the trail at output terminal 0
                if let Some((n, _)) = self.find_connected_input_node(node, 0) {
                    self.build_trail(n as usize, palette, textures, ctx);
                }
            }
        }

        self.nodes[node].build(palette, textures, ctx);
    }

    pub fn update_area(&mut self) {
        self.area.clear();

        for geo in &self.nodes {
            let area = geo.area(false);
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
            LeftWall | BackWall | RightWall | FrontWall | MiddleWallH | MiddleWallV => {
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

    /// Returns true if this object is vertical.
    pub fn is_vertical(&self) -> bool {
        if let Some(geo) = self.nodes.first() {
            geo.is_vertical()
        } else {
            false
        }
    }

    /// Returns the area of the object without any 2D transforms, so that the 3D rendere can mask against it.
    pub fn area_without_2d_transforms(&self) -> Vec<Vec2i> {
        if let Some(geo) = self.nodes.first() {
            geo.area(true)
        } else {
            vec![]
        }
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

    /// Get the tile position of the node.
    pub fn get_position_2d(&self) -> Vec2f {
        if let Some(geo) = self.nodes.first() {
            let collection = geo.collection();
            let mut pos = geo.position(&collection);

            if let Some(value) = collection.get("2D Mode") {
                if let Some(mode) = value.to_i32() {
                    if mode == 1 {
                        if geo.is_vertical() {
                            pos.y -= 1.0;
                        } else {
                            pos.x -= 1.0;
                        }
                    }
                }
            }

            pos
        } else {
            Vec2f::zero()
        }
    }

    /// Returns the 2D mode of the object.
    pub fn get_2d_mode(&self) -> i32 {
        let mut mode = 0;

        if let Some(geo) = self.nodes.first() {
            let collection = geo.collection();

            if let Some(value) = collection.get("2D Mode") {
                if let Some(value) = value.to_i32() {
                    mode = value;
                }
            }
        }
        mode
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

        for (index, node) in self.nodes.iter_mut().enumerate() {
            let n = TheNode {
                name: node.name(),
                position: node.position,
                inputs: node.inputs(),
                outputs: node.outputs(&index, &self.connections),
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

    pub fn preview_2d(
        &self,
        buffer: &mut TheRGBABuffer,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
    ) {
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height;

        let ft = ForgedTiles::default();

        let code = self.build(palette, textures);

        if let Ok(ctx) = ft.compile_code(code) {
            buffer
                .pixels_mut()
                .par_rchunks_exact_mut(width * 4)
                .enumerate()
                .for_each(|(j, line)| {
                    for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                        let i = j * width + i;

                        let xx = (i % width) as f32 / width as f32;
                        let yy = (i / width) as f32 / height as f32;

                        let p = vec2f(xx, yy);

                        if let Some(col) = ctx.face_pixel_at(p) {
                            pixel.copy_from_slice(&col);
                        }
                    }
                });
        }
    }
}
