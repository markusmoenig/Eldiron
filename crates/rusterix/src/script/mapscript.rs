use crate::script::ParseError;
use crate::{Entity, Light, Map, MapMeta, PixelSource, Texture, Tile, Value};
// use rustpython::vm;
// use rustpython::vm::*;
use std::sync::{LazyLock, RwLock};
use theframework::prelude::*;
use vek::Vec2;

#[derive(Clone, Copy)]
struct MapCursorState {
    pub position: Vec2<f32>,
    pub orientation: Vec2<f32>,

    pub last_wall: Option<u32>,
    pub last_sector: Option<u32>,
}

impl Default for MapCursorState {
    fn default() -> Self {
        Self::new()
    }
}

impl MapCursorState {
    pub fn new() -> Self {
        Self {
            position: Vec2::zero(),
            orientation: Vec2::new(1.0, 0.0),

            last_wall: None,
            last_sector: None,
        }
    }
}

static DEFAULT_WALL_TEXTURE: LazyLock<RwLock<Option<Uuid>>> = LazyLock::new(|| RwLock::new(None));
static DEFAULT_WALL_TEXTURE_ROW2: LazyLock<RwLock<Option<Uuid>>> =
    LazyLock::new(|| RwLock::new(None));
static DEFAULT_WALL_TEXTURE_ROW3: LazyLock<RwLock<Option<Uuid>>> =
    LazyLock::new(|| RwLock::new(None));
static DEFAULT_FLOOR_TEXTURE: LazyLock<RwLock<Option<Uuid>>> = LazyLock::new(|| RwLock::new(None));
static DEFAULT_CEILING_TEXTURE: LazyLock<RwLock<Option<Uuid>>> =
    LazyLock::new(|| RwLock::new(None));

static DEFAULT_WALL_HEIGHT: LazyLock<RwLock<f32>> = LazyLock::new(|| RwLock::new(2.0));
static DEFAULT_WALL_WIDTH: LazyLock<RwLock<f32>> = LazyLock::new(|| RwLock::new(0.0));

static MAP: LazyLock<RwLock<Map>> = LazyLock::new(|| RwLock::new(Map::default()));
static TEXTURES: LazyLock<RwLock<FxHashMap<String, Texture>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));
static TILES: LazyLock<RwLock<FxHashMap<Uuid, Tile>>> =
    LazyLock::new(|| RwLock::new(FxHashMap::default()));

static CURSORSTATE: LazyLock<RwLock<MapCursorState>> =
    LazyLock::new(|| RwLock::new(MapCursorState::default()));

static SAVEDSTATE: LazyLock<RwLock<MapCursorState>> =
    LazyLock::new(|| RwLock::new(MapCursorState::default()));

fn push() {
    *SAVEDSTATE.write().unwrap() = *CURSORSTATE.read().unwrap()
}

fn pop() {
    *CURSORSTATE.write().unwrap() = *SAVEDSTATE.read().unwrap()
}

/// Converts a hex color string  to an [f32; 3]
fn hex_to_rgb_f32(hex: &str) -> [f32; 3] {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return [1.0, 1.0, 1.0]; // Return white for invalid input
    }

    match (
        u8::from_str_radix(&hex[0..2], 16),
        u8::from_str_radix(&hex[2..4], 16),
        u8::from_str_radix(&hex[4..6], 16),
    ) {
        (Ok(r), Ok(g), Ok(b)) => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0],
        _ => [1.0, 1.0, 1.0], // Return white for invalid input
    }
}

fn add_entity(name: String, class_name: String, texture: String) {
    let state = CURSORSTATE.read().unwrap();

    let mut entity = Entity::default();

    entity.set_position(Vec3::new(state.position.x, 1.0, state.position.y));
    entity.set_attribute("name", Value::Str(name));
    entity.set_attribute("class_name", Value::Str(class_name));

    if let Some(id) = get_texture(&texture) {
        entity.set_attribute("tile_id", Value::Id(id));
    }

    let mut map = MAP.write().unwrap();
    map.entities.push(entity);
}

fn add_point_light(color: String, intensity: f32, start_distance: f32, end_distance: f32) {
    let state = CURSORSTATE.read().unwrap();
    let mut map = MAP.write().unwrap();

    // let light = Light::PointLight {
    //     position: Vec3::new(state.position.x, 0.5, state.position.y),
    //     color: hex_to_rgb_f32(&color),
    //     intensity,
    //     start_distance,
    //     end_distance,
    //     flicker: None,
    // };

    let mut light = Light::new(crate::LightType::Point);
    light.set_position(Vec3::new(state.position.x, 0.5, state.position.y));
    light.set_color(hex_to_rgb_f32(&color));
    light.set_intensity(intensity);
    light.set_start_distance(start_distance);
    light.set_end_distance(end_distance);

    map.lights.push(light);
}

fn set_default(key: PyObjectRef, value: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
    let key: String = String::try_from_object(vm, key)?;

    match key.as_str() {
        "floor_tex" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    *DEFAULT_FLOOR_TEXTURE.write().unwrap() = Some(id);
                    Ok(())
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'floor_texture'".to_owned()))
            }
        }
        "wall_tex" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    *DEFAULT_WALL_TEXTURE.write().unwrap() = Some(id);
                    Ok(())
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'wall_texture'".to_owned()))
            }
        }
        "wall_tex_row2" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    *DEFAULT_WALL_TEXTURE_ROW2.write().unwrap() = Some(id);
                    Ok(())
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'wall_texture'".to_owned()))
            }
        }
        "wall_tex_row3" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    *DEFAULT_WALL_TEXTURE_ROW3.write().unwrap() = Some(id);
                    Ok(())
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'wall_texture'".to_owned()))
            }
        }
        "wall_height" => {
            *DEFAULT_WALL_HEIGHT.write().unwrap() = if value.class().is(vm.ctx.types.int_type) {
                let value: i32 = value.try_into_value(vm)?;
                value as f32
            } else if value.class().is(vm.ctx.types.float_type) {
                let value: f32 = value.try_into_value(vm)?;
                value
            } else {
                0.0
            };
            Ok(())
        }
        "wall_width" => {
            *DEFAULT_WALL_WIDTH.write().unwrap() = if value.class().is(vm.ctx.types.int_type) {
                let value: i32 = value.try_into_value(vm)?;
                value as f32
            } else if value.class().is(vm.ctx.types.float_type) {
                let value: f32 = value.try_into_value(vm)?;
                value
            } else {
                0.0
            };
            Ok(())
        }
        "ceiling_tex" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    *DEFAULT_CEILING_TEXTURE.write().unwrap() = Some(id);
                    Ok(())
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'ceiling_texture'".to_owned()))
            }
        }
        _ => Err(vm.new_type_error("Unsupported value type".to_owned())),
    }
}

/// Set a value from Python.
fn set(key: PyObjectRef, value: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
    let key: String = String::try_from_object(vm, key)?;

    match key.as_str() {
        "sky_tex" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    let mut map = MAP.write().unwrap();
                    map.sky_texture = Some(id);
                    Ok(())
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'floor_texture'".to_owned()))
            }
        }
        "floor_tex" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    if let Some(sectori_id) = CURSORSTATE.read().unwrap().last_sector {
                        let mut map = MAP.write().unwrap();
                        if let Some(sector) = map.find_sector_mut(sectori_id) {
                            sector
                                .properties
                                .set("source", Value::Source(PixelSource::TileId(id)));
                        }
                        Ok(())
                    } else {
                        Err(vm.new_type_error("No sector available".to_owned()))
                    }
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'floor_texture'".to_owned()))
            }
        }
        "wall_tex" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    if let Some(wall_id) = CURSORSTATE.read().unwrap().last_wall {
                        let mut map = MAP.write().unwrap();
                        if let Some(linedef) = map.find_linedef_mut(wall_id) {
                            linedef
                                .properties
                                .set("row1_source", Value::Source(PixelSource::TileId(id)));
                        }
                        Ok(())
                    } else {
                        Err(vm.new_type_error("No wall available".to_owned()))
                    }
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'wall_texture'".to_owned()))
            }
        }
        "wall_tex_row2" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    if let Some(wall_id) = CURSORSTATE.read().unwrap().last_wall {
                        let mut map = MAP.write().unwrap();
                        if let Some(linedef) = map.find_linedef_mut(wall_id) {
                            linedef
                                .properties
                                .set("row2_source", Value::Source(PixelSource::TileId(id)));
                        }
                        Ok(())
                    } else {
                        Err(vm.new_type_error("No wall available".to_owned()))
                    }
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'wall_texture'".to_owned()))
            }
        }
        "wall_tex_row3" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    if let Some(wall_id) = CURSORSTATE.read().unwrap().last_wall {
                        let mut map = MAP.write().unwrap();
                        if let Some(linedef) = map.find_linedef_mut(wall_id) {
                            linedef
                                .properties
                                .set("row3_source", Value::Source(PixelSource::TileId(id)));
                        }
                        Ok(())
                    } else {
                        Err(vm.new_type_error("No wall available".to_owned()))
                    }
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'wall_texture'".to_owned()))
            }
        }
        "wall_height" => {
            if let Some(wall_id) = CURSORSTATE.read().unwrap().last_wall {
                let mut map = MAP.write().unwrap();
                if let Some(linedef) = map.find_linedef_mut(wall_id) {
                    let height = if value.class().is(vm.ctx.types.int_type) {
                        let value: i32 = value.try_into_value(vm)?;
                        value as f32
                    } else if value.class().is(vm.ctx.types.float_type) {
                        let value: f32 = value.try_into_value(vm)?;
                        value
                    } else {
                        0.0
                    };
                    linedef.properties.set("wall_height", Value::Float(height));
                }
                Ok(())
            } else {
                Err(vm.new_type_error("No wall available".to_owned()))
            }
        }
        "wall_width" => {
            if let Some(wall_id) = CURSORSTATE.read().unwrap().last_wall {
                let mut map = MAP.write().unwrap();
                if let Some(linedef) = map.find_linedef_mut(wall_id) {
                    let height = if value.class().is(vm.ctx.types.int_type) {
                        let value: i32 = value.try_into_value(vm)?;
                        value as f32
                    } else if value.class().is(vm.ctx.types.float_type) {
                        let value: f32 = value.try_into_value(vm)?;
                        value
                    } else {
                        0.0
                    };
                    linedef.properties.set("wall_width", Value::Float(height));
                }
                Ok(())
            } else {
                Err(vm.new_type_error("No wall available".to_owned()))
            }
        }
        "ceiling_tex" => {
            if let Ok(val) = String::try_from_object(vm, value) {
                if let Some(id) = get_texture(&val) {
                    if let Some(sectori_id) = CURSORSTATE.read().unwrap().last_sector {
                        let mut map = MAP.write().unwrap();
                        if let Some(sector) = map.find_sector_mut(sectori_id) {
                            sector
                                .properties
                                .set("ceiling_source", Value::Source(PixelSource::TileId(id)));
                        }
                        Ok(())
                    } else {
                        Err(vm.new_type_error("No sector available".to_owned()))
                    }
                } else {
                    Err(vm.new_type_error(format!("Could not fnd texture {}", val).to_owned()))
                }
            } else {
                Err(vm.new_type_error("Unsupported value type for 'ceiling_texture'".to_owned()))
            }
        }
        _ => Err(vm.new_type_error("Unsupported value type".to_owned())),
    }
}

/// Set a default value from Python.
fn wall(value: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
    let length = if value.class().is(vm.ctx.types.int_type) {
        let value: i32 = value.try_into_value(vm)?;
        value as f32
    } else if value.class().is(vm.ctx.types.float_type) {
        let value: f32 = value.try_into_value(vm)?;
        value
    } else {
        0.0
    };

    let mut map = MAP.write().unwrap();
    let mut state = CURSORSTATE.write().unwrap();

    let orientation = state.orientation;

    // Calculate the "to" position based on the current orientation
    let to = state.position + orientation * length;

    // Add vertices to the map
    let from_index = map.add_vertex_at(state.position.x, state.position.y);
    let to_index = map.add_vertex_at(to.x, to.y);

    // Create the linedef
    let (linedef_id, sector_id) = map.create_linedef(from_index, to_index);

    if let Some(linedef) = map.find_linedef_mut(linedef_id) {
        linedef.properties.set(
            "row1_source",
            Value::Source(if let Some(id) = *DEFAULT_WALL_TEXTURE.read().unwrap() {
                PixelSource::TileId(id)
            } else {
                PixelSource::Off
            }),
        );
        linedef.properties.set(
            "row2_source",
            Value::Source(
                if let Some(id) = *DEFAULT_WALL_TEXTURE_ROW2.read().unwrap() {
                    PixelSource::TileId(id)
                } else {
                    PixelSource::Off
                },
            ),
        );
        linedef.properties.set(
            "row3_source",
            Value::Source(
                if let Some(id) = *DEFAULT_WALL_TEXTURE_ROW3.read().unwrap() {
                    PixelSource::TileId(id)
                } else {
                    PixelSource::Off
                },
            ),
        );
        linedef.properties.set(
            "wall_height",
            Value::Float(*DEFAULT_WALL_HEIGHT.read().unwrap()),
        );
        state.last_wall = Some(linedef.id);
    }

    if let Some(sector_id) = sector_id {
        if let Some(sector) = map.find_sector_mut(sector_id) {
            sector.properties.set(
                "source",
                Value::Source(if let Some(id) = *DEFAULT_FLOOR_TEXTURE.read().unwrap() {
                    PixelSource::TileId(id)
                } else {
                    PixelSource::Off
                }),
            );
            sector.properties.set(
                "ceiling_source",
                Value::Source(if let Some(id) = *DEFAULT_CEILING_TEXTURE.read().unwrap() {
                    PixelSource::TileId(id)
                } else {
                    PixelSource::Off
                }),
            );
            /*
            sector.properties.set(
                "row1_source",
                Value::Source(if let Some(id) = *DEFAULT_WALL_TEXTURE.read().unwrap() {
                    PixelSource::TileId(id)
                } else {
                    PixelSource::Off
                }),
            );
            sector.properties.set(
                "row2_source",
                Value::Source(
                    if let Some(id) = *DEFAULT_WALL_TEXTURE_ROW2.read().unwrap() {
                        PixelSource::TileId(id)
                    } else {
                        PixelSource::Off
                    },
                ),
            );
            sector.properties.set(
                "row3_source",
                Value::Source(
                    if let Some(id) = *DEFAULT_WALL_TEXTURE_ROW3.read().unwrap() {
                        PixelSource::TileId(id)
                    } else {
                        PixelSource::Off
                    },
                ),
            );*/
        }
        state.last_sector = Some(sector_id);
    }

    // Update the current position
    state.position = to;

    Ok(())
}

/// Gets or add the texture of the given name and returns its id
fn get_texture(texture_name: &str) -> Option<Uuid> {
    let mut tiles = TILES.write().unwrap();
    let textures = TEXTURES.read().unwrap();

    if let Some(tex) = textures.get(texture_name) {
        let tile = Tile::from_texture(tex.clone());
        let id = tile.id;
        tiles.insert(id, tile);
        Some(id)
    } else {
        None
    }

    /*
    if let Some(id) = tiles
        .iter()
        .find(|(_, tile)| tile.name == texture_name)
        .map(|(uuid, _)| *uuid)
    {
        Some(id)
    } else if let Some(tex) = load_texture(texture_name, PATH.read().unwrap().clone()) {
        let tile = Tile::from_texture(texture_name, tex);
        let id = tile.id;

        tiles.insert(id, tile);

        Some(id)
    } else {
        None
    }*/
}

fn move_forward(length: f32) -> PyResult<()> {
    let mut state = CURSORSTATE.write().unwrap();
    let orientation = state.orientation;

    // Update the position based on the current orientation
    state.position += orientation * length;

    Ok(())
}

fn move_to(x: PyObjectRef, y: PyObjectRef, vm: &VirtualMachine) -> PyResult<()> {
    let x: f32 = if x.class().is(vm.ctx.types.int_type) {
        x.try_into_value::<i32>(vm)? as f32
    } else if x.class().is(vm.ctx.types.float_type) {
        x.try_into_value::<f32>(vm)?
    } else {
        return Err(vm.new_type_error("Expected an integer or float for x".to_owned()));
    };

    let y: f32 = if y.class().is(vm.ctx.types.int_type) {
        y.try_into_value::<i32>(vm)? as f32
    } else if y.class().is(vm.ctx.types.float_type) {
        y.try_into_value::<f32>(vm)?
    } else {
        return Err(vm.new_type_error("Expected an integer or float for y".to_owned()));
    };

    let mut state = CURSORSTATE.write().unwrap();
    state.position = Vec2::new(x, y);

    Ok(())
}

fn rotate(angle: f32) -> PyResult<()> {
    let mut state = CURSORSTATE.write().unwrap();
    let orientation = state.orientation;

    // Calculate the new orientation by rotating the vector
    let radians = angle.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();

    let new_orientation = Vec2::new(
        orientation.x * cos - orientation.y * sin,
        orientation.x * sin + orientation.y * cos,
    );

    state.orientation = new_orientation;

    Ok(())
}

fn turn_left() -> PyResult<()> {
    rotate(-90.0)
}

fn turn_right() -> PyResult<()> {
    rotate(90.0)
}

pub struct MapScript {
    error: Option<ParseError>,
}

impl Default for MapScript {
    fn default() -> Self {
        MapScript::new()
    }
}

impl MapScript {
    pub fn new() -> Self {
        Self { error: None }
    }

    /// Parse the source and return the new or compiled map.
    pub fn compile(
        &mut self,
        source: &str,
        textures: &FxHashMap<String, Texture>,
        ctx_map: Option<Map>,
        ctx_linedef: Option<u32>,
        ctx_sector: Option<u32>,
    ) -> Result<MapMeta, Vec<String>> {
        self.error = None;
        *MAP.write().unwrap() = ctx_map.unwrap_or_default();
        *TILES.write().unwrap() = FxHashMap::default();
        *TEXTURES.write().unwrap() = textures.clone();
        *DEFAULT_WALL_TEXTURE.write().unwrap() = None;
        *DEFAULT_WALL_TEXTURE_ROW2.write().unwrap() = None;
        *DEFAULT_WALL_TEXTURE_ROW3.write().unwrap() = None;
        *DEFAULT_CEILING_TEXTURE.write().unwrap() = None;
        *DEFAULT_FLOOR_TEXTURE.write().unwrap() = None;

        let state = MapCursorState {
            last_wall: ctx_linedef,
            last_sector: ctx_sector,
            ..Default::default()
        };

        *CURSORSTATE.write().unwrap() = state;
        *SAVEDSTATE.write().unwrap() = state;

        let interpreter = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();

        interpreter.enter(|vm| {
            let scope = vm.new_scope_with_builtins();

            let _ = scope.globals.set_item(
                "add_entity",
                vm.new_function("add_entity", add_entity).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "add_point_light",
                vm.new_function("add_point_light", add_point_light).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("push", vm.new_function("push", push).into(), vm);

            let _ = scope
                .globals
                .set_item("pop", vm.new_function("pop", pop).into(), vm);

            let _ = scope.globals.set_item(
                "set_default",
                vm.new_function("set_default", set_default).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("set", vm.new_function("set", set).into(), vm);

            let _ = scope
                .globals
                .set_item("wall", vm.new_function("wall", wall).into(), vm);

            let _ = scope.globals.set_item(
                "move_forward",
                vm.new_function("turn_left", move_forward).into(),
                vm,
            );

            let _ =
                scope
                    .globals
                    .set_item("move_to", vm.new_function("move_to", move_to).into(), vm);

            let _ = scope.globals.set_item(
                "turn_left",
                vm.new_function("turn_left", turn_left).into(),
                vm,
            );

            let _ = scope.globals.set_item(
                "turn_right",
                vm.new_function("turn_right", turn_right).into(),
                vm,
            );

            let _ = scope
                .globals
                .set_item("rotate", vm.new_function("rotate", rotate).into(), vm);

            if let Ok(code_obj) = vm
                .compile(source, vm::compiler::Mode::Exec, "<embedded>".to_owned())
                .map_err(|err| vm.new_syntax_error(&err, Some(source)))
            {
                if let Err(err) = vm.run_code_obj(code_obj, scope) {
                    let args = err.args();

                    let mut errors: Vec<String> = vec![];
                    for error in args.iter() {
                        if let Ok(msg) = error.str(vm) {
                            errors.push(msg.to_string());
                        }
                    }

                    return Err(errors);
                }
            }

            let meta = MapMeta::new(MAP.read().unwrap().clone(), TILES.read().unwrap().clone());
            Ok(meta)
        })
    }
}
