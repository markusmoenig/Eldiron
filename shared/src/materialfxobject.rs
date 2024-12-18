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

            scroll_offset: Vec2i::zero(),
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
        material_index: usize,
        p: Vec3f,
        hit: &mut Hit,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        mat_obj_params: &[Vec<f32>],
    ) -> Vec3f {
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

                    let mut color = Vec4f::zero();

                    let mut hit = Hit {
                        normal: vec3f(0., 1., 0.),
                        uv: vec2f(xx / width as f32, 1.0 - yy / height as f32),
                        two_d: true,
                        ..Default::default()
                    };

                    hit.hit_point = vec3f(hit.uv.x, 0.0, hit.uv.y);
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

    pub fn render_preview_3d(
        &mut self,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        buffer: &mut TheRGBABuffer,
        sample: i32,
    ) {
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height;

        let time = TheTime::default();
        let mat_obj_params = self.load_parameters(&time);

        let camera = Camera::new(vec3f(0., 0., 2.), Vec3f::zero(), 70.0);

        let has_bump = self.has_bump();

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                let mut rng = rand::thread_rng();

                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let xx = (i % width) as f32;
                    let yy = (i / width) as f32;

                    let mut color = Vec4f::new(0.0, 0.0, 0.0, 0.0);

                    let mut ray = camera.create_ray(
                        vec2f(xx / width as f32, 1.0 - yy / height as f32),
                        vec2f(width as f32, height as f32),
                        vec2f(rng.gen(), rng.gen()),
                    );

                    let mut radiance = Vec3f::zero();
                    let mut throughput = Vec3f::one();

                    let mut state = BSDFState::default();
                    //let mut light_sample = BSDFLightSampleRec::default();
                    //let mut scatter_sample = BSDFScatterSampleRec::default();

                    // For medium tracking
                    let mut _in_medium = false;
                    let mut _medium_sampled = false;
                    let mut _surface_scatter = false;

                    for depth in 0..8 {
                        let mut hit = Hit::default();
                        let mut has_hit = false;

                        let mut t = 0.0;

                        for _ in 0..100 {
                            let p = ray.at(t);

                            let mut d = length(p) - 1.0;

                            hit.hit_point = p;
                            hit.global_uv.x = p.x + 1.0;
                            hit.global_uv.y = p.y + 1.0;
                            hit.pattern_pos = hit.global_uv;
                            hit.uv = vec2f((p.x + 0.5).fract(), (p.y + 0.5).fract());
                            hit.distance = t;

                            if t > 3.0 {
                                break;
                            }

                            if has_bump {
                                hit.mode = HitMode::Bump;
                                self.follow_trail(
                                    0,
                                    0,
                                    &mut hit,
                                    palette,
                                    textures,
                                    &mat_obj_params,
                                );

                                d -= hit.bump;
                            }

                            if d < 0.001 {
                                has_hit = true;
                                hit.hit_point = p;
                                hit.global_uv.x = p.x + 1.0;
                                hit.global_uv.y = p.y + 1.0;
                                hit.pattern_pos = hit.global_uv;
                                hit.uv = vec2f((p.x + 0.5).fract(), (p.y + 0.5).fract());
                                hit.distance = t;
                                break;
                            }
                            t += d * 0.5;
                        }

                        if has_hit {
                            hit.normal = normalize(hit.hit_point);
                            // hit.normal = self.get_material_normal(
                            //     0,
                            //     hit.hit_point,
                            //     &mut hit,
                            //     palette,
                            //     textures,
                            //     &mat_obj_params,
                            // );

                            hit.mode = HitMode::Albedo;
                            self.compute(&mut hit, palette, textures, &mat_obj_params);

                            state.depth = depth;

                            state.mat.clone_from(&hit.mat);
                            state.mat.base_color = powf(hit.mat.base_color, 2.2);
                            state.mat.roughness = max(state.mat.roughness, 0.001);
                            // Remapping from clearcoat gloss to roughness
                            state.mat.clearcoat_roughness =
                                lerp(0.1, 0.001, state.mat.clearcoat_roughness);

                            state.hit_dist = hit.distance;
                            state.fhp = hit.hit_point;

                            state.normal = hit.normal;
                            state.ffnormal = if dot(state.normal, ray.d) <= 0.0 {
                                state.normal
                            } else {
                                -state.normal
                            };

                            state.eta = if dot(ray.d, state.normal) < 0.0 {
                                1.0 / state.mat.ior
                            } else {
                                state.mat.ior
                            };

                            onb(state.normal, &mut state.tangent, &mut state.bitangent);

                            let aspect = sqrt(1.0 - state.mat.anisotropic * 0.9);
                            state.mat.ax = max(0.001, state.mat.roughness / aspect);
                            state.mat.ay = max(0.001, state.mat.roughness * aspect);

                            // --- Sample light
                            //
                            let mut light_sample = BSDFLightSampleRec::default();
                            let mut scatter_sample = BSDFScatterSampleRec::default();

                            let scatter_pos = state.fhp + state.normal * hit.eps;

                            let light_pos = vec3f(1.0, 0.0, 3.0);

                            let radius = 0.2;

                            let l = BSDFLight {
                                position: light_pos,
                                emission: Vec3f::one() * 10.0,
                                radius,
                                type_: 1.0,
                                u: Vec3f::zero(),
                                v: Vec3f::zero(),
                                area: 4.0 * f32::pi() * radius * radius,
                            };

                            sample_sphere_light(
                                &l,
                                scatter_pos,
                                &mut light_sample,
                                1,
                                &mut rng,
                                5.0,
                            );

                            let li = light_sample.emission;

                            if ray_sphere(
                                Ray::new(scatter_pos, light_sample.direction),
                                light_pos,
                                radius,
                            )
                            .is_some()
                            {
                                scatter_sample.f = disney_eval(
                                    &state,
                                    -ray.d,
                                    state.ffnormal,
                                    light_sample.direction,
                                    &mut scatter_sample.pdf,
                                );

                                let mut mis_weight = 1.0;
                                if l.area > 0.0 {
                                    // No MIS for distant light
                                    mis_weight =
                                        power_heuristic(light_sample.pdf, scatter_sample.pdf);
                                }

                                let mut ld = Vec3f::zero();

                                if scatter_sample.pdf > 0.0 {
                                    ld += (mis_weight * li * scatter_sample.f / light_sample.pdf)
                                        * throughput;
                                }

                                radiance += ld * throughput;
                            }
                            //

                            scatter_sample.f = disney_sample(
                                &state,
                                -ray.d,
                                state.ffnormal,
                                &mut scatter_sample.l,
                                &mut scatter_sample.pdf,
                                &mut rng,
                            );
                            if scatter_sample.pdf > 0.0 {
                                throughput *= scatter_sample.f / scatter_sample.pdf;
                            } else {
                                break;
                            }

                            ray.d = scatter_sample.l;
                            ray.o = state.fhp + ray.d * 0.01;

                            color.x = radiance.x;
                            color.y = radiance.y;
                            color.z = radiance.z;
                            color.w = 1.0;
                        } else {
                            // Env color
                            color.x += throughput.x * 0.5;
                            color.y += throughput.y * 0.5;
                            color.z += throughput.z * 0.5;
                            break;
                        }
                    }

                    if sample == 0 {
                        pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                    } else {
                        let mut ex = Vec4f::zero();

                        ex.x = pixel[0] as f32 / 255.0;
                        ex.y = pixel[1] as f32 / 255.0;
                        ex.z = pixel[2] as f32 / 255.0;
                        ex.w = pixel[3] as f32 / 255.0;

                        //color = powf(color, 0.4545);
                        //color = clamp(color, Vec4f::zero(), vec4f(1.0, 1.0, 1.0, 1.0));

                        fn aces(x: Vec3f) -> Vec3f {
                            let a = 2.51;
                            let b = 0.03;
                            let c = 2.43;
                            let d = 0.59;
                            let e = 0.14;

                            (x * (a * x + b)) / (x * (c * x + d) + e)
                        }

                        let mut c = aces(color.xyz());
                        c = powf(c, 1.0 / 2.2);

                        color[0] = c[0];
                        color[1] = c[1];
                        color[2] = c[2];

                        let s = 1.0 / (sample as f32 + 1.0);
                        let accumulated_color = lerp(ex, color, s);
                        // let accumulated_color =
                        //     (ex * (sample as f32) + color) / (sample as f32 + 1.0);

                        pixel.copy_from_slice(
                            &TheColor::from_vec4f(accumulated_color).to_u8_array(),
                        );
                    }
                }
            });
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
