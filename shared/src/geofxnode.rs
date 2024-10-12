use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum GeoFXNodeRole {
    Table,

    LeftWall,
    BackWall,
    RightWall,
    FrontWall,
    MiddleWallH,
    MiddleWallV,

    Box,
    Disc,
    Group,

    Material,

    Repeat,
    Stack,

    MetaMaterial,
    MetaDelete,

    Modeling,
}

use GeoFXNodeRole::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXNode {
    pub id: Uuid,
    pub role: GeoFXNodeRole,
    pub timeline: TheTimeline,

    pub position: Vec2i,

    pub supports_preview: bool,
    pub preview_is_open: bool,

    pub preview: TheRGBABuffer,
}

impl GeoFXNode {
    pub fn is_shape(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match &self.role {
            Box => true,
            Disc => true,
            _ => false,
        }
    }

    pub fn new(role: GeoFXNodeRole) -> Self {
        let mut coll = TheCollection::named(str!("Geo"));
        let supports_preview = false;
        let preview_is_open = false;

        match role {
            Table => {}
            Box => {
                coll.set("Length", TheValue::FloatRange(1.0, 0.001..=1.0));
                coll.set("Height", TheValue::FloatRange(1.0, 0.001..=3.0));
                coll.set("Rounding", TheValue::Text(str!("0.0")));
                coll.set("Rotation", TheValue::Text(str!("0.0")));
                coll.set("Annular", TheValue::Text(str!("0.0")));
                coll.set("Extrusion", TheValue::Text(str!("thickness")));
            }
            Disc => {
                coll.set("Radius", TheValue::FloatRange(0.5, 0.0..=1.0));
                coll.set("Annular", TheValue::Text(str!("0.0")));
                coll.set("Extrusion", TheValue::Text(str!("thickness")));
            }
            LeftWall | FrontWall | RightWall | BackWall | MiddleWallH | MiddleWallV => {
                coll.set("Pos X", TheValue::Float(0.1));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("Length", TheValue::FloatRange(1.0, 0.001..=1.0));
                coll.set("Height", TheValue::FloatRange(1.0, 0.001..=3.0));
                coll.set("Thickness", TheValue::FloatRange(0.1, 0.001..=1.0));
                coll.set(
                    "2D Mode",
                    TheValue::TextList(
                        0,
                        vec![
                            str!("Normal"),
                            str!("-1 Pos, +1 Length"),
                            str!("-1 Pos, +2 Length"),
                        ],
                    ),
                );
            }
            Material => {
                coll.set("Color", TheValue::PaletteIndex(0));
                coll.set("Modifier", TheValue::Text(str!("0.0")));
                coll.set("Roughness", TheValue::Text(str!("0.5")));
                coll.set("Metallic", TheValue::Text(str!("0.0")));
                coll.set("Anisotropic", TheValue::Text(str!("0.0")));
                coll.set("Subsurface", TheValue::Text(str!("0.0")));
                coll.set("Specular Tint", TheValue::Text(str!("0.0")));
                coll.set("Sheen", TheValue::Text(str!("0.0")));
                coll.set("Sheen Tint", TheValue::Text(str!("0.0")));
                coll.set("Clearcoat", TheValue::Text(str!("0.0")));
                coll.set("Clearcoat Gloss", TheValue::Text(str!("0.0")));
                coll.set("Transmission", TheValue::Text(str!("0.0")));
                coll.set("Emission", TheValue::Text(str!("0.0")));
                coll.set("IOR", TheValue::Text(str!("1.5")));
                coll.set("Texture", TheValue::Text(str!("")));
            }
            Repeat => {
                coll.set("Spacing", TheValue::FloatRange(0.01, 0.0..=1.0));
                coll.set("Offset", TheValue::FloatRange(0.0, 0.0..=1.0));
            }
            Stack => {
                coll.set("Spacing", TheValue::FloatRange(0.01, 0.0..=1.0));
                coll.set("Offset", TheValue::FloatRange(0.0, 0.0..=1.0));
            }
            Group => {
                coll.set("X", TheValue::FloatRange(0.0, 0.0..=10.0));
                coll.set("Y", TheValue::FloatRange(0.0, 0.0..=10.0));
            }
            MetaMaterial => {
                coll.set("Meta", TheValue::Text(str!("")));
            }
            MetaDelete => {
                coll.set("Meta", TheValue::Text(str!("")));
            }
            Modeling => {
                // coll.set(
                //     "Direction",
                //     TheValue::TextList(0, vec![str!("Horizontal"), str!("Vertical")]),
                // );
            }
        }
        let timeline = TheTimeline::collection(coll);

        Self {
            id: Uuid::new_v4(),
            role,
            timeline,
            position: Vec2i::new(10, 5),
            supports_preview,
            preview_is_open,
            preview: TheRGBABuffer::empty(),
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![
            Self::new(GeoFXNodeRole::Table),
            Self::new(GeoFXNodeRole::LeftWall),
            Self::new(GeoFXNodeRole::BackWall),
            Self::new(GeoFXNodeRole::RightWall),
            Self::new(GeoFXNodeRole::FrontWall),
            Self::new(GeoFXNodeRole::MiddleWallH),
            Self::new(GeoFXNodeRole::MiddleWallV),
            Self::new(GeoFXNodeRole::Box),
            Self::new(GeoFXNodeRole::Disc),
            Self::new(GeoFXNodeRole::Material),
            Self::new(GeoFXNodeRole::Repeat),
            Self::new(GeoFXNodeRole::Stack),
            Self::new(GeoFXNodeRole::Group),
            Self::new(GeoFXNodeRole::MetaMaterial),
            Self::new(GeoFXNodeRole::MetaDelete),
            Self::new(GeoFXNodeRole::Modeling),
        ]
    }

    pub fn name(&self) -> String {
        match &self.role {
            Table => str!("A table."),
            LeftWall => str!("Left Wall"),
            BackWall => str!("Back Wall"),
            RightWall => str!("Right Wall"),
            FrontWall => str!("Front Wall"),
            MiddleWallH => str!("Middle Wall X"),
            MiddleWallV => str!("Niddle Wall Y"),
            Box => "Box".to_string(),
            Disc => "Disc".to_string(),
            Material => "Material".to_string(),
            Repeat => "Repeat".to_string(),
            Stack => "Stack".to_string(),
            Group => "Group".to_string(),
            MetaMaterial => "Meta Material".to_string(),
            MetaDelete => "Meta Delete".to_string(),
            Modeling => "Modeling".to_string(),
        }
    }

    /// Creates a new node from a name.
    pub fn new_from_name(name: String) -> Self {
        let nodes = GeoFXNode::nodes();
        for n in nodes {
            if n.name() == name {
                return n;
            }
        }
        GeoFXNode::new(MiddleWallH)
    }

    pub fn icon_name(&self) -> Option<String> {
        match self.role {
            LeftWall => Some(str!("geo_wall_left")),
            BackWall => Some(str!("geo_wall_back")),
            RightWall => Some(str!("geo_wall_right")),
            FrontWall => Some(str!("geo_wall_front")),
            MiddleWallH => Some(str!("geo_wall_middlex")),
            MiddleWallV => Some(str!("geo_wall_middley")),
            _ => None,
        }
    }

    pub fn description(&self) -> String {
        match &self.role {
            Table => str!("A table."),
            LeftWall => str!("A wall on the left side of the tile."),
            BackWall => str!("A wall on the back side of the tile."),
            RightWall => str!("A wall on the right side of the tile."),
            FrontWall => str!("A wall on the front side of the tile."),
            MiddleWallH => str!("A horizontal wall in the middle of the tile."),
            MiddleWallV => str!("A vertical wall in the middle of the tile."),
            _ => "".to_string(),
        }
    }

    /// Returns the layer role (RemoveHeightBrush, Wall etc) for this node.
    pub fn get_layer_role(&self) -> Layer2DRole {
        match self.role {
            GeoFXNodeRole::Table => Layer2DRole::Ground,
            _ => Layer2DRole::Wall,
        }
    }

    pub fn build(
        &self,
        palette: &ThePalette,
        _textures: &FxHashMap<Uuid, TheRGBATile>,
        ctx: &mut FTBuilderContext,
    ) {
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            let mut shape_params = str!("");

            if self.is_shape() {
                if let Some(value) = coll
                    .get_default("Extrusion", TheValue::Text(str!("thickness")))
                    .to_string()
                {
                    if value != "thickness" {
                        shape_params += &format!(", extrusion = {}", value);
                    }
                }
                if let Some(value) = coll
                    .get_default("Rotation", TheValue::Text(str!("0.0")))
                    .to_string()
                {
                    if value != "0.0" {
                        shape_params += &format!(", rotation = {}", value);
                    }
                }
                if let Some(value) = coll
                    .get_default("Annular", TheValue::Text(str!("0.0")))
                    .to_string()
                {
                    if value != "0.0" {
                        shape_params += &format!(", annular = {}", value);
                    }
                }
            }

            match self.role {
                Box => {
                    let geo = format!(
                        "let box_{id_counter} = Shape<Box>: material = {material}, length = {length}, height = {height}, rounding = {rounding}{shape_params};\n",
                        id_counter = { ctx.id_counter },
                        material = { if ctx.material_id.is_some() { ctx.material_id.clone().unwrap()} else {str!("none") }},
                        length = coll.get_f32_default("Length", 1.0),
                        height = coll.get_f32_default("Height", 1.0),
                        rounding = coll.get_f32_default("Rounding", 0.0) / 2.0,
                    );
                    ctx.geometry.push(format!("box_{}", ctx.id_counter));
                    ctx.out += &geo;
                    ctx.id_counter += 1;
                    ctx.material_id = None;
                }
                Disc => {
                    let geo = format!(
                        "let disc_{id_counter} = Shape<Disc>: material = {material}, radius = {radius}{shape_params};\n",
                        id_counter = { ctx.id_counter },
                        material = { if ctx.material_id.is_some() { ctx.material_id.clone().unwrap()} else {str!("none") }},
                        radius = coll.get_f32_default("Radius", 0.5),
                    );
                    ctx.geometry.push(format!("disc_{}", ctx.id_counter));
                    ctx.out += &geo;
                    ctx.id_counter += 1;
                    ctx.material_id = None;
                }
                Repeat => {
                    let geometry = ctx.geometry.join(",");
                    let geo = format!(
                        "let pattern_{id_counter} = Pattern<Repeat>: content = [{geometry}], spacing = {spacing}, offset = {offset};\n",
                        id_counter = { ctx.id_counter },
                        spacing = coll.get_f32_default("Spacing", 0.01),
                        offset = coll.get_f32_default("Offset", 0.0),
                        geometry = geometry
                    );
                    ctx.geometry.clear();
                    ctx.geometry.push(format!("pattern_{}", ctx.id_counter));
                    ctx.out += &geo;
                    ctx.id_counter += 1;
                    ctx.material_id = None;
                }
                Stack => {
                    let geometry = ctx.geometry.join(",");
                    let geo = format!(
                        "let pattern_{id_counter} = Pattern<Stack>: content = [{geometry}], spacing = {spacing}, offset = {offset};\n",
                        id_counter = { ctx.id_counter },
                        spacing = coll.get_f32_default("Spacing", 0.01),
                        offset = coll.get_f32_default("Offset", 0.0),
                        geometry = geometry
                    );
                    ctx.geometry.clear();
                    ctx.geometry.push(format!("pattern_{}", ctx.id_counter));
                    ctx.out += &geo;
                    ctx.id_counter += 1;
                    ctx.material_id = None;
                }
                Group => {
                    let mut cut_param = str!("");
                    if let Some(cutout) = &ctx.cut_out {
                        cut_param = cutout.clone();
                    }

                    let geometry = ctx.geometry.join(",");
                    let geo = format!(
                        "let pattern_{id_counter} = Pattern<Group>: content = [{geometry}], x = {x}, y = {y}, cutout = [{cutout}];\n",
                        id_counter = { ctx.id_counter },
                        x = coll.get_f32_default("X", 0.0),
                        y = coll.get_f32_default("Y", 0.0),
                        cutout = cut_param,
                        geometry = geometry
                    );
                    ctx.geometry.clear();
                    ctx.geometry.push(format!("pattern_{}", ctx.id_counter));
                    ctx.out += &geo;
                    ctx.id_counter += 1;
                    ctx.material_id = None;
                }
                Material => {
                    let mut hex = "000000".to_string();
                    let color_index = coll.get_i32_default("Color", 0);
                    if let Some(color) = &palette.colors[color_index as usize] {
                        hex = color.to_hex();
                        hex.remove(0);
                    }

                    let mut parameters = "".to_string();

                    // Modifier
                    if let Some(modifier) = coll
                        .get_default("Modifier", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if modifier != "0.0" {
                            parameters += &format!(", modifier = {}", modifier);
                        }
                    }
                    // Roughness
                    if let Some(value) = coll
                        .get_default("Roughness", TheValue::Text(str!("0.5")))
                        .to_string()
                    {
                        if value != "0.5" {
                            parameters += &format!(", roughness = {}", value);
                        }
                    }
                    // Metallic
                    if let Some(value) = coll
                        .get_default("Metallic", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if value != "0.0" {
                            parameters += &format!(", metallic = {}", value);
                        }
                    }
                    // Anisotropic
                    if let Some(value) = coll
                        .get_default("Anisotropic", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if value != "0.0" {
                            parameters += &format!(", anisotropic = {}", value);
                        }
                    }
                    // Subsurface
                    if let Some(value) = coll
                        .get_default("Subsurface", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if value != "0.0" {
                            parameters += &format!(", subsurface = {}", value);
                        }
                    }
                    // Specular Tint
                    if let Some(value) = coll
                        .get_default("Specular Tint", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if value != "0.0" {
                            parameters += &format!(", specular_tint = {}", value);
                        }
                    }
                    // Sheen
                    if let Some(value) = coll
                        .get_default("Sheen", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if value != "0.0" {
                            parameters += &format!(", sheen = {}", value);
                        }
                    }
                    // Sheen Tint
                    if let Some(value) = coll
                        .get_default("Sheen Tint", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if value != "0.0" {
                            parameters += &format!(", sheen_tint = {}", value);
                        }
                    }
                    // Clearcoat
                    if let Some(value) = coll
                        .get_default("Clearcoat", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if value != "0.0" {
                            parameters += &format!(", clearcoat = {}", value);
                        }
                    }
                    // Clearcoat Gloss
                    if let Some(value) = coll
                        .get_default("Clearcoat Gloss", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if value != "0.0" {
                            parameters += &format!(", clearcoat_gloss = {}", value);
                        }
                    }
                    // Transmission
                    if let Some(value) = coll
                        .get_default("Transmission", TheValue::Text(str!("0.0")))
                        .to_string()
                    {
                        if value != "0.0" {
                            parameters += &format!(", transmission = {}", value);
                        }
                    }
                    // IOR
                    if let Some(value) = coll
                        .get_default("IOR", TheValue::Text(str!("1.5")))
                        .to_string()
                    {
                        if value != "1.5" {
                            parameters += &format!(", ior = {}", value);
                        }
                    }

                    // println!("parameters {}", parameters);

                    let mat = format!(
                        "let material_{id_counter} = Material<BSDF>: color = #{hex}{parameters};\n",
                        id_counter = { ctx.id_counter },
                        hex = { hex },
                        parameters = { parameters }
                    );
                    ctx.out += &mat;
                    ctx.material_id = Some(format!("material_{}", ctx.id_counter));
                    ctx.id_counter += 1;
                }
                LeftWall | MiddleWallV | RightWall | BackWall | MiddleWallH | FrontWall => {
                    let face_type = match &self.role {
                        LeftWall => "Left",
                        MiddleWallV => "MiddleY",
                        RightWall => "Right",
                        BackWall => "Back",
                        MiddleWallH => "MiddleX",
                        FrontWall => "Front",
                        _ => "",
                    };

                    let geometry = ctx.geometry.join(",");
                    let geo = format!(
                        "let face = Face<{face_type}> : length = {length}, height = {height}, thickness = {thickness}, content = [{geometry}];\n",
                        face_type = face_type,
                        length = coll.get_f32_default("Length", 1.0),
                        height = coll.get_f32_default("Height", 1.0),
                        thickness = coll.get_f32_default("Thickness", 0.2),
                        geometry = geometry
                    );
                    ctx.out += &geo;
                }
                MetaMaterial => {
                    if let Some(value) = coll
                        .get_default("Meta", TheValue::Text(str!("")))
                        .to_string()
                    {
                        if !value.is_empty() {
                            let geo = format!(
                                "let meta_{id_counter}  = Meta<Material> : material = {material}, content = [{meta}];\n",
                                id_counter = { ctx.id_counter },
                                material = { if ctx.material_id.is_some() { ctx.material_id.clone().unwrap()} else {str!("none") }},
                                meta = { value }
                            );
                            ctx.id_counter += 1;
                            ctx.out += &geo;
                            ctx.material_id = None;
                        }
                    }
                }
                MetaDelete => {
                    if let Some(value) = coll
                        .get_default("Meta", TheValue::Text(str!("")))
                        .to_string()
                    {
                        if !value.is_empty() {
                            let geo = format!(
                                "let meta_{id_counter}  = Meta<Delete> : content = [{meta}];\n",
                                id_counter = { ctx.id_counter },
                                meta = { value }
                            );
                            ctx.id_counter += 1;
                            ctx.out += &geo;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Returns all tiles which are touched by this geometry.
    pub fn area(&self, no_2d_transforms: bool) -> Vec<Vec2i> {
        let mut area = Vec::new();
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            match self.role {
                /*
                Column => {
                    let radius = coll.get_f32_default("Radius", 0.4);

                    let center = self.position(&coll);
                    let min_x = (center.x - radius).floor() as i32;
                    let max_x = (center.x + radius).ceil() as i32;
                    let min_y = (center.y - radius).floor() as i32;
                    let max_y = (center.y + radius).ceil() as i32;

                    fn tile_intersects_disc(center: Vec2f, radius: f32, x: i32, y: i32) -> bool {
                        let closest_x = if center.x < x as f32 {
                            x as f32
                        } else if center.x > (x + 1) as f32 {
                            (x + 1) as f32
                        } else {
                            center.x
                        };
                        let closest_y = if center.y < y as f32 {
                            y as f32
                        } else if center.y > (y + 1) as f32 {
                            (y + 1) as f32
                        } else {
                            center.y
                        };

                        let dist_x = center.x - closest_x;
                        let dist_y = center.y - closest_y;

                        dist_x * dist_x + dist_y * dist_y <= radius * radius
                    }

                    for x in min_x..=max_x {
                        for y in min_y..=max_y {
                            if tile_intersects_disc(center, radius, x, y) {
                                area.push(Vec2i::new(x, y));
                            }
                        }
                    }
                }*/
                LeftWall | RightWall | MiddleWallV => {
                    let mut pos = Vec2i::from(self.position(&coll));
                    let mut length = self.length().ceil() as i32;

                    if !no_2d_transforms {
                        if let Some(value) = coll.get("2D Mode") {
                            if let Some(mode) = value.to_i32() {
                                let is_vertical = self.is_vertical();
                                if is_vertical {
                                    if mode == 1 {
                                        pos.y -= 1;
                                        length += 1;
                                    } else if mode == 2 {
                                        pos.y -= 1;
                                        length += 2;
                                    }
                                }
                            }
                        }
                    }

                    for i in 0..length {
                        area.push(Vec2i::new(pos.x, pos.y + i));
                    }
                }
                BackWall | FrontWall | MiddleWallH => {
                    let mut pos = Vec2i::from(self.position(&coll));
                    let mut length = self.length().ceil() as i32;

                    if let Some(value) = coll.get("2D Mode") {
                        if let Some(mode) = value.to_i32() {
                            let is_vertical = self.is_vertical();
                            if !is_vertical {
                                if mode == 1 {
                                    pos.x -= 1;
                                    length += 1;
                                } else if mode == 2 {
                                    pos.x -= 1;
                                    length += 2;
                                }
                            }
                        }
                    }

                    for i in 0..length {
                        area.push(Vec2i::new(pos.x + i, pos.y));
                    }
                }
                _ => {
                    area.push(Vec2i::from(self.position(&coll)));
                }
            }
        }
        area
    }

    /// Returns the length of the geometry.
    pub fn length(&self) -> f32 {
        let mut length = 1.0;
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            if let Some(h) = coll.get("Length") {
                if let Some(h) = h.to_f32() {
                    length = h;
                }
            }
        }
        length
    }

    /// Returns the height of the geometry.
    pub fn height(&self) -> f32 {
        let mut height = 1.0;
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            if let Some(h) = coll.get("Height") {
                if let Some(h) = h.to_f32() {
                    height = h;
                }
            }
        }
        height
    }

    /// Returns the thickness of the geometry.
    pub fn thickness(&self) -> f32 {
        let mut thickness = 0.2;
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            if let Some(h) = coll.get("Thickness") {
                if let Some(h) = h.to_f32() {
                    thickness = h;
                }
            }
        }
        thickness
    }

    #[inline(always)]
    pub fn position(&self, coll: &TheCollection) -> Vec2f {
        let x = coll.get_f32_default("Pos X", 0.0);
        let y = coll.get_f32_default("Pos Y", 0.0);
        vec2f(x, y)
    }

    /// Set the position
    pub fn set_position(&mut self, pos: Vec2f) {
        self.set("Pos X", TheValue::Float(pos.x));
        self.set("Pos Y", TheValue::Float(pos.y));
    }

    pub fn set_default_position(&mut self, p: Vec2i) {
        let pf = vec2f(p.x as f32, p.y as f32);
        /*
        match self.role {
            LeftWall => {
                pf.x += 0.1;
                pf.y += 0.5;
            }
            TopWall => {
                pf.x += 0.5;
                pf.y += 0.1;
            }
            RightWall => {
                pf.x += 0.9;
                pf.y += 0.5;
            }
            BottomWall => {
                pf.x += 0.5;
                pf.y += 0.9;
            }
            _ => {
                pf.x += 0.5;
                pf.y += 0.5;
            }
        }*/
        self.set("Pos X", TheValue::Float(pf.x));
        self.set("Pos Y", TheValue::Float(pf.y));
    }

    pub fn collection(&self) -> TheCollection {
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            return coll;
        }

        TheCollection::default()
    }

    pub fn set(&mut self, key: &str, value: TheValue) {
        self.timeline.set(&TheTime::default(), key, "Geo", value);
    }

    pub fn is_blocking(&self) -> bool {
        // match self.role {
        //     RemoveHeightBrush => false,
        //     AddHeightBrush => {
        //         if let Some(coll) = self
        //             .timeline
        //             .get_collection_at(&TheTime::default(), str!("Geo"))
        //         {
        //             let height = coll.get_f32_default("Height", 0.01);
        //             height > 0.3
        //         } else {
        //             false
        //         }
        //     }
        //     _ => true,
        // }
        true
    }

    /// Return true if the object has a vertical alignment.
    pub fn is_vertical(&self) -> bool {
        matches!(self.role, LeftWall | RightWall | MiddleWallV)
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            LeftWall | BackWall | RightWall | FrontWall | MiddleWallH | MiddleWallV => {
                vec![]
            }
            Box | Disc | Material | Repeat | Stack | Group => {
                vec![TheNodeTerminal {
                    name: str!("in"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            _ => {
                vec![]
            }
        }
    }

    pub fn outputs(
        &self,
        index: &usize,
        connections: &[(u16, u8, u16, u8)],
    ) -> Vec<TheNodeTerminal> {
        let mut highest_output_terminal: i32 = 0;
        #[allow(clippy::collapsible_if)]
        for (s, st, _, _) in connections {
            if *s as usize == *index {
                if *st as i32 + 1 > highest_output_terminal {
                    highest_output_terminal = *st as i32 + 1;
                }
            }
        }
        highest_output_terminal += 1;

        match self.role {
            Modeling => {
                let mut terminals = vec![];
                for i in 1..=highest_output_terminal {
                    terminals.push(TheNodeTerminal {
                        name: format!("face/obj #{}", i),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    });
                }
                terminals
            }
            LeftWall | BackWall | RightWall | FrontWall | MiddleWallH | MiddleWallV => {
                let mut terminals = vec![];
                for i in 1..=highest_output_terminal {
                    terminals.push(TheNodeTerminal {
                        name: format!("layer #{}", i),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    });
                }
                terminals
            }
            Box | Disc => {
                vec![TheNodeTerminal {
                    name: str!("mat"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            Repeat => {
                let mut terminals = vec![];
                for i in 1..=highest_output_terminal {
                    terminals.push(TheNodeTerminal {
                        name: format!("shape #{}", i),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    });
                }
                terminals
            }
            Stack => {
                let mut terminals = vec![];
                for i in 1..=highest_output_terminal {
                    terminals.push(TheNodeTerminal {
                        name: format!("row #{}", i),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    });
                }
                terminals
            }
            MetaMaterial => {
                vec![TheNodeTerminal {
                    name: str!("mat"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            Group => {
                let mut terminals = vec![TheNodeTerminal {
                    name: str!("cutout"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }];
                for i in 1..highest_output_terminal {
                    terminals.push(TheNodeTerminal {
                        name: format!("shape #{}", i),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    });
                }
                terminals
            }
            _ => vec![],
        }
    }

    /// Palette index has been changed. If we are a material, adjust the color.
    pub fn set_palette_index(&mut self, index: u16) -> bool {
        if self.role == GeoFXNodeRole::Material {
            self.set("Color", TheValue::PaletteIndex(index));
            true
        } else {
            false
        }
    }

    pub fn preview(
        &self,
        buffer: &mut TheRGBABuffer,
        material: Option<&MaterialFXObject>,
        _palette: &ThePalette,
        _tiles: &FxHashMap<Uuid, TheRGBATile>,
        coord: Vec2f,
        _ctx: &mut TheContext,
    ) {
        fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
            [
                (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[3] as f32 / 255.0) + b[3] as f32 / 255.0 * v) * 255.0) as u8,
            ]
        }

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height;

        //let time = TheTime::default();

        // let mut mat_obj_params: Vec<Vec<f32>> = vec![];
        // if let Some(material) = material {
        //     mat_obj_params = material.load_parameters(&time);
        // }

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as f32;
                    let y = (i / width) as f32;

                    let mut hit = Hit {
                        two_d: true,
                        ..Default::default()
                    };

                    let p = vec2f(x / width as f32, 1.0 - y / height as f32);
                    let p_coord = p + coord;
                    hit.uv = p;
                    hit.global_uv = p_coord;
                    hit.pattern_pos = p_coord;
                    hit.hit_point = vec3f(p.x + coord.x, 0.0, p.y + coord.y);
                    hit.normal = vec3f(0.0, 1.0, 0.0);
                    let d = 1.0; //self.distance(&time, p_coord, 1.0, &mut Some(&mut hit));
                    hit.distance = d;

                    // if let Some(material) = material {
                    //     material.follow_geo_trail(&TheTime::default(), &mut hit, &mat_obj_params);
                    //     if hit.interior_distance <= 0.01 {
                    //         hit.value = 0.0;
                    //     } else {
                    //         hit.value = 1.0;
                    //     }
                    //     material.compute(&mut hit, palette, tiles, &mat_obj_params);
                    // };

                    let t = smoothstep(-0.04, 0.0, d);

                    let color = if material.is_some() {
                        TheColor::from_vec3f(hit.mat.base_color).to_u8_array()
                    } else {
                        [209, 209, 209, 255]
                    };
                    pixel.copy_from_slice(&mix_color(&color, &[81, 81, 81, 255], t));
                }
            });
    }

    pub fn update_parameters(&mut self) {
        match self.role {
            LeftWall | FrontWall | RightWall | BackWall | MiddleWallH | MiddleWallV => {
                self.set(
                    "2D Mode",
                    TheValue::TextList(
                        0,
                        vec![
                            str!("Normal"),
                            str!("-1 Pos, +1 Length"),
                            str!("-1 Pos, +2 Length"),
                        ],
                    ),
                );
            }
            _ => {}
        }
    }
}
