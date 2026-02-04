use crate::textures::patterns::{pattern_normal_safe, pattern_safe};
use crate::{NodeOp, Program, TexStorage, Value};
use std::path::PathBuf;
use theframework::thepalette::ThePalette;
use vek::Vec3;

#[derive(Clone)]
pub struct Execution {
    /// Global variables. The parser keeps count of all global variables and we allocate the array on creation.
    pub globals: Vec<Value>,

    /// Local variables used inside function bodies.
    locals: Vec<Value>,

    /// The locals state for recursive functions
    locals_stack: Vec<Vec<Value>>,

    /// The execution stack.
    pub stack: Vec<Value>,

    /// Function return value.
    return_value: Option<Value>,

    /// Allocated textures.
    textures: Vec<TexStorage>,

    /// UV
    pub uv: Value,

    /// Input/Output color
    pub color: Value,

    /// Roughness
    pub roughness: Value,

    /// Metallic
    pub metallic: Value,

    /// Emissive
    pub emissive: Value,

    /// Opacity
    pub opacity: Value,

    /// Bump
    pub bump: Value,

    /// Normal
    pub normal: Value,

    /// Hitpoint
    pub hitpoint: Value,

    /// Time
    pub time: Value,
}

impl Execution {
    pub fn new(var_size: usize) -> Self {
        Self {
            globals: vec![Value::zero(); var_size],
            locals: vec![],
            locals_stack: vec![],
            stack: Vec::with_capacity(32),
            return_value: None,
            textures: vec![],
            uv: Vec3::zero(),
            color: Vec3::zero(),
            roughness: Vec3::broadcast(0.5),
            metallic: Vec3::zero(),
            emissive: Vec3::zero(),
            opacity: Vec3::zero(),
            bump: Vec3::zero(),
            normal: Vec3::zero(),
            hitpoint: Vec3::zero(),
            time: Vec3::zero(),
        }
    }

    pub fn new_from_var(execution: &Execution) -> Self {
        Self {
            globals: execution.globals.clone(),
            locals: vec![],
            locals_stack: vec![],
            stack: Vec::with_capacity(32),
            return_value: None,
            textures: vec![],
            uv: Vec3::zero(),
            color: Vec3::zero(),
            roughness: Vec3::broadcast(0.5),
            metallic: Vec3::zero(),
            emissive: Vec3::zero(),
            opacity: Vec3::zero(),
            bump: Vec3::zero(),
            normal: Vec3::zero(),
            hitpoint: Vec3::zero(),
            time: Vec3::zero(),
        }
    }

    /// When switching between programs we need to resize the count of global variables.
    #[inline]
    pub fn reset(&mut self, var_size: usize) {
        if var_size != self.globals.len() {
            self.globals.resize(var_size, Value::zero());
        }
    }

    pub fn execute(&mut self, code: &[NodeOp], program: &Program, palette: &ThePalette) {
        for op in code {
            // Unwind if return is set
            if self.return_value.is_some() {
                break;
            }
            match op {
                NodeOp::LoadGlobal(index) => {
                    self.stack.push(self.globals[*index]);
                }
                NodeOp::StoreGlobal(index) => {
                    self.globals[*index] = self.stack.pop().unwrap();
                }
                NodeOp::LoadLocal(index) => {
                    self.stack.push(self.locals[*index]);
                }
                NodeOp::StoreLocal(index) => {
                    self.locals[*index] = self.stack.pop().unwrap();
                }
                NodeOp::Swap => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(b);
                    self.stack.push(a);
                }
                NodeOp::GetComponents(swizzle) => {
                    let v = self.stack.pop().unwrap();
                    let mut result = vec![];

                    for &index in swizzle {
                        let f = match index {
                            0 => v.x,
                            1 => v.y,
                            2 => v.z,
                            _ => continue,
                        };
                        result.push(f);
                    }

                    // Push as single Value: scalar or vector depending on result length
                    let pushed = match result.as_slice() {
                        [x] => Value::broadcast(*x),
                        [x, y] => Value::new(*x, *y, 0.0),
                        [x, y, z] => Value::new(*x, *y, *z),
                        _ => Value::broadcast(0.0),
                    };

                    self.stack.push(pushed);
                }
                NodeOp::SetComponents(swizzle) => {
                    let value = self.stack.pop().unwrap();
                    let mut target = self.stack.pop().unwrap();

                    let components = match swizzle.len() {
                        1 => vec![value.x],
                        2 => vec![value.x, value.y],
                        3 => vec![value.x, value.y, value.z],
                        _ => vec![],
                    };

                    for (i, &idx) in swizzle.iter().enumerate() {
                        if i >= components.len() {
                            break;
                        }
                        match idx {
                            0 => target.x = components[i],
                            1 => target.y = components[i],
                            2 => target.z = components[i],
                            _ => {}
                        }
                    }

                    self.stack.push(target);
                }
                NodeOp::Push(v) => self.stack.push(*v),
                NodeOp::Clear => _ = self.stack.pop(),
                NodeOp::FunctionCall(arity, total_locals, index) => {
                    self.push_locals_state();
                    self.locals = vec![Value::zero(); *total_locals as usize];

                    // Arguments are on stack in call order
                    for index in (0..*arity as usize).rev() {
                        if let Some(arg) = self.stack.pop() {
                            self.locals[index] = arg;
                        }
                    }

                    // Save the stack position
                    let stack_base = self.stack.len();

                    // Execute the function body
                    let body = program.user_functions[*index].clone(); // Arc clone
                    self.execute(&body, program, palette);

                    // Retrieve the return value. A function always returns exactly one value.
                    let ret = if self.return_value.is_some() {
                        self.return_value.take().unwrap_or(Value::zero())
                    } else if self.stack.len() > stack_base {
                        self.stack.pop().unwrap()
                    } else {
                        Value::zero()
                    };

                    // Clean up temporaries
                    self.stack.truncate(stack_base);

                    self.pop_locals_state();

                    // Push the return value
                    self.stack.push(ret);
                }
                NodeOp::Return => {
                    let v = if let Some(top) = self.stack.pop() {
                        top
                    } else if let Some(prev) = self.return_value.take() {
                        prev
                    } else {
                        Value::zero()
                    };
                    self.return_value = Some(v);
                    break;
                }
                NodeOp::Pack2 => {
                    let y = self.stack.pop().unwrap();
                    let x = self.stack.pop().unwrap();
                    self.stack.push(Value::new(x.x, y.x, 0.0));
                }
                NodeOp::Pack3 => {
                    let z = self.stack.pop().unwrap();
                    let y = self.stack.pop().unwrap();
                    let x = self.stack.pop().unwrap();
                    self.stack.push(Value::new(x.x, y.x, z.x));
                }
                NodeOp::Dup => {
                    if let Some(top) = self.stack.last() {
                        self.stack.push(*top);
                    }
                }
                NodeOp::For(init, cond, incr, body) => {
                    let base = self.stack.len();
                    let mut iter = 0usize;
                    self.execute(init, program, palette);
                    self.stack.truncate(base);

                    loop {
                        self.execute(cond, program, palette);

                        let z = self.stack.pop().unwrap();
                        if z.x == 0.0 {
                            break;
                        }
                        self.stack.truncate(base);

                        self.execute(body, program, palette);
                        self.stack.truncate(base);

                        self.execute(incr, program, palette);
                        self.stack.truncate(base);

                        iter += 1;
                        if iter > 10_000_000 {
                            panic!("Inifinite for loop detected");
                        }
                    }
                }
                NodeOp::If(then_code, else_code) => {
                    let value = self.stack.pop().unwrap().x != 0.0;
                    if value {
                        self.execute(then_code, program, palette);
                    } else if let Some(else_code) = else_code {
                        self.execute(else_code, program, palette);
                    }
                }
                // Math
                NodeOp::Add => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a + b);
                }
                NodeOp::Sub => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a - b);
                }
                NodeOp::Mul => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a * b);
                }
                NodeOp::Div => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a / b);
                }
                NodeOp::Length => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::broadcast(a.magnitude()));
                }
                NodeOp::Length2 => {
                    let a = self.stack.pop().unwrap();
                    let len2 = (a.x * a.x + a.y * a.y).sqrt();
                    self.stack.push(Value::new(len2, 0.0, 0.0));
                }
                NodeOp::Length3 => {
                    let a = self.stack.pop().unwrap();
                    let len3 = (a.x * a.x + a.y * a.y + a.z * a.z).sqrt();
                    self.stack.push(Value::new(len3, 0.0, 0.0));
                }
                NodeOp::Abs => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.abs()));
                }
                NodeOp::Sin => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.sin()));
                }
                NodeOp::Sin1 => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::new(a.x.sin(), 0.0, 0.0));
                }
                NodeOp::Sin2 => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::new(a.x.sin(), a.y.sin(), 0.0));
                }
                NodeOp::Cos => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.cos()));
                }
                NodeOp::Cos1 => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::new(a.x.sin(), 0.0, 0.0));
                }
                NodeOp::Cos2 => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::new(a.x.sin(), a.y.sin(), 0.0));
                }
                NodeOp::Normalize => {
                    let a = self.stack.pop().unwrap();
                    let len = a.magnitude();
                    self.stack.push(if len > 0.0 {
                        a / Value::broadcast(len)
                    } else {
                        a
                    });
                }
                NodeOp::Tan => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.tan()));
                }
                NodeOp::Atan => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.atan()));
                }
                NodeOp::Atan2 => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map2(b, |x, y| x.atan2(y)));
                }
                NodeOp::Rotate2D => {
                    let angle = self.stack.pop().unwrap();
                    let v = self.stack.pop().unwrap();
                    let rad = angle.x.to_radians();
                    let (s, c) = rad.sin_cos();
                    self.stack
                        .push(Value::new(v.x * c - v.y * s, v.x * s + v.y * c, v.z));
                }
                NodeOp::Dot => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::broadcast(a.dot(b)));
                }
                NodeOp::Dot2 => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    let dot2 = a.x * b.x + a.y * b.y;
                    self.stack.push(Value::new(dot2, 0.0, 0.0));
                }
                NodeOp::Dot3 => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    let dot3 = a.x * b.x + a.y * b.y + a.z * b.z;
                    self.stack.push(Value::new(dot3, 0.0, 0.0));
                }
                NodeOp::Cross => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.cross(b));
                }
                NodeOp::Floor => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.floor()));
                }
                NodeOp::Ceil => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.ceil()));
                }
                NodeOp::Round => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.round()));
                }
                NodeOp::Fract => {
                    let a = self.stack.pop().unwrap();
                    // GLSL fract: x - floor(x) (Rust's .fract() == x - trunc(x), which breaks for negatives)
                    // self.stack.push(a.map(|x| x.fract()));
                    let f = a.map(|x| x - x.floor());
                    self.stack.push(f);
                }
                /*
                Rust style
                NodeOp::Mod => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::new(a.x % b.x, a.y % b.y, a.z % b.z));
                }*/
                NodeOp::Mod => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    let rx = a.x - b.x * (a.x / b.x).floor();
                    let ry = a.y - b.y * (a.y / b.y).floor();
                    let rz = a.z - b.z * (a.z / b.z).floor();
                    self.stack.push(Value::new(rx, ry, rz));
                }
                NodeOp::Radians => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.to_radians()));
                }
                NodeOp::Degrees => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.to_degrees()));
                }
                NodeOp::Min => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)));
                }
                NodeOp::Max => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)));
                }
                NodeOp::Mix => {
                    let c: Value = self.stack.pop().unwrap(); // t
                    let b: Value = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    // mix(a,b,t) = a*(1-t) + b*t, all component-wise
                    self.stack.push(a + (b - a) * c);
                }
                NodeOp::Smoothstep => {
                    let c: Value = self.stack.pop().unwrap(); // x
                    let b: Value = self.stack.pop().unwrap(); // edge1
                    let a = self.stack.pop().unwrap(); // edge0

                    let denom = b.x - a.x;
                    let mut t = if denom != 0.0 {
                        (c.x - a.x) / denom
                    } else {
                        0.0
                    };
                    if t < 0.0 {
                        t = 0.0;
                    } else if t > 1.0 {
                        t = 1.0;
                    }
                    let s = t * t * (3.0 - 2.0 * t);
                    self.stack.push(Value::broadcast(s));
                }
                NodeOp::Step => {
                    // step(edge, x): returns 0.0 if x < edge else 1.0 (per component)
                    let b: Value = self.stack.pop().unwrap(); // x
                    let a = self.stack.pop().unwrap(); // edge
                    self.stack.push(Value::new(
                        if b.x >= a.x { 1.0 } else { 0.0 },
                        if b.y >= a.y { 1.0 } else { 0.0 },
                        if b.z >= a.z { 1.0 } else { 0.0 },
                    ));
                }
                NodeOp::Clamp => {
                    let c: Value = self.stack.pop().unwrap(); // hi
                    let b: Value = self.stack.pop().unwrap(); // lo
                    let a = self.stack.pop().unwrap(); // x
                    self.stack.push(Value::new(
                        a.x.clamp(b.x, c.x),
                        a.y.clamp(b.y, c.y),
                        a.z.clamp(b.z, c.z),
                    ));
                }
                NodeOp::Sqrt => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.sqrt()));
                }
                NodeOp::Log => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.ln()));
                }
                NodeOp::Pow => {
                    let b: Value = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new(a.x.powf(b.x), a.y.powf(b.y), a.z.powf(b.z)));
                }
                // Comparison (booleans encoded as splat(1.0) / splat(0.0), using .x lane)
                NodeOp::Eq => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x == b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Ne => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x != b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Lt => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x < b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Le => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x <= b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Gt => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x > b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Ge => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x >= b.x { 1.0 } else { 0.0 }));
                }
                // Logical (use .x lane)
                NodeOp::And => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::broadcast(
                        ((a.x != 0.0) & (b.x != 0.0)) as i32 as f32,
                    ));
                }
                NodeOp::Or => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::broadcast(
                        ((a.x != 0.0) | (b.x != 0.0)) as i32 as f32,
                    ));
                }
                // Unary
                NodeOp::Not => {
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast((a.x == 0.0) as i32 as f32));
                }
                NodeOp::Neg => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(-a);
                }
                NodeOp::Print => {
                    let a = self.stack.pop().unwrap();
                    println!("print: {:?}", a);
                }
                NodeOp::UV => {
                    self.stack.push(self.uv);
                }
                NodeOp::SetUV => {
                    self.uv = self.stack.pop().unwrap();
                }
                NodeOp::Normal => {
                    self.stack.push(self.normal);
                }
                NodeOp::SetNormal => {
                    self.normal = self.stack.pop().unwrap().normalized();
                }
                NodeOp::Hitpoint => {
                    self.stack.push(self.hitpoint);
                }
                NodeOp::Time => {
                    self.stack.push(self.time);
                }
                NodeOp::Color => {
                    self.stack.push(self.color);
                }
                NodeOp::SetColor => {
                    self.color = self.stack.pop().unwrap();
                }
                NodeOp::Roughness => {
                    self.stack.push(self.roughness);
                }
                NodeOp::SetRoughness => {
                    self.roughness = self.stack.pop().unwrap();
                }
                NodeOp::Metallic => {
                    self.stack.push(self.metallic);
                }
                NodeOp::SetMetallic => {
                    self.metallic = self.stack.pop().unwrap();
                }
                NodeOp::Emissive => {
                    self.stack.push(self.emissive);
                }
                NodeOp::SetEmissive => {
                    self.emissive = self.stack.pop().unwrap();
                }
                NodeOp::Opacity => {
                    self.stack.push(self.opacity);
                }
                NodeOp::SetOpacity => {
                    self.opacity = self.stack.pop().unwrap();
                }
                NodeOp::Bump => {
                    self.stack.push(self.bump);
                }
                NodeOp::SetBump => {
                    self.bump = self.stack.pop().unwrap();
                }
                NodeOp::Sample => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    if let Some(tex) = pattern_safe(b.x as usize) {
                        self.stack.push(tex.sample(a));
                    } else {
                        self.stack.push(Vec3::zero());
                    }
                }
                NodeOp::SampleNormal => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    if let Some(tex) = pattern_normal_safe(b.x as usize) {
                        let nm = tex.sample(a);
                        let nmp = nm * 2.0 - 1.0;

                        // let strength = 1.0; // tweak 0.2..2.0
                        //self.normal = (self.normal + nmp * strength).normalized();

                        // self.stack.push(self.normal);
                        self.stack.push(nmp);
                    } else {
                        self.stack.push(Vec3::zero());
                    }
                }
                NodeOp::Alloc => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    let index = self.textures.len();
                    let tex = TexStorage::new(a.x as usize, b.x as usize);
                    self.textures.push(tex);
                    self.stack.push(Vec3::broadcast(index as f32));
                }
                NodeOp::Iterate => {
                    let b = self.stack.pop().unwrap(); // string index
                    let a = self.stack.pop().unwrap(); // texture index
                    if let Some(tex) = self.textures.get_mut(a.x as usize) {
                        if let Some(p) = program.strings.get(b.x as usize) {
                            if let Some(fn_index) = program.user_functions_name_map.get(p) {
                                let fidx = *fn_index;
                                tex.par_iterate_with(
                                    || {
                                        // Per-row execution context cloned from current globals
                                        let mut ex = Execution::new(0);
                                        // Carry over shared context fields
                                        ex.time = self.time;
                                        ex.normal = self.normal;
                                        ex.hitpoint = self.hitpoint;
                                        ex.color = self.color;
                                        ex.roughness = self.roughness;
                                        ex.metallic = self.metallic;
                                        ex.emissive = self.emissive;
                                        ex.opacity = self.opacity;
                                        ex.bump = self.bump;
                                        ex
                                    },
                                    |state, _x, _y, uv| {
                                        // Prepare state for this pixel
                                        state.stack.truncate(0);
                                        state.return_value = None;
                                        state.uv = uv;

                                        // Todo: Get the exact number of locals for this fn
                                        // Currently we do not store every local variable count in program
                                        state.locals.resize(20, Value::zero());

                                        // Execute function body
                                        state.execute(
                                            &program.user_functions[fidx],
                                            program,
                                            palette,
                                        );

                                        // Prefer explicit return value; else use color
                                        if let Some(ret) = state.return_value.take() {
                                            ret
                                        } else {
                                            state.color
                                        }
                                    },
                                );
                            }
                        }
                    }
                }
                NodeOp::Save => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    if let Some(tex) = self.textures.get(a.x as usize) {
                        if let Some(p) = program.strings.get(b.x as usize) {
                            if let Err(err) = tex.save_png(&PathBuf::from(p)) {
                                println!("{}", err.to_string());
                            }
                            let normal = tex.to_normal_map(5.0);
                            let orig = PathBuf::from(p);
                            let mut normal_path = orig.clone();
                            let stem = orig
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("texture");
                            if let Some(ext) = orig.extension().and_then(|e| e.to_str()) {
                                normal_path.set_file_name(format!("{}_normal.{}", stem, ext));
                            } else {
                                normal_path.set_file_name(format!("{}_normal", stem));
                            }
                            if let Err(err) = normal.save_png(&normal_path) {
                                println!("{}", err.to_string());
                            }
                        }
                    }
                }
                NodeOp::PaletteIndex => {
                    let a = self.stack.pop().unwrap();
                    if let Some(col) = palette.colors.get(a.x as usize) {
                        if let Some(col) = col {
                            self.stack.push(col.to_vec3());
                        }
                    }
                }
            }
        }
    }

    // Push the current locals state when we enter a function.
    fn push_locals_state(&mut self) {
        self.locals_stack.push(self.locals.clone());
    }

    // Pop the last locals state when we exit a function.
    fn pop_locals_state(&mut self) {
        if let Some(state) = self.locals_stack.pop() {
            self.locals = state;
        }
    }

    /// Execute the shading function
    #[inline]
    pub fn shade(&mut self, index: usize, program: &Program, palette: &ThePalette) {
        // Reset state for this call
        self.stack.truncate(0);
        self.return_value = None;

        self.locals.resize(program.shade_locals, Value::zero());
        self.execute(&program.user_functions[index], program, palette);
    }

    /// Call a function with no arguments
    #[inline]
    pub fn execute_function_no_args(
        &mut self,
        index: usize,
        program: &Program,
        palette: &ThePalette,
    ) -> Value {
        // Reset state for this call
        self.stack.truncate(0);
        self.return_value = None;

        self.locals.resize(program.shade_locals, Value::zero());
        self.execute(&program.user_functions[index], program, palette);

        // Prefer an explicit return value; else top of stack; else zero
        if let Some(ret) = self.return_value.take() {
            return ret;
        }
        if let Some(rc) = self.stack.pop() {
            rc
        } else {
            Value::zero()
        }
    }

    /// Call a function with arguments provided as a slice.
    #[inline]
    pub fn execute_function(
        &mut self,
        args: &[Value],
        index: usize,
        program: &Program,
        palette: &ThePalette,
    ) -> Value {
        // Reset state for this call
        self.stack.truncate(0);
        self.return_value = None;

        // Prepare locals without reallocating each time
        let argc = args.len();
        if self.locals.len() < argc {
            self.locals.resize(argc, Value::zero());
        }
        // Copy args into locals in order (0..argc)
        self.locals[..argc].clone_from_slice(args);

        self.execute(&program.user_functions[index], program, palette);

        // Prefer an explicit return value; else top of stack; else zero
        if let Some(ret) = self.return_value.take() {
            return ret;
        }
        if let Some(rc) = self.stack.pop() {
            rc
        } else {
            Value::zero()
        }
    }
}
