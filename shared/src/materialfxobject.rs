use crate::prelude::*;
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

    #[serde(skip)]
    pub node_previews: Vec<Option<TheRGBABuffer>>,

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

            node_previews: Vec::new(),
            zoom: 1.0,
            selected_node: None,

            scroll_offset: Vec2i::zero(),
        }
    }

    /// Computes the material
    pub fn compute(&self, hit: &mut Hit, palette: &ThePalette) {
        let mut material_index = None;
        for (i, node) in self.nodes.iter().enumerate() {
            if node.role == MaterialFXNodeRole::Material {
                material_index = Some(i);
                break;
            }
        }

        if let Some(material_index) = material_index {
            let material_node = &self.nodes[material_index];

            let p = material_node.get("Color");
            if let Some(TheValue::PaletteIndex(i)) = p {
                if let Some(c) = &palette.colors[i as usize] {
                    hit.albedo = c.to_vec3f();
                }
            }
        } else {
            hit.albedo = Vec3f::new(0.5, 0.5, 0.5);
        }
    }

    /// Convert the model to a node canvas.
    pub fn to_canvas(&mut self, _palette: &ThePalette) -> TheNodeCanvas {
        let mut canvas = TheNodeCanvas {
            node_width: 95,
            ..Default::default()
        };

        let preview_size = (40.0 * self.zoom) as i32;

        for (i, node) in self.nodes.iter().enumerate() {
            if i >= self.node_previews.len() {
                self.node_previews.resize(i + 1, None);
            }

            // Remove preview buffer if size has changed
            if let Some(preview_buffer) = &self.node_previews[i] {
                if preview_buffer.dim().width != preview_size
                    && preview_buffer.dim().height != preview_size
                {
                    self.node_previews[i] = None;
                }
            }

            // Create preview if it doesn't exist
            if self.node_previews[i].is_none() {
                let preview_buffer = TheRGBABuffer::new(TheDim::sized(preview_size, preview_size));
                //self.render_node_preview(&mut preview_buffer, i, palette);
                self.node_previews[i] = Some(preview_buffer);
            }

            let n = TheNode {
                name: node.name(),
                position: node.position,
                inputs: node.inputs(),
                outputs: node.outputs(),
                preview: self.node_previews[i].clone().unwrap(),
            };
            canvas.nodes.push(n);
        }
        canvas.connections.clone_from(&self.connections);
        canvas.zoom = self.zoom;
        canvas.offset = self.scroll_offset;
        canvas.selected_node = self.selected_node;

        canvas
    }
}
