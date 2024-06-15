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

    #[serde(default = "Vec2i::zero")]
    pub scroll_offset: Vec2i,
}

impl Default for MaterialFXObject {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialFXObject {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),

            name: "New Material".to_string(),

            nodes: Vec::new(),
            connections: Vec::new(),

            // node_previews: Vec::new(),
            zoom: 1.0,
            selected_node: None,

            scroll_offset: Vec2i::zero(),
        }
    }

    /// Computes the material
    pub fn compute(&self, hit: &mut Hit, palette: &ThePalette) {
        for (i, node) in self.nodes.iter().enumerate() {
            if node.role == MaterialFXNodeRole::Geometry {
                self.follow_trail(i, 0, hit, palette);
                break;
            }
        }
    }

    /// Computes the displacement if any
    pub fn displacement(&self, hit: &mut Hit) {
        for (i, node) in self.nodes.iter().enumerate() {
            if node.role == MaterialFXNodeRole::Geometry {
                if let Some((node, _)) = self.find_connected_input_node(i, 1) {
                    self.nodes[node as usize].displacement(hit);
                }
                break;
            }
        }
    }

    /// Returns true if the material supports displacement
    pub fn has_displacement(&self) -> bool {
        for (i, node) in self.nodes.iter().enumerate() {
            if node.role == MaterialFXNodeRole::Geometry {
                if let Some((_, _)) = self.find_connected_input_node(i, 1) {
                    return true;
                }
            }
        }
        false
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

    /// After exiting a geometry node follow the trail of material nodes to calculate the final material.
    pub fn follow_trail(
        &self,
        node: usize,
        terminal_index: usize,
        hit: &mut Hit,
        palette: &ThePalette,
    ) {
        let mut connections = vec![];
        for (o, ot, i, it) in &self.connections {
            if *o == node as u16 && *ot == terminal_index as u8 {
                connections.push((*i, *it));
            }
        }

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

            for (o, _) in &to_resolve {
                let mut h = hit.clone();
                _ = self.nodes[*o as usize].compute(&mut h, palette, vec![]);
                resolved.push(h);
            }
            _ = self.nodes[resolver as usize].compute(hit, palette, resolved);
        } else {
            // The node decides its own trail

            match connections.len() {
                0 => {}
                1 => {
                    let o = connections[0].0 as usize;

                    /*
                    let mut noise = 0.0;
                    if let Some(noise_index) = self.find_connected_output_node(o, 1) {
                        if let ModelFXNode::Noise3D(_coll) = &self.nodes[noise_index] {
                            noise = self.nodes[noise_index].noise(hit);
                            hit.uv += 7.23;
                            let noise2 = self.nodes[noise_index].noise(hit);
                            let wobble = vec2f(noise, noise2);
                            hit.uv -= 7.23;
                            hit.uv += wobble * 0.5;
                        }
                    }

                    */
                    if let Some(ot) = self.nodes[o].compute(hit, palette, vec![]) {
                        self.follow_trail(o, ot as usize, hit, palette);
                    }
                }
                _ => {
                    let index = (hit.hash * connections.len() as f32).floor() as usize;
                    if let Some(random_connection) = connections.get(index) {
                        let o = random_connection.0 as usize;
                        /*
                        let mut noise = 0.0;
                        if let Some(noise_index) = self.find_connected_output_node(o, 1) {
                            if let ModelFXNode::Noise3D(_coll) = &self.nodes[noise_index] {
                                noise = self.nodes[noise_index].noise(hit);
                            }
                            }*/
                        if let Some(ot) = self.nodes[o].compute(hit, palette, vec![]) {
                            self.follow_trail(o, ot as usize, hit, palette);
                        }
                    }
                }
            }
        }
    }

    /// Convert the model to a node canvas.
    pub fn to_canvas(&mut self, _palette: &ThePalette) -> TheNodeCanvas {
        let mut canvas = TheNodeCanvas {
            node_width: 136,
            selected_node: self.selected_node,
            ..Default::default()
        };

        //let preview_size = 100;

        for node in self.nodes.iter() {
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
            };
            canvas.nodes.push(n);
        }
        canvas.connections.clone_from(&self.connections);
        canvas.zoom = self.zoom;
        canvas.offset = self.scroll_offset;
        canvas.selected_node = self.selected_node;

        canvas
    }

    pub fn render_preview(&mut self, palette: &ThePalette) {
        let size: usize = 100;
        let mut buffer = TheRGBABuffer::new(TheDim::sized(size as i32, size as i32));

        fn distance(p: Vec3f) -> f32 {
            length(p) - 2.0
        }

        // fn distance(p: Vec3f) -> f32 {
        //     let q = abs(p) - vec3f(2.0, 2.0, 2.0);
        //     length(max(q, Vec3f::zero())) + min(max(q.x, max(q.y, q.z)), 0.0)
        // }

        pub fn normal(p: Vec3f) -> Vec3f {
            let scale = 0.5773 * 0.0005;
            let e = vec2f(1.0 * scale, -1.0 * scale);

            // IQs normal function

            let e1 = vec3f(e.x, e.y, e.y);
            let e2 = vec3f(e.y, e.y, e.x);
            let e3 = vec3f(e.y, e.x, e.y);
            let e4 = vec3f(e.x, e.x, e.x);

            let n = e1 * distance(p + e1)
                + e2 * distance(p + e2)
                + e3 * distance(p + e3)
                + e4 * distance(p + e4);
            normalize(n)
        }

        fn sphere_to_uv(hitpoint: Vec3f) -> Vec2f {
            let normalized_hitpoint = normalize(hitpoint);

            // Calculate spherical coordinates
            let theta = atan2(normalized_hitpoint.y, normalized_hitpoint.x);
            let phi = acos(normalized_hitpoint.z);

            // Map to UV coordinates
            let u = (theta + std::f32::consts::PI) / (2.0 * std::f32::consts::PI);
            let v = phi / std::f32::consts::PI;

            vec2f(u, v)
        }

        fn _get_uv_face(normal: Vec3f, hp: Vec3f) -> (Vec2f, usize) {
            // Calculate the absolute values of the normal components
            let abs_normal = abs(normal);

            // Determine which face of the cube was hit based on the maximum component of the normal
            let face_index = if abs_normal.x > abs_normal.y {
                if abs_normal.x > abs_normal.z {
                    0 // X-axis face
                } else {
                    2 // Z-axis face
                }
            } else if abs_normal.y > abs_normal.z {
                1 // Y-axis face
            } else {
                2 // Z-axis face
            };

            // Calculate UV coordinates based on the face
            match face_index {
                0 => (Vec2f::new(frac(hp.z), 1.0 - frac(hp.y)), 0), // X-axis face
                1 => (Vec2f::new(frac(hp.x), frac(hp.z)), 1),       // Y-axis face
                2 => (Vec2f::new(frac(hp.x), 1.0 - frac(hp.y)), 2), // Z-axis face
                _ => (Vec2f::zero(), 0),
            }
        }

        let ro = vec3f(0.0, 2.5, 2.5);
        // let ro = vec3f(2.0, 2.0, 2.0);
        let rd = vec3f(0.0, 0.0, 0.0);

        let camera = Camera::new(ro, rd, 90.0);

        let has_displacement = self.has_displacement();

        let noise2d = MaterialFXNode::new(MaterialFXNodeRole::Noise2D);

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(size * 4)
            .enumerate()
            .for_each(|(j, line)| {
                let mut rng = rand::thread_rng();

                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * size + i;

                    let xx = (i % size) as f32;
                    let yy = (i / size) as f32;

                    let mut color = Vec4f::zero();

                    for sample in 0..40 {
                        let mut ray = camera.create_ray(
                            vec2f(xx / size as f32, 1.0 - yy / size as f32),
                            vec2f(size as f32, size as f32),
                            vec2f(rng.gen(), rng.gen()),
                        );

                        let mut state = TracerState {
                            is_refracted: false,
                            has_been_refracted: false,
                            last_ior: 1.0,
                        };

                        let mut acc = Vec3f::zero();
                        let mut abso = Vec3f::one();
                        let mut hit: Option<Hit>;
                        let mut alpha = 0.0;
                        //let mut early_exit = false;

                        for depth in 0..8 {
                            hit = None;

                            let mut t = 0.0;
                            for _ in 0..20 {
                                let p = ray.at(t);
                                let mut d = distance(p);

                                if has_displacement {
                                    let normal = normal(p);
                                    let mut hit = Hit {
                                        hit_point: p,
                                        normal, //: normal(p),
                                        uv: sphere_to_uv(p),
                                        // uv: get_uv_face(p, normal).0,
                                        distance: t,
                                        ..Default::default()
                                    };
                                    noise2d.compute(&mut hit, palette, vec![]);
                                    self.displacement(&mut hit);
                                    d += hit.displacement;
                                }

                                if d.abs() < 0.0001 {
                                    let normal = normal(p);
                                    let mut h = Hit {
                                        hit_point: p,
                                        normal,
                                        uv: sphere_to_uv(p),
                                        // uv: get_uv_face(p, normal).0,
                                        distance: t,
                                        albedo: Vec3f::zero(),
                                        ..Default::default()
                                    };
                                    noise2d.compute(&mut h, palette, vec![]);

                                    for (i, node) in self.nodes.iter().enumerate() {
                                        if node.role == MaterialFXNodeRole::Geometry {
                                            self.follow_trail(i, 0, &mut h, palette);

                                            alpha = 1.0;

                                            hit = Some(h);
                                            break;
                                        }
                                    }
                                }
                                if hit.is_some() {
                                    break;
                                }
                                t += d;
                            }

                            if let Some(hit) = &mut hit {
                                let mut n = hit.normal;
                                if state.is_refracted {
                                    n = -n
                                };

                                // sample BSDF
                                let mut out_dir: Vec3f = Vec3f::zero();
                                let bsdf = sample_disney_bsdf(
                                    -ray.d,
                                    n,
                                    hit,
                                    &mut out_dir,
                                    &mut state,
                                    &mut rng,
                                );

                                // add emissive part of the current material
                                acc += hit.emissive * abso;

                                // bsdf absorption (pdf are in bsdf.a)
                                if bsdf.1 > 0.0 {
                                    abso *= bsdf.0 / bsdf.1;
                                }

                                // medium absorption
                                if state.has_been_refracted {
                                    abso *= exp(-hit.distance
                                        * ((Vec3f::one() - hit.albedo) * hit.absorption));
                                }

                                ray.o = hit.hit_point;
                                ray.d = out_dir;

                                if state.is_refracted {
                                    ray.o += -n * 0.01;
                                } else if state.has_been_refracted && !state.is_refracted {
                                    ray.o += -n * 0.01;
                                    state.last_ior = 1.;
                                } else {
                                    ray.o += n * 0.01;
                                }
                            } else {
                                //acc += vec3f(0.5, 0.5, 0.5) * abso;
                                acc += vec3f(1.0, 1.0, 1.0) * abso;
                                if depth == 0 {
                                    //early_exit = true;
                                };
                                break;
                            }
                        }
                        let c = vec4f(acc.x, acc.y, acc.z, alpha);
                        color = lerp(color, c, 1.0 / (sample as f32 + 1.0));
                        // if early_exit {
                        //     break;
                        // }
                    }

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
