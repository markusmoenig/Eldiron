use super::hosthandler::HostHandler;
use crate::vm::{NodeOp, Program, VMValue};
use rustc_hash::FxHashMap;

pub struct Execution {
    /// Global variables. The parser keeps count of all global variables and we allocate the array on creation.
    pub globals: Vec<VMValue>,

    /// Local variables used inside function bodies.
    locals: Vec<VMValue>,

    /// The locals state for recursive functions
    locals_stack: Vec<Vec<VMValue>>,

    /// The execution stack.
    pub stack: Vec<VMValue>,

    /// Function return VMValue.
    return_value: Option<VMValue>,

    /// Time
    pub time: VMValue,

    /// Custom outputs set by special ops (legacy; prefer HostHandler)
    pub outputs: FxHashMap<String, VMValue>,
}

impl Execution {
    pub fn new(var_size: usize) -> Self {
        Self {
            globals: vec![VMValue::zero(); var_size],
            locals: vec![],
            locals_stack: vec![],
            stack: Vec::with_capacity(32),
            return_value: None,
            time: VMValue::zero(),
            outputs: FxHashMap::default(),
        }
    }

    pub fn default() -> Self {
        Self::new(0)
    }

    pub fn new_from_var(execution: &Execution) -> Self {
        Self {
            globals: execution.globals.clone(),
            locals: vec![],
            locals_stack: vec![],
            stack: Vec::with_capacity(32),
            return_value: None,
            time: VMValue::zero(),
            outputs: FxHashMap::default(),
        }
    }

    /// When switching between programs we need to resize the count of global variables.
    #[inline]
    pub fn reset(&mut self, var_size: usize) {
        if var_size != self.globals.len() {
            self.globals.resize(var_size, VMValue::zero());
        }
        self.outputs.clear();
    }

    #[inline(always)]
    pub fn execute_op(&mut self, op: &NodeOp, program: &Program) {
        match op {
            NodeOp::LoadGlobal(index) => {
                self.stack.push(self.globals[*index].clone());
            }
            NodeOp::StoreGlobal(index) => {
                self.globals[*index] = self.stack.pop().unwrap();
            }
            NodeOp::LoadLocal(index) => {
                self.stack.push(self.locals[*index].clone());
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

                // Push as single VMValue: scalar or vector depending on result length
                let pushed = match result.as_slice() {
                    [x] => VMValue::broadcast(*x),
                    [x, y] => VMValue::new(*x, *y, 0.0),
                    [x, y, z] => VMValue::new(*x, *y, *z),
                    _ => VMValue::broadcast(0.0),
                };

                self.stack.push(pushed);
            }
            NodeOp::SetComponents(swizzle) => {
                let value = self.stack.pop().unwrap();
                let mut target = self.stack.pop().unwrap();
                target.string = None;

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
            NodeOp::Push(v) => self.stack.push(v.clone()),
            NodeOp::Clear => _ = self.stack.pop(),
            NodeOp::FunctionCall(arity, total_locals, index) => {
                self.push_locals_state();
                self.locals = vec![VMValue::zero(); *total_locals as usize];

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
                self.execute(&body, program);

                // Retrieve the return VMValue. A function always returns exactly one VMValue.
                let ret = if self.return_value.is_some() {
                    self.return_value.take().unwrap_or(VMValue::zero())
                } else if self.stack.len() > stack_base {
                    self.stack.pop().unwrap()
                } else {
                    VMValue::zero()
                };

                // Clean up temporaries
                self.stack.truncate(stack_base);

                self.pop_locals_state();

                // Push the return VMValue
                self.stack.push(ret);
            }
            NodeOp::Return => {
                let v = if let Some(top) = self.stack.pop() {
                    top
                } else if let Some(prev) = self.return_value.take() {
                    prev
                } else {
                    VMValue::zero()
                };
                self.return_value = Some(v);
            }
            NodeOp::Pack2 => {
                let y = self.stack.pop().unwrap();
                let x = self.stack.pop().unwrap();
                self.stack
                    .push(VMValue::new_with_string(x.x, y.x, 0.0, "vec2"));
            }
            NodeOp::Pack3 => {
                let z = self.stack.pop().unwrap();
                let y = self.stack.pop().unwrap();
                let x = self.stack.pop().unwrap();
                self.stack
                    .push(VMValue::new_with_string(x.x, y.x, z.x, "vec3"));
            }
            NodeOp::Dup => {
                if let Some(top) = self.stack.last() {
                    self.stack.push(top.clone());
                }
            }
            NodeOp::For(init, cond, incr, body) => {
                let base = self.stack.len();
                let mut iter = 0usize;
                self.execute(init, program);
                self.stack.truncate(base);

                loop {
                    self.execute(cond, program);

                    let z = self.stack.pop().unwrap();
                    if !z.is_truthy() {
                        break;
                    }
                    self.stack.truncate(base);

                    self.execute(body, program);
                    self.stack.truncate(base);

                    self.execute(incr, program);
                    self.stack.truncate(base);

                    iter += 1;
                    if iter > 10_000_000 {
                        panic!("Inifinite for loop detected");
                    }
                }
            }
            NodeOp::If(then_code, else_code) => {
                let value = self.stack.pop().unwrap().is_truthy();
                if value {
                    self.execute(then_code, program);
                } else if let Some(else_code) = else_code {
                    self.execute(else_code, program);
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
                self.stack.push(VMValue::broadcast(a.magnitude()));
            }
            NodeOp::Length2 => {
                let a = self.stack.pop().unwrap();
                let len2 = (a.x * a.x + a.y * a.y).sqrt();
                self.stack.push(VMValue::new(len2, 0.0, 0.0));
            }
            NodeOp::Length3 => {
                let a = self.stack.pop().unwrap();
                let len3 = (a.x * a.x + a.y * a.y + a.z * a.z).sqrt();
                self.stack.push(VMValue::new(len3, 0.0, 0.0));
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
                self.stack.push(VMValue::new(a.x.sin(), 0.0, 0.0));
            }
            NodeOp::Sin2 => {
                let a = self.stack.pop().unwrap();
                self.stack.push(VMValue::new(a.x.sin(), a.y.sin(), 0.0));
            }
            NodeOp::Cos => {
                let a = self.stack.pop().unwrap();
                self.stack.push(a.map(|x| x.cos()));
            }
            NodeOp::Cos1 => {
                let a = self.stack.pop().unwrap();
                self.stack.push(VMValue::new(a.x.sin(), 0.0, 0.0));
            }
            NodeOp::Cos2 => {
                let a = self.stack.pop().unwrap();
                self.stack.push(VMValue::new(a.x.sin(), a.y.sin(), 0.0));
            }
            NodeOp::Normalize => {
                let a = self.stack.pop().unwrap();
                let len = a.magnitude();
                self.stack.push(if len > 0.0 {
                    a / VMValue::broadcast(len)
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
                    .push(VMValue::new(v.x * c - v.y * s, v.x * s + v.y * c, v.z));
            }
            NodeOp::Dot => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack.push(VMValue::broadcast(a.dot(b)));
            }
            NodeOp::Dot2 => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let dot2 = a.x * b.x + a.y * b.y;
                self.stack.push(VMValue::new(dot2, 0.0, 0.0));
            }
            NodeOp::Dot3 => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let dot3 = a.x * b.x + a.y * b.y + a.z * b.z;
                self.stack.push(VMValue::new(dot3, 0.0, 0.0));
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
                self.stack.push(VMValue::new(a.x % b.x, a.y % b.y, a.z % b.z));
            }*/
            NodeOp::Mod => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let rx = a.x - b.x * (a.x / b.x).floor();
                let ry = a.y - b.y * (a.y / b.y).floor();
                let rz = a.z - b.z * (a.z / b.z).floor();
                self.stack.push(VMValue::new(rx, ry, rz));
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
                    .push(VMValue::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)));
            }
            NodeOp::Max => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack
                    .push(VMValue::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)));
            }
            NodeOp::Mix => {
                let c: VMValue = self.stack.pop().unwrap(); // t
                let b: VMValue = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                // mix(a,b,t) = a*(1-t) + b*t, all component-wise
                let mix = a.clone() + (b - a) * c;
                self.stack.push(mix);
            }
            NodeOp::Smoothstep => {
                let c: VMValue = self.stack.pop().unwrap(); // x
                let b: VMValue = self.stack.pop().unwrap(); // edge1
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
                self.stack.push(VMValue::broadcast(s));
            }
            NodeOp::Step => {
                // step(edge, x): returns 0.0 if x < edge else 1.0 (per component)
                let b: VMValue = self.stack.pop().unwrap(); // x
                let a = self.stack.pop().unwrap(); // edge
                self.stack.push(VMValue::new(
                    if b.x >= a.x { 1.0 } else { 0.0 },
                    if b.y >= a.y { 1.0 } else { 0.0 },
                    if b.z >= a.z { 1.0 } else { 0.0 },
                ));
            }
            NodeOp::Clamp => {
                let c: VMValue = self.stack.pop().unwrap(); // hi
                let b: VMValue = self.stack.pop().unwrap(); // lo
                let a = self.stack.pop().unwrap(); // x
                self.stack.push(VMValue::new(
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
                let b: VMValue = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack
                    .push(VMValue::new(a.x.powf(b.x), a.y.powf(b.y), a.z.powf(b.z)));
            }
            // Comparison (booleans encoded as splat(1.0) / splat(0.0), using .x lane)
            NodeOp::Eq => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let equals = if let (Some(sa), Some(sb)) = (a.as_string(), b.as_string()) {
                    sa == sb
                } else {
                    a.x == b.x
                };
                self.stack
                    .push(VMValue::broadcast(if equals { 1.0 } else { 0.0 }));
            }
            NodeOp::Ne => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let not_equals = if let (Some(sa), Some(sb)) = (a.as_string(), b.as_string()) {
                    sa != sb
                } else {
                    a.x != b.x
                };
                self.stack
                    .push(VMValue::broadcast(if not_equals { 1.0 } else { 0.0 }));
            }
            NodeOp::Lt => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let result = if let (Some(sa), Some(sb)) = (a.as_string(), b.as_string()) {
                    sa < sb
                } else {
                    a.x < b.x
                };
                self.stack
                    .push(VMValue::broadcast(if result { 1.0 } else { 0.0 }));
            }
            NodeOp::Le => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let result = if let (Some(sa), Some(sb)) = (a.as_string(), b.as_string()) {
                    sa <= sb
                } else {
                    a.x <= b.x
                };
                self.stack
                    .push(VMValue::broadcast(if result { 1.0 } else { 0.0 }));
            }
            NodeOp::Gt => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let result = if let (Some(sa), Some(sb)) = (a.as_string(), b.as_string()) {
                    sa > sb
                } else {
                    a.x > b.x
                };
                self.stack
                    .push(VMValue::broadcast(if result { 1.0 } else { 0.0 }));
            }
            NodeOp::Ge => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let result = if let (Some(sa), Some(sb)) = (a.as_string(), b.as_string()) {
                    sa >= sb
                } else {
                    a.x >= b.x
                };
                self.stack
                    .push(VMValue::broadcast(if result { 1.0 } else { 0.0 }));
            }
            // Logical (use .x lane)
            NodeOp::And => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let result = a.is_truthy() && b.is_truthy();
                self.stack
                    .push(VMValue::broadcast(if result { 1.0 } else { 0.0 }));
            }
            NodeOp::Or => {
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                let result = a.is_truthy() || b.is_truthy();
                self.stack
                    .push(VMValue::broadcast(if result { 1.0 } else { 0.0 }));
            }
            // Unary
            NodeOp::Not => {
                let a = self.stack.pop().unwrap();
                self.stack
                    .push(VMValue::broadcast(if a.is_truthy() { 0.0 } else { 1.0 }));
            }
            NodeOp::Neg => {
                let a = self.stack.pop().unwrap();
                self.stack.push(-a);
            }
            NodeOp::Print(count) => {
                let mut args = Vec::with_capacity(*count as usize);
                for _ in 0..*count as usize {
                    if let Some(v) = self.stack.pop() {
                        args.push(v);
                    }
                }
                args.reverse();
                let text = args
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("print: {}", text);
            }
            NodeOp::Format(count) => {
                let mut args = Vec::with_capacity(*count as usize);
                for _ in 0..*count as usize {
                    if let Some(v) = self.stack.pop() {
                        args.push(v);
                    }
                }
                args.reverse();
                let formatted = format_vm_string(&args);
                self.stack.push(VMValue::from_string(formatted));
            }
            NodeOp::GetString => {
                if let Some(v) = self.stack.pop() {
                    if let Some(s) = v.as_string() {
                        self.stack.push(VMValue::from_string(s.to_string()));
                    } else {
                        self.stack.push(VMValue::zero());
                    }
                }
            }
            NodeOp::SetString => {
                if let (Some(new_str), Some(mut target)) = (self.stack.pop(), self.stack.pop()) {
                    target.string = new_str.as_string().map(|s| s.to_string());
                    self.stack.push(target);
                }
            }
            NodeOp::HostCall { name, argc } => {
                let mut args = Vec::with_capacity(*argc as usize);
                for _ in 0..*argc as usize {
                    if let Some(v) = self.stack.pop() {
                        args.push(v);
                    }
                }
                args.reverse();
                match name.as_str() {
                    // In pure VM runs, record common host outputs for tests
                    "action" => {
                        if let Some(v) = args.get(0) {
                            self.outputs.insert("action".to_string(), v.clone());
                        }
                    }
                    "intent" => {
                        if let Some(v) = args.get(0) {
                            self.outputs.insert("intent".to_string(), v.clone());
                        }
                    }
                    "message" => {
                        if let Some(text) = args.get(1) {
                            self.outputs
                                .insert("message_text".to_string(), text.clone());
                        }
                        if let Some(cat) = args.get(2) {
                            self.outputs
                                .insert("message_category".to_string(), cat.clone());
                        }
                    }
                    "id" => {
                        self.stack.push(VMValue::zero());
                    }
                    _ => { /* discard in pure VM mode */ }
                }
            }
            NodeOp::Time => {
                self.stack.push(self.time.clone());
            }
        }
    }

    #[inline(always)]
    pub fn execute_op_host<H: HostHandler>(
        &mut self,
        op: &NodeOp,
        program: &Program,
        host: &mut H,
    ) {
        if host.handle_host_op(op, &mut self.stack) {
            return;
        }

        match op {
            NodeOp::FunctionCall(arity, total_locals, index) => {
                self.push_locals_state();
                self.locals = vec![VMValue::zero(); *total_locals as usize];

                for idx in (0..*arity as usize).rev() {
                    if let Some(arg) = self.stack.pop() {
                        self.locals[idx] = arg;
                    }
                }

                let stack_base = self.stack.len();
                let body = program.user_functions[*index].clone();
                self.execute_host(&body, program, host);

                let ret = if self.return_value.is_some() {
                    self.return_value.take().unwrap_or(VMValue::zero())
                } else if self.stack.len() > stack_base {
                    self.stack.pop().unwrap()
                } else {
                    VMValue::zero()
                };

                self.stack.truncate(stack_base);
                self.pop_locals_state();
                self.stack.push(ret);
            }
            NodeOp::For(init, cond, incr, body) => {
                let base = self.stack.len();
                let mut iter = 0usize;
                self.execute_host(init, program, host);
                self.stack.truncate(base);

                loop {
                    self.execute_host(cond, program, host);

                    let z = self.stack.pop().unwrap();
                    if !z.is_truthy() {
                        break;
                    }
                    self.stack.truncate(base);

                    self.execute_host(body, program, host);
                    self.stack.truncate(base);

                    self.execute_host(incr, program, host);
                    self.stack.truncate(base);

                    iter += 1;
                    if iter > 10_000_000 {
                        panic!("Inifinite for loop detected");
                    }
                }
            }
            NodeOp::If(then_code, else_code) => {
                let value = self.stack.pop().unwrap().is_truthy();
                if value {
                    self.execute_host(then_code, program, host);
                } else if let Some(else_code) = else_code {
                    self.execute_host(else_code, program, host);
                }
            }
            _ => self.execute_op(op, program),
        }
    }

    pub fn execute(&mut self, code: &[NodeOp], program: &Program) {
        for op in code {
            // Unwind if return is set
            if self.return_value.is_some() {
                break;
            }
            self.execute_op(op, program);
        }
    }

    pub fn execute_host<H: HostHandler>(
        &mut self,
        code: &[NodeOp],
        program: &Program,
        host: &mut H,
    ) {
        for op in code {
            if self.return_value.is_some() {
                break;
            }
            self.execute_op_host(op, program, host);
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

    /// Call a function with no arguments
    #[inline]
    pub fn execute_function_no_args(&mut self, index: usize, program: &Program) -> VMValue {
        // Reset state for this call
        self.stack.truncate(0);
        self.return_value = None;

        self.execute(&program.user_functions[index], program);

        // Prefer an explicit return VMValue; else top of stack; else zero
        if let Some(ret) = self.return_value.take() {
            return ret;
        }
        if let Some(rc) = self.stack.pop() {
            rc
        } else {
            VMValue::zero()
        }
    }

    /// Call a function with arguments provided as a slice.
    #[inline]
    pub fn execute_function(
        &mut self,
        args: &[VMValue],
        index: usize,
        program: &Program,
    ) -> VMValue {
        // Reset state for this call
        self.stack.truncate(0);
        self.return_value = None;

        // Prepare locals without reallocating each time
        let argc = args.len();
        let total_locals = program.user_functions_locals[index];
        if self.locals.len() < total_locals {
            self.locals.resize(total_locals, VMValue::zero());
        }
        // Copy args into locals in order (0..argc)
        self.locals[..argc].clone_from_slice(args);

        self.execute(&program.user_functions[index], program);

        // Prefer an explicit return VMValue; else top of stack; else zero
        if let Some(ret) = self.return_value.take() {
            return ret;
        }
        if let Some(rc) = self.stack.pop() {
            rc
        } else {
            VMValue::zero()
        }
    }

    /// Execute a user function, invoking host handler methods inline for host-sensitive ops.
    pub fn execute_function_host<H: HostHandler>(
        &mut self,
        args: &[VMValue],
        index: usize,
        program: &Program,
        host: &mut H,
    ) -> VMValue {
        self.stack.truncate(0);
        self.return_value = None;

        let argc = args.len();
        let total_locals = program.user_functions_locals[index];
        if self.locals.len() < total_locals {
            self.locals.resize(total_locals, VMValue::zero());
        }
        self.locals[..argc].clone_from_slice(args);

        self.execute_host(&program.user_functions[index], program, host);

        if let Some(ret) = self.return_value.take() {
            return ret;
        }
        if let Some(rc) = self.stack.pop() {
            rc
        } else {
            VMValue::zero()
        }
    }
}

fn vm_value_to_string(val: &VMValue) -> String {
    if let Some(s) = val.as_string() {
        s.to_string()
    } else if val.y == val.x && val.z == val.x {
        format!("{}", val.x)
    } else {
        format!("{},{},{}", val.x, val.y, val.z)
    }
}

fn format_vm_string(args: &[VMValue]) -> String {
    if args.is_empty() {
        return String::new();
    }
    let mut iter = args.iter();
    let template = iter.next().unwrap();
    let tmpl = template
        .as_string()
        .map(|s| s.to_string())
        .unwrap_or_else(|| template.x.to_string());

    let mut out = String::new();
    let mut parts = tmpl.split("{}");
    out.push_str(parts.next().unwrap_or_default());
    for (slot, val) in parts.zip(iter) {
        let val_str = vm_value_to_string(val);
        out.push_str(&val_str);
        out.push_str(slot);
    }
    out
}
