use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct MaterialFXObject {
    pub id: Uuid,

    pub name: String,

    /// The nodes which make up the material.
    pub nodes: Vec<MaterialFXNode>,

    /// The node connections: Source node index, source terminal, dest node index, dest terminal
    pub connections: Vec<(u16, u8, u16, u8)>,

    //#[serde(skip)]
    //pub node_previews: Vec<Option<TheRGBABuffer>>,
    pub zoom: f32,
    pub selected_node: Option<usize>,

    pub scroll_offset: Vec2<i32>,
}

impl Default for MaterialFXObject {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialFXObject {
    pub fn new() -> Self {
        let nodes = vec![MaterialFXNode::new(MaterialFXNodeRole::Geometry)];
        let selected_node = Some(0);

        Self {
            id: Uuid::new_v4(),

            name: "Unnamed".to_string(),

            nodes,
            connections: Vec::new(),

            // node_previews: Vec::new(),
            zoom: 1.0,
            selected_node,

            scroll_offset: Vec2::zero(),
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

    /// Computes the material
    pub fn compute(
        &self,
        hit: &mut Hit,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        mat_obj_params: &[Vec<f32>],
    ) {
        hit.mode = HitMode::Albedo;
        self.follow_trail(0, 0, hit, palette, textures, mat_obj_params);
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

    /// Returns the connected output node for the given input node and terminal.
    pub fn find_connected_output_node(&self, node: usize, terminal_index: usize) -> Option<usize> {
        for (o, _, i, it) in &self.connections {
            if *i == node as u16 && *it == terminal_index as u8 {
                return Some(*o as usize);
            }
        }
        None
    }

    /// Checks if we have a bump node.
    pub fn has_bump(&self) -> bool {
        for n in &self.nodes {
            if n.role == MaterialFXNodeRole::Bump {
                return true;
            }
        }
        false
    }

    /// Get the distance to the material.
    pub fn get_material_distance(
        &self,
        material_index: usize,
        hit: &mut Hit,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        mat_obj_params: &[Vec<f32>],
    ) -> f32 {
        hit.mode = HitMode::Bump;
        // hit.extrusion = GeoFXNodeExtrusion::Y;
        // hit.extrusion_length = 0.0;
        // hit.interior_distance = -0.1;

        self.follow_trail(0, material_index, hit, palette, textures, mat_obj_params);
        hit.bump
    }

    /// Get the distance to the material.
    pub fn get_material_normal(
        &self,
        _material_index: usize,
        _p: Vec3<f32>,
        _hit: &mut Hit,
        _palette: &ThePalette,
        _textures: &FxHashMap<Uuid, TheRGBATile>,
        _mat_obj_params: &[Vec<f32>],
    ) -> Vec3<f32> {
        /*
        let scale = 0.5773 * 0.0005;
        let e = vec2f(1.0 * scale, -1.0 * scale);

        let mut hit = hit.clone();
        // IQs normal function

        let e1 = vec3f(e.x, e.y, e.y);
        let e2 = vec3f(e.y, e.y, e.x);
        let e3 = vec3f(e.y, e.x, e.y);
        let e4 = vec3f(e.x, e.x, e.x);

        let pattern_pos = vec2f(p.x, p.z);

        hit.pattern_pos = pattern_pos + vec2f(e1.x, e1.z);
        let re1 = e1
            * (p.y
                + self.get_material_distance(
                    material_index,
                    &mut hit,
                    palette,
                    textures,
                    mat_obj_params,
                ));

        hit.pattern_pos = pattern_pos + vec2f(e2.x, e2.z);
        let re2 = e2
            * (p.y
                + self.get_material_distance(
                    material_index,
                    &mut hit,
                    palette,
                    textures,
                    mat_obj_params,
                ));

        hit.pattern_pos = pattern_pos + vec2f(e3.x, e3.z);
        let re3 = e3
            * (p.y
                + self.get_material_distance(
                    material_index,
                    &mut hit,
                    palette,
                    textures,
                    mat_obj_params,
                ));

        hit.pattern_pos = pattern_pos + vec2f(e4.x, e4.z);
        let re4 = e4
            * (p.y
                + self.get_material_distance(
                    material_index,
                    &mut hit,
                    palette,
                    textures,
                    mat_obj_params,
                ));

        // let n = e1 * self.get_material_distance(0, e1, hit, palette, textures, mat_obj_params)
        //     + e2 * self.get_heightmap_distance_3d(time, p + e2, hit, mat_obj_params)
        //     + e3 * self.get_heightmap_distance_3d(time, p + e3, hit, mat_obj_params)
        //     + e4 * self.get_heightmap_distance_3d(time, p + e4, hit, mat_obj_params);
        normalize(re1 + re2 + re3 + re4)
        */
        Vec3::zero()
    }

    /// After exiting a geometry node follow the trail of material nodes to compute the material.
    pub fn follow_trail(
        &self,
        node: usize,
        terminal_index: usize,
        hit: &mut Hit,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        mat_obj_params: &[Vec<f32>],
    ) {
        let mut connections = vec![];
        for (o, ot, i, it) in &self.connections {
            if *o == node as u16 && *ot == terminal_index as u8 {
                connections.push((*i, *it));
            }
        }

        if !connections.is_empty() {
            // Resolve material outputs

            let mut resolved: Vec<Hit> = vec![];
            let resolver = connections[0].0;

            // We only need to resolve materials when in Albedo mode.
            if hit.mode == HitMode::Albedo {
                let mut to_resolve = vec![];

                // We collected the trails which need to be resolved
                let need_to_be_resolved = self.nodes[resolver as usize].trails_to_resolve();
                for trail in need_to_be_resolved {
                    for (o, ot, i, it) in &self.connections {
                        if *o == resolver && *ot == trail {
                            to_resolve.push((*i, *it));
                        }
                    }
                }

                //println!("to resolve #{}", to_resolve.len());

                let mut follow_ups = vec![];

                for (o, _) in &to_resolve {
                    let mut h = hit.clone();

                    if let Some(noise_index) = self.find_connected_output_node(*o as usize, 1) {
                        if self.nodes[noise_index].role == MaterialFXNodeRole::Noise2D
                            || self.nodes[noise_index].role == MaterialFXNodeRole::Noise3D
                        {
                            _ = self.nodes[noise_index].compute(
                                &mut h,
                                palette,
                                textures,
                                vec![],
                                &mat_obj_params[noise_index],
                            );
                        }
                    }

                    if let Some(ot) = self.nodes[*o as usize].compute(
                        &mut h,
                        palette,
                        textures,
                        vec![],
                        &mat_obj_params[*o as usize],
                    ) {
                        follow_ups.push((*o, ot));
                    }

                    resolved.push(h);
                }

                // Noise in for the resolver,
                if let Some(noise_index) = self.find_connected_output_node(resolver as usize, 1) {
                    if self.nodes[noise_index].role == MaterialFXNodeRole::Noise2D
                        || self.nodes[noise_index].role == MaterialFXNodeRole::Noise3D
                    {
                        _ = self.nodes[noise_index].compute(
                            hit,
                            palette,
                            textures,
                            vec![],
                            &mat_obj_params[noise_index],
                        );
                    }
                }

                //println!("resolved #{}", resolved.len());
            }

            // Execute the resolver
            if let Some(ot) = self.nodes[resolver as usize].compute(
                hit,
                palette,
                textures,
                resolved,
                &mat_obj_params[resolver as usize],
            ) {
                // And follow the trail
                hit.noise = None;
                hit.noise_scale = 1.0;
                self.follow_trail(
                    resolver as usize,
                    ot as usize,
                    hit,
                    palette,
                    textures,
                    mat_obj_params,
                );
            }
        }

        /*
        for (node, terminal) in follow_ups {
            hit.noise = None;
            hit.noise_scale = 1.0;
            self.follow_trail(
                node as usize,
                terminal as usize,
                hit,
                palette,
                textures,
                mat_obj_params,
            );
        }*/

        /*

        if connections.len() == 1 && self.nodes[connections[0].0 as usize].resolve_branches {
            // Resolve branches of the node and feed them into the resolver

            let mut resolved: Vec<Hit> = vec![];

            let resolver = connections[0].0;

            let mut to_resolve = vec![];
            for (o, _, i, it) in &self.connections {
                if *o == resolver {
                    to_resolve.push((*i, *it));
                }
            }

            let mut follow_ups = vec![];

            for (o, _) in &to_resolve {
                let mut h = hit.clone();

                if let Some(noise_index) = self.find_connected_output_node(*o as usize, 1) {
                    if self.nodes[noise_index].role == MaterialFXNodeRole::Noise2D
                        || self.nodes[noise_index].role == MaterialFXNodeRole::Noise3D
                    {
                        _ = self.nodes[noise_index].compute(
                            &mut h,
                            palette,
                            textures,
                            vec![],
                            &mat_obj_params[noise_index],
                        );
                    }
                }

                if let Some(ot) = self.nodes[*o as usize].compute(
                    &mut h,
                    palette,
                    textures,
                    vec![],
                    &mat_obj_params[*o as usize],
                ) {
                    follow_ups.push((*o, ot));
                }

                resolved.push(h);
            }

            // Noise in for the resolver,
            if let Some(noise_index) = self.find_connected_output_node(resolver as usize, 1) {
                if self.nodes[noise_index].role == MaterialFXNodeRole::Noise2D
                    || self.nodes[noise_index].role == MaterialFXNodeRole::Noise3D
                {
                    _ = self.nodes[noise_index].compute(
                        hit,
                        palette,
                        textures,
                        vec![],
                        &mat_obj_params[noise_index],
                    );
                }
            }

            // Execute the resolver
            _ = self.nodes[resolver as usize].compute(
                hit,
                palette,
                textures,
                resolved,
                &mat_obj_params[resolver as usize],
            );

            for (node, terminal) in follow_ups {
                hit.noise = None;
                hit.noise_scale = 1.0;
                self.follow_trail(
                    node as usize,
                    terminal as usize,
                    hit,
                    palette,
                    textures,
                    mat_obj_params,
                );
            }
        } else {
            // The node decides its own trail

            match connections.len() {
                0 => {}
                1 => {
                    let o = connections[0].0 as usize;

                    if let Some(noise_index) = self.find_connected_output_node(o, 1) {
                        if self.nodes[noise_index].role == MaterialFXNodeRole::Noise2D
                            || self.nodes[noise_index].role == MaterialFXNodeRole::Noise3D
                        {
                            _ = self.nodes[noise_index].compute(
                                hit,
                                palette,
                                textures,
                                vec![],
                                &mat_obj_params[noise_index],
                            );
                            // hit.uv += 7.23;
                            // let noise2 = self.nodes[noise_index].noise(hit);
                            // let wobble = vec2f(noise, noise2);
                            // hit.uv -= 7.23;
                            // hit.uv += wobble * 0.5;
                        }
                    }

                    if let Some(ot) =
                        self.nodes[o].compute(hit, palette, textures, vec![], &mat_obj_params[o])
                    {
                        hit.noise = None;
                        hit.noise_scale = 1.0;
                        self.follow_trail(o, ot as usize, hit, palette, textures, mat_obj_params);
                    }
                }
                _ => {
                    let index = (hit.hash * connections.len() as f32).floor() as usize;
                    if let Some(random_connection) = connections.get(index) {
                        let o = random_connection.0 as usize;

                        if let Some(noise_index) = self.find_connected_output_node(o, 1) {
                            if self.nodes[noise_index].role == MaterialFXNodeRole::Noise2D
                                || self.nodes[noise_index].role == MaterialFXNodeRole::Noise3D
                            {
                                _ = self.nodes[noise_index].compute(
                                    hit,
                                    palette,
                                    textures,
                                    vec![],
                                    &mat_obj_params[noise_index],
                                );
                            }
                        }

                        if let Some(ot) = self.nodes[o].compute(
                            hit,
                            palette,
                            textures,
                            vec![],
                            &mat_obj_params[o],
                        ) {
                            hit.noise = None;
                            hit.noise_scale = 1.0;
                            self.follow_trail(
                                o,
                                ot as usize,
                                hit,
                                palette,
                                textures,
                                mat_obj_params,
                            );
                        }
                    }
                }
            }
        }*/
    }

    /// Convert the model to a node canvas.
    pub fn to_canvas(&mut self, _palette: &ThePalette) -> TheNodeCanvas {
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

    pub fn render_preview(
        &mut self,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
    ) {
        let width = 111;
        let height = 104;

        let mut buffer = TheRGBABuffer::new(TheDim::sized(width as i32, height));

        let time = TheTime::default();
        let mat_obj_params = self.load_parameters(&time);

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                // let mut rng = rand::thread_rng();

                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let xx = (i % width) as f32;
                    let yy = (i / width) as f32;

                    let mut color = Vec4::zero();

                    let mut hit = Hit {
                        normal: Vec3::new(0., 1., 0.),
                        uv: Vec2::new(xx / width as f32, 1.0 - yy / height as f32),
                        two_d: true,
                        ..Default::default()
                    };

                    hit.hit_point = Vec3::new(hit.uv.x, 0.0, hit.uv.y);
                    hit.global_uv = hit.uv;
                    hit.pattern_pos = hit.global_uv;

                    self.compute(&mut hit, palette, textures, &mat_obj_params);

                    color.x = hit.mat.base_color.x;
                    color.y = hit.mat.base_color.y;
                    color.z = hit.mat.base_color.z;
                    color.w = 1.0;

                    pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                }
            });

        self.nodes[0].preview = buffer;
    }

    pub fn get_preview(&self) -> TheRGBABuffer {
        if self.nodes.is_empty() {
            TheRGBABuffer::empty()
        } else {
            self.nodes[0].preview.clone()
        }
    }

    /// Load a model from a JSON string.
    pub fn from_json(json: &str) -> Self {
        let material: MaterialFXObject = serde_json::from_str(json).unwrap_or_default();
        //let cnt = material.nodes.len();
        // for _ in 0..cnt {
        //     material.node_previews.push(None);
        // }
        material
    }

    /// Convert the model to a JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}
