//! MaterialZ Tile Language 1: bounded, typed, immutable expression programs.
//! Both the scalar evaluator and Metal emitter consume this same representation.
use crate::{
    Error, Result,
    metadata::{Kernel, Metadata},
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f32),
    Variable(usize),
    Neg(Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Swizzle(Box<Expr>, Vec<usize>),
}
#[derive(Debug, Clone)]
pub enum Control {
    Value,
    Repeat {
        count: Expr,
        maximum: usize,
        end: usize,
    },
    Update {
        target: usize,
    },
    End {
        start: usize,
    },
}
#[derive(Debug, Clone)]
pub struct Step {
    pub name: String,
    pub width: usize,
    pub expression: Expr,
    pub control: Control,
}
#[derive(Debug, Clone)]
pub struct Algorithm {
    pub(crate) raster: Option<std::sync::Arc<crate::raster::Raster>>,
    pub steps: Vec<Step>,
    pub domain: Option<usize>,
    pub image: Option<usize>,
    pub shape: Option<usize>,
    pub primitive: bool,
    pub variables: Vec<(String, usize)>,
}
fn fail(message: impl Into<String>) -> Error {
    Error(format!("Tile language: {}", message.into()))
}
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Name(String),
    Number(f32),
    Symbol(char),
}
fn lex(s: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = vec![];
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1
            }
            out.push(Token::Name(chars[start..i].iter().collect()));
        } else if c.is_ascii_digit()
            || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1
            }
            if i < chars.len() && matches!(chars[i], 'e' | 'E') {
                i += 1;
                if i < chars.len() && matches!(chars[i], '+' | '-') {
                    i += 1
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1
                }
            }
            let value = chars[start..i]
                .iter()
                .collect::<String>()
                .parse::<f32>()
                .map_err(|_| fail("Invalid number"))?;
            if !value.is_finite() {
                return Err(fail("Numbers must be finite"));
            }
            out.push(Token::Number(value));
        } else if "+-*/(),.".contains(c) {
            out.push(Token::Symbol(c));
            i += 1
        } else {
            return Err(fail(format!("Unexpected character {c:?}")));
        }
        if out.len() > 256 {
            return Err(fail("Expression exceeds 256 tokens"));
        }
    }
    Ok(out)
}
type Scope = HashMap<String, (usize, usize)>;
struct Parser<'a> {
    tokens: Vec<Token>,
    index: usize,
    vars: &'a HashMap<String, (usize, usize)>,
}
impl Parser<'_> {
    fn symbol(&mut self, c: char) -> bool {
        if self.tokens.get(self.index) == Some(&Token::Symbol(c)) {
            self.index += 1;
            true
        } else {
            false
        }
    }
    fn expression(&mut self, min: u8, depth: usize) -> Result<(Expr, usize)> {
        if depth > 32 {
            return Err(fail("Expression nesting exceeds 32"));
        }
        let (mut lhs, mut width) = if self.symbol('-') {
            let (e, w) = self.expression(3, depth + 1)?;
            (Expr::Neg(Box::new(e)), w)
        } else if self.symbol('(') {
            let result = self.expression(0, depth + 1)?;
            if !self.symbol(')') {
                return Err(fail("Expected )"));
            }
            result
        } else {
            let token = self
                .tokens
                .get(self.index)
                .cloned()
                .ok_or_else(|| fail("Missing expression"))?;
            self.index += 1;
            match token {
                Token::Number(n) => (Expr::Number(n), 1),
                Token::Name(name) => {
                    if self.symbol('(') {
                        let mut args = vec![];
                        let mut widths = vec![];
                        if !self.symbol(')') {
                            loop {
                                let (e, w) = self.expression(0, depth + 1)?;
                                args.push(e);
                                widths.push(w);
                                if self.symbol(')') {
                                    break;
                                }
                                if !self.symbol(',') {
                                    return Err(fail("Expected , or )"));
                                }
                            }
                        }
                        let w = call_width(&name, &widths)?;
                        (Expr::Call(name, args), w)
                    } else {
                        let (i, w) = *self
                            .vars
                            .get(&name)
                            .ok_or_else(|| fail(format!("Unknown value {name:?}")))?;
                        (Expr::Variable(i), w)
                    }
                }
                _ => return Err(fail("Expected a value")),
            }
        };
        loop {
            if self.symbol('.') {
                let Some(Token::Name(swizzle)) = self.tokens.get(self.index) else {
                    return Err(fail("Expected a component selection"));
                };
                self.index += 1;
                let indices = swizzle
                    .chars()
                    .map(|c| match c {
                        'x' | 'r' => Ok(0),
                        'y' | 'g' => Ok(1),
                        'z' | 'b' => Ok(2),
                        'w' | 'a' => Ok(3),
                        _ => Err(fail("Components must be xyzw or rgba")),
                    })
                    .collect::<Result<Vec<_>>>()?;
                if indices.is_empty() || indices.len() > 4 || indices.iter().any(|&i| i >= width) {
                    return Err(fail("Component selection exceeds value width"));
                }
                width = indices.len();
                lhs = Expr::Swizzle(Box::new(lhs), indices);
                continue;
            }
            let Some(Token::Symbol(op)) = self.tokens.get(self.index) else {
                break;
            };
            let op = *op;
            let prec = match op {
                '+' | '-' => 1,
                '*' | '/' => 2,
                _ => break,
            };
            if prec < min {
                break;
            }
            self.index += 1;
            let (rhs, rw) = self.expression(prec + 1, depth + 1)?;
            width = compatible(width, rw)?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok((lhs, width))
    }
}
fn compatible(a: usize, b: usize) -> Result<usize> {
    if a == b || a == 1 || b == 1 {
        Ok(a.max(b))
    } else {
        Err(fail("Incompatible vector widths"))
    }
}
fn call_width(name: &str, w: &[usize]) -> Result<usize> {
    let count = |n| {
        if w.len() == n {
            Ok(())
        } else {
            Err(fail(format!("{name} expects {n} arguments")))
        }
    };
    match name {
        "vec2" | "vec3" | "vec4" => {
            let n = name.as_bytes()[3] as usize - b'0' as usize;
            if (w.len() == 1 && w[0] == 1) || w.iter().sum::<usize>() == n {
                Ok(n)
            } else {
                Err(fail("Vector constructor width mismatch"))
            }
        }
        "abs" | "floor" | "fract" | "sqrt" | "inversesqrt" | "sin" | "cos" | "nonzero"
        | "radians" => {
            count(1)?;
            Ok(w[0])
        }
        "sign" | "ceil" | "round" | "trunc" | "tan" | "exp" | "exp2" | "log" | "log2"
        | "normalize" => {
            count(1)?;
            Ok(w[0])
        }
        "mod" | "step" | "atan2" => {
            count(2)?;
            compatible(w[0], w[1])
        }
        "atan" => {
            if w.len() == 1 {
                Ok(w[0])
            } else {
                count(2)?;
                compatible(w[0], w[1])
            }
        }
        "smoothstep" => {
            count(3)?;
            compatible(compatible(w[0], w[1])?, w[2])
        }
        "equal" | "different" | "less" | "greater" | "less_equal" | "greater_equal"
        | "logical_and" | "logical_or" => {
            count(2)?;
            compatible(w[0], w[1])?;
            Ok(1)
        }
        "length" => {
            count(1)?;
            Ok(1)
        }
        "dot" => {
            count(2)?;
            if w[0] != w[1] {
                return Err(fail("dot requires matching vectors"));
            }
            Ok(1)
        }
        "min" | "max" | "pow" => {
            count(2)?;
            compatible(w[0], w[1])
        }
        "clamp" | "mix" => {
            count(3)?;
            compatible(compatible(w[0], w[1])?, w[2])
        }
        "select" => {
            count(3)?;
            if w[0] != 1 {
                return Err(fail("select condition must be scalar"));
            }
            if w[1] == w[2] {
                Ok(w[1])
            } else {
                Err(fail("select branches must have the same width"))
            }
        }
        "bit_and" | "bit_or" | "bit_xor" | "shift_left" | "shift_right" => {
            count(2)?;
            compatible(w[0], w[1])
        }
        "sample_scoped" => {
            if w != [2, 1, 3, 3] && w != [2, 1, 3, 3, 3, 3, 3] {
                return Err(fail(
                    "sample_scoped expects UV, footprint, cell key and group key",
                ));
            }
            Ok(4)
        }
        "sample_context" => {
            if w != [2, 3, 3, 1, 3, 3, 3] {
                return Err(fail(
                    "sample_context requires UV, position and three auxiliary vectors",
                ));
            }
            Ok(4)
        }
        "sample" | "sample_image" => {
            count(1)?;
            if w[0] != 2 {
                return Err(fail("sample requires UV coordinates"));
            }
            Ok(4)
        }
        "distance" => {
            count(2)?;
            compatible(w[0], w[1])?;
            Ok(1)
        }
        "hash33" => {
            count(1)?;
            if w[0] != 3 {
                return Err(fail("hash33 requires vec3"));
            }
            Ok(3)
        }
        "component" => {
            count(2)?;
            if w[1] != 1 {
                return Err(fail("Component index must be scalar"));
            }
            Ok(1)
        }
        "disc_context" => {
            if w != [2, 1, 1, 1, 1, 2, 2, 3] {
                return Err(fail("disc_context needs Disc controls and cell context"));
            }
            Ok(1)
        }
        "disc" => {
            if w != [2, 1, 1, 1, 1, 2, 2] {
                return Err(fail(
                    "disc expects uv, bevel, gradient, strength, scale, local scale, offset",
                ));
            }
            Ok(1)
        }
        _ => Err(fail(format!("Unknown function {name:?}"))),
    }
}
impl Algorithm {
    pub fn compile(source: &str, metadata: &Metadata, kernel: Kernel) -> Result<Self> {
        let raster = crate::raster::Raster::parse(source)?.map(std::sync::Arc::new);
        let instruction_bytes: usize = source
            .lines()
            .filter(|line| !line.trim().starts_with("# raster-rgba8 "))
            .map(str::len)
            .sum();
        if instruction_bytes
            > if matches!(kernel, Kernel::Library(_)) {
                262144
            } else {
                32768
            }
        {
            return Err(fail("Source exceeds 32 KiB"));
        }
        let mut variables = vec![
            ("uv".into(), 2),
            ("aa".into(), 1),
            ("input".into(), 4),
            ("pos".into(), 3),
            ("aux1".into(), 3),
            ("aux2".into(), 3),
            ("aux3".into(), 3),
            ("aux_valid".into(), 1),
            ("normal".into(), 3),
            ("sample_time".into(), 1),
            ("instance_cell".into(), 3),
            ("instance_group".into(), 3),
            ("instance_valid".into(), 1),
            ("instance_origin".into(), 3),
            ("instance_axis_x".into(), 3),
            ("instance_axis_y".into(), 3),
        ];
        variables.extend(
            metadata
                .parameters
                .iter()
                .map(|p| (p.id.clone(), p.kind.width())),
        );
        let mut vars: HashMap<String, (usize, usize)> = variables
            .iter()
            .enumerate()
            .map(|(i, (n, w))| (n.clone(), (i, *w)))
            .collect();
        let mut result = Self {
            raster,
            steps: vec![],
            domain: None,
            image: None,
            shape: None,
            primitive: false,
            variables,
        };
        let mut mutable = HashSet::new();
        let mut loops: Vec<(usize, Scope)> = Vec::new();
        let mut version = false;
        for (line, raw) in source.lines().enumerate() {
            let raw = raw.split('#').next().unwrap().trim();
            if raw.is_empty() {
                continue;
            }
            let parse = (|| -> Result<()> {
                if !version {
                    if raw != "tile 1" {
                        return Err(fail("First declaration must be tile 1"));
                    }
                    version = true;
                    return Ok(());
                }
                if [
                    "name ",
                    "category ",
                    "description ",
                    "references ",
                    "group ",
                    "param ",
                    "tooltip ",
                    "choices ",
                    "output_field ",
                    "output_channel ",
                    "default_channel ",
                    "execution ",
                    "distance ",
                ]
                .iter()
                .any(|prefix| raw.starts_with(prefix))
                {
                    return Ok(());
                }
                if let Some(spec) = raw.strip_prefix("repeat ") {
                    let (iterations, condition) = spec
                        .split_once(" while ")
                        .ok_or_else(|| fail("repeat needs maximum times count while condition"))?;
                    let (maximum, count) = iterations
                        .split_once(" times ")
                        .ok_or_else(|| fail("repeat needs a bounded iteration count"))?;
                    let maximum = maximum
                        .parse::<usize>()
                        .ok()
                        .filter(|x| *x > 0 && *x <= 1024)
                        .ok_or_else(|| fail("Repeat maximum must be 1..1024"))?;
                    let parse_expr = |text| -> Result<Expr> {
                        let mut p = Parser {
                            tokens: lex(text)?,
                            index: 0,
                            vars: &vars,
                        };
                        let (e, w) = p.expression(0, 0)?;
                        if w != 1 || p.index != p.tokens.len() {
                            return Err(fail("Repeat count and condition must be scalar"));
                        }
                        Ok(e)
                    };
                    let count = parse_expr(count)?;
                    let expression = parse_expr(condition)?;
                    loops.push((result.steps.len(), vars.clone()));
                    if loops.len() > 8 {
                        return Err(fail("Repeat nesting exceeds eight levels"));
                    }
                    result.steps.push(Step {
                        name: "repeat".into(),
                        width: 1,
                        expression,
                        control: Control::Repeat {
                            count,
                            maximum,
                            end: usize::MAX,
                        },
                    });
                    return Ok(());
                }
                if raw == "end repeat" {
                    let (start, scope) = loops
                        .pop()
                        .ok_or_else(|| fail("end repeat without repeat"))?;
                    let end = result.steps.len();
                    if let Control::Repeat { end: target, .. } = &mut result.steps[start].control {
                        *target = end;
                    }
                    result.steps.push(Step {
                        name: "end".into(),
                        width: 1,
                        expression: Expr::Number(0.),
                        control: Control::End { start },
                    });
                    vars = scope;
                    return Ok(());
                }
                if raw == "primitive border" {
                    if kernel != Kernel::Border || !result.steps.is_empty() || result.primitive {
                        return Err(fail("border is a dedicated bounded sampling primitive"));
                    }
                    result.primitive = true;
                    return Ok(());
                }
                if result.primitive {
                    return Err(fail("No statements after primitive border"));
                }
                let (lhs,rhs)=raw.split_once('=').ok_or_else(||fail("Expected let name = expression, image = expression, shape = expression or domain = expression"))?;
                let lhs = lhs.trim();
                let state = lhs.starts_with("state ");
                let update = lhs.starts_with("update ");
                let name = lhs
                    .strip_prefix("let ")
                    .or_else(|| lhs.strip_prefix("state "))
                    .or_else(|| lhs.strip_prefix("update "))
                    .unwrap_or(lhs)
                    .trim();
                if !name.chars().enumerate().all(|(i, c)| {
                    c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                }) || name.is_empty()
                {
                    return Err(fail("Invalid value name"));
                }
                let output = matches!(name, "image" | "shape" | "domain");
                if !output && !lhs.starts_with("let ") && !state && !update {
                    return Err(fail("Local values need let"));
                }
                if vars.contains_key(name) && !update {
                    return Err(fail(format!("Duplicate value {name:?}")));
                }
                let mut parser = Parser {
                    tokens: lex(rhs)?,
                    index: 0,
                    vars: &vars,
                };
                let (expression, width) = parser.expression(0, 0)?;
                if parser.index != parser.tokens.len() {
                    return Err(fail("Unexpected token after expression"));
                }
                if update {
                    let (target, w) = *vars.get(name).ok_or_else(|| fail("Unknown state"))?;
                    if loops.is_empty() || !mutable.contains(&target) || w != width {
                        return Err(fail("update requires a matching state inside repeat"));
                    }
                    result.steps.push(Step {
                        name: name.into(),
                        width,
                        expression,
                        control: Control::Update { target },
                    });
                    return Ok(());
                }
                let idx = result.steps.len();
                match name {
                    "image" => {
                        if width != 4 {
                            return Err(fail("image needs vec4 RGBA"));
                        }
                        result.image = Some(idx)
                    }
                    "shape" => {
                        if width != 1 || !kernel.is_shape() {
                            return Err(fail("shape needs a scalar on a shape tile"));
                        }
                        result.shape = Some(idx)
                    }
                    "domain" => {
                        if width != 2 || kernel != Kernel::Transform {
                            return Err(fail("domain needs vec2 on a Transform tile"));
                        }
                        if result.steps.iter().any(|s| uses_input(&s.expression))
                            || uses_input(&expression)
                        {
                            return Err(fail("domain must not depend on input"));
                        }
                        result.domain = Some(idx)
                    }
                    _ => {}
                }
                vars.insert(name.into(), (result.variables.len() + idx, width));
                if state {
                    mutable.insert(result.variables.len() + idx);
                }
                result.steps.push(Step {
                    name: name.into(),
                    width,
                    expression,
                    control: Control::Value,
                });
                if result.steps.len()
                    > if matches!(kernel, Kernel::Library(_)) {
                        2048
                    } else {
                        128
                    }
                {
                    return Err(fail("Tile exceeds 128 instructions"));
                }
                Ok(())
            })();
            parse.map_err(|e| Error(format!("line {}: {e}", line + 1)))?;
        }
        if !loops.is_empty() {
            return Err(fail("Repeat needs end repeat"));
        }
        if !version || (!result.primitive && result.image.is_none() && result.shape.is_none()) {
            return Err(fail("Tile needs image or shape output"));
        }
        if result.shape.is_some() && result.image.is_some() {
            return Err(fail(
                "Shape tiles produce their image from coverage; choose one output",
            ));
        }
        if result.operations()
            > if matches!(kernel, Kernel::Library(_)) {
                32768
            } else {
                4096
            }
        {
            return Err(fail("Tile exceeds 4096 expression operations"));
        }
        Ok(result)
    }
    pub fn operations(&self) -> usize {
        fn count(e: &Expr) -> usize {
            1 + match e {
                Expr::Neg(a) | Expr::Swizzle(a, _) => count(a),
                Expr::Binary(_, a, b) => count(a) + count(b),
                Expr::Call(_, args) => args.iter().map(count).sum(),
                _ => 0,
            }
        }
        self.steps
            .iter()
            .map(|s| count(&s.expression))
            .sum::<usize>()
            .max(1)
    }
    pub fn cost(&self, parameters: &[Option<Vec<f32>>]) -> (usize, usize) {
        fn known(e: &Expr, v: &[Option<Vec<f32>>]) -> bool {
            match e {
                Expr::Number(_) => true,
                Expr::Variable(i) => v[*i].is_some(),
                Expr::Neg(a) | Expr::Swizzle(a, _) => known(a, v),
                Expr::Binary(_, a, b) => known(a, v) && known(b, v),
                Expr::Call(n, a) => {
                    n != "sample"
                        && n != "sample_image"
                        && n != "sample_context"
                        && n != "sample_scoped"
                        && a.iter().all(|e| known(e, v))
                }
            }
        }
        fn sample_count(e: &Expr) -> usize {
            match e {
                Expr::Call(n, a) => {
                    usize::from(
                        n == "sample"
                            || n == "sample_image"
                            || n == "sample_context"
                            || n == "sample_scoped",
                    ) + a.iter().map(sample_count).sum::<usize>()
                }
                Expr::Neg(a) | Expr::Swizzle(a, _) => sample_count(a),
                Expr::Binary(_, a, b) => sample_count(a) + sample_count(b),
                _ => 0,
            }
        }
        fn op_count(e: &Expr) -> usize {
            1 + match e {
                Expr::Call(_, a) => a.iter().map(op_count).sum(),
                Expr::Neg(a) | Expr::Swizzle(a, _) => op_count(a),
                Expr::Binary(_, a, b) => op_count(a) + op_count(b),
                _ => 0,
            }
        }
        let mut values = vec![None; self.variables.len()];
        for (i, p) in parameters.iter().enumerate() {
            values[16 + i] = p.clone();
        }
        values.resize(self.variables.len() + self.steps.len(), None);
        let mut weight = 1usize;
        let mut stack = Vec::new();
        let mut ops = 0usize;
        let mut calls = 1usize;
        for (i, s) in self.steps.iter().enumerate() {
            let get = |e: &Expr, values: &[Option<Vec<f32>>]| -> Option<Vec<f32>> {
                if !known(e, values) {
                    return None;
                }
                let dense = values
                    .iter()
                    .map(|v| v.clone().unwrap_or_default())
                    .collect::<Vec<_>>();
                Some(evaluate(e, &dense, &mut |_, _| [0.; 4]))
            };
            match &s.control {
                Control::Repeat {
                    count,
                    maximum,
                    end,
                } => {
                    let bound = get(count, &values)
                        .map(|v| {
                            if v[0].is_finite() {
                                v[0].ceil().clamp(0., *maximum as f32) as usize
                            } else {
                                *maximum
                            }
                        })
                        .unwrap_or(*maximum);
                    let header_weight = weight.saturating_mul(bound.saturating_add(1));
                    ops =
                        ops.saturating_add(header_weight.saturating_mul(
                            op_count(count).saturating_add(op_count(&s.expression)),
                        ));
                    calls = calls.saturating_add(header_weight.saturating_mul(
                        sample_count(count).saturating_add(sample_count(&s.expression)),
                    ));
                    stack.push(weight);
                    weight = weight.saturating_mul(bound);
                    for step in &self.steps[i + 1..*end] {
                        if let Control::Update { target } = step.control {
                            values[target] = None;
                        }
                    }
                }
                Control::End { .. } => weight = stack.pop().unwrap_or(1),
                _ => {
                    ops = ops.saturating_add(weight.saturating_mul(op_count(&s.expression)));
                    calls =
                        calls.saturating_add(weight.saturating_mul(sample_count(&s.expression)));
                    let value = get(&s.expression, &values);
                    if let Control::Update { target } = s.control {
                        values[target] = None;
                    }
                    values[self.variables.len() + i] = value;
                }
            }
        }
        (ops.max(1), calls)
    }
    pub(crate) fn evaluate(
        &self,
        uv: [f32; 2],
        aa: f32,
        parameters: Vec<Vec<f32>>,
        context: crate::cpu::SampleContext,
        mut input: impl FnMut([f32; 2], Option<crate::cpu::SampleContext>) -> [f32; 4],
    ) -> (Option<[f32; 4]>, Option<f32>) {
        if let Some(raster) = &self.raster {
            return (Some(raster.sample(uv)), None);
        }
        let mut values = vec![
            uv.to_vec(),
            vec![aa],
            vec![1.; 4],
            context.position.to_vec(),
            context.auxiliary[0].to_vec(),
            context.auxiliary[1].to_vec(),
            context.auxiliary[2].to_vec(),
            vec![if context.valid { 1. } else { 0. }],
            context.normal.to_vec(),
            vec![context.time],
            context.cell.to_vec(),
            context.group.to_vec(),
            vec![if context.instance_valid { 1. } else { 0. }],
            context.origin.to_vec(),
            context.axis_x.to_vec(),
            context.axis_y.to_vec(),
        ];
        values.extend(parameters);
        if self.domain.is_none() {
            values[2] = input(uv, None).to_vec()
        }
        let base = values.len();
        values.extend(self.steps.iter().map(|s| vec![0.; s.width]));
        let mut image = None;
        let mut shape = None;
        let mut i = 0;
        let mut iterations = vec![0usize; self.steps.len()];
        while i < self.steps.len() {
            let step = &self.steps[i];
            match &step.control {
                Control::Repeat {
                    count,
                    maximum,
                    end,
                } => {
                    let count = evaluate(count, &values, &mut input)[0]
                        .ceil()
                        .clamp(0., *maximum as f32) as usize;
                    if iterations[i] >= count
                        || evaluate(&step.expression, &values, &mut input)[0] == 0.
                    {
                        iterations[i] = 0;
                        i = end + 1;
                        continue;
                    }
                    iterations[i] += 1;
                }
                Control::End { start } => {
                    i = *start;
                    continue;
                }
                _ => {
                    let value = evaluate(&step.expression, &values, &mut input);
                    if let Control::Update { target } = step.control {
                        values[target] = value.clone();
                    }
                    if self.domain == Some(i) {
                        values[2] = input([value[0], value[1]], None).to_vec();
                    }
                    if self.image == Some(i) {
                        image = Some([value[0], value[1], value[2], value[3]]);
                    }
                    if self.shape == Some(i) {
                        shape = Some(value[0]);
                    }
                    values[base + i] = value;
                }
            }
            i += 1;
        }
        (image, shape)
    }
    /// Emit typed SSA statements. The caller inserts callable input evaluation at domain.
    pub fn emit(
        &self,
        mut input: impl FnMut(&str) -> String,
        sample: impl Fn(&[String]) -> String,
    ) -> String {
        let mut names = self
            .variables
            .iter()
            .map(|(name, _)| {
                if name == "input" {
                    "mz_input".into()
                } else if name == "pos" {
                    "ctx.position".into()
                } else if name == "aux1" {
                    "ctx.a".into()
                } else if name == "aux2" {
                    "ctx.b".into()
                } else if name == "aux3" {
                    "ctx.c".into()
                } else if name == "aux_valid" {
                    "ctx.valid".into()
                } else if name == "normal" {
                    "ctx.normal".into()
                } else if name == "instance_cell" {
                    "ctx.cell".into()
                } else if name == "instance_group" {
                    "ctx.group".into()
                } else if name == "instance_origin" {
                    "ctx.origin".into()
                } else if name == "instance_axis_x" {
                    "ctx.axis_x".into()
                } else if name == "instance_axis_y" {
                    "ctx.axis_y".into()
                } else if name == "instance_valid" {
                    "ctx.instance_valid".into()
                } else if name == "sample_time" {
                    "ctx.time".into()
                } else {
                    name.clone()
                }
            })
            .collect::<Vec<String>>();
        let mut widths = self.variables.iter().map(|(_, w)| *w).collect::<Vec<_>>();
        let mut s = String::new();
        if self.domain.is_none() {
            s.push_str(&input("uv"))
        }
        for (i, step) in self.steps.iter().enumerate() {
            let name = format!("mz_value_{i}");
            match &step.control {
                Control::Repeat { count, maximum, .. } => {
                    let count = emit(count, &names, &widths, &sample);
                    let condition = emit(&step.expression, &names, &widths, &sample);
                    s.push_str(&format!("    for(uint mz_iter_{i}=0; mz_iter_{i}<mz_loop_count({count},{maximum}u) && ({condition}!=0.0f); ++mz_iter_{i}) {{\n"));
                    names.push("0.0f".into());
                    widths.push(1);
                }
                Control::End { .. } => {
                    s.push_str("    }\n");
                    names.push("0.0f".into());
                    widths.push(1);
                }
                _ => {
                    let kind = if step.width == 1 {
                        "float".into()
                    } else {
                        format!("float{}", step.width)
                    };
                    s.push_str(&format!(
                        "    {kind} {name} = {};\n",
                        emit(&step.expression, &names, &widths, &sample)
                    ));
                    if let Control::Update { target } = step.control {
                        s.push_str(&format!("    {} = {name};\n", names[target]));
                    }
                    names.push(name.clone());
                    widths.push(step.width);
                    if self.domain == Some(i) {
                        s.push_str(&input(&name));
                    }
                }
            }
        }
        if let Some(i) = self.image {
            s.push_str(&format!("    result.color = mz_value_{i};\n"))
        }
        if let Some(i) = self.shape {
            s.push_str(&format!("    float mz_shape_distance = mz_value_{i};\n"))
        }
        s
    }
}
fn uses_input(e: &Expr) -> bool {
    match e {
        Expr::Variable(2) => true,
        Expr::Neg(a) | Expr::Swizzle(a, _) => uses_input(a),
        Expr::Binary(_, a, b) => uses_input(a) || uses_input(b),
        Expr::Call(_, args) => args.iter().any(uses_input),
        _ => false,
    }
}
fn lane(v: &[f32], i: usize) -> f32 {
    v[if v.len() == 1 { 0 } else { i }]
}
fn evaluate(
    e: &Expr,
    values: &[Vec<f32>],
    input: &mut dyn FnMut([f32; 2], Option<crate::cpu::SampleContext>) -> [f32; 4],
) -> Vec<f32> {
    match e {
        Expr::Number(n) => vec![*n],
        Expr::Variable(i) => values[*i].clone(),
        Expr::Neg(a) => evaluate(a, values, input).iter().map(|v| -v).collect(),
        Expr::Swizzle(a, idx) => {
            let a = evaluate(a, values, input);
            idx.iter().map(|&i| a[i]).collect()
        }
        Expr::Binary(op, a, b) => {
            let a = evaluate(a, values, input);
            let b = evaluate(b, values, input);
            (0..a.len().max(b.len()))
                .map(|i| {
                    let x = lane(&a, i);
                    let y = lane(&b, i);
                    match op {
                        '+' => x + y,
                        '-' => x - y,
                        '*' => x * y,
                        '/' => x / crate::cpu::nonzero(y),
                        _ => unreachable!(),
                    }
                })
                .collect()
        }
        Expr::Call(name, args) => {
            let args = args
                .iter()
                .map(|a| evaluate(a, values, input))
                .collect::<Vec<_>>();
            let a = &args[0];
            if name == "sample_scoped" {
                let v = |i: usize| [values[i][0], values[i][1], values[i][2]];
                let frame = if args.len() == 7 {
                    let local = |i: usize| [args[i][0], args[i][1], args[i][2]];
                    let transform = |p: [f32; 3]| {
                        std::array::from_fn(|i| {
                            values[14][i] * p[0]
                                + values[15][i] * p[1]
                                + if i == 2 { p[2] } else { 0. }
                        })
                    };
                    let o: [f32; 3] = transform(local(4));
                    (
                        std::array::from_fn(|i| values[13][i] + o[i]),
                        transform(local(5)),
                        transform(local(6)),
                    )
                } else {
                    (v(13), v(14), v(15))
                };
                return input(
                    [a[0], a[1]],
                    Some(crate::cpu::SampleContext {
                        origin: frame.0,
                        axis_x: frame.1,
                        axis_y: frame.2,
                        position: [a[0], a[1], values[3][2]],
                        auxiliary: [v(4), v(5), v(6)],
                        valid: values[7][0] != 0.,
                        normal: v(8),
                        time: values[9][0],
                        cell: [args[2][0], args[2][1], args[2][2]],
                        group: [args[3][0], args[3][1], args[3][2]],
                        instance_valid: true,
                        footprint: Some(args[1][0]),
                    }),
                )
                .to_vec();
            }
            if name == "sample_context" {
                let v = |i: usize| [args[i][0], args[i][1], args[i][2]];
                return input(
                    [a[0], a[1]],
                    Some(crate::cpu::SampleContext {
                        position: v(1),
                        auxiliary: [v(4), v(5), v(6)],
                        normal: v(2),
                        time: args[3][0],
                        valid: true,
                        cell: [values[10][0], values[10][1], values[10][2]],
                        group: [values[11][0], values[11][1], values[11][2]],
                        instance_valid: values[12][0] != 0.,
                        origin: [values[13][0], values[13][1], values[13][2]],
                        axis_x: [values[14][0], values[14][1], values[14][2]],
                        axis_y: [values[15][0], values[15][1], values[15][2]],
                        footprint: None,
                    }),
                )
                .to_vec();
            }
            if name == "sample" || name == "sample_image" {
                return input([a[0], a[1]], None).to_vec();
            }
            match name.as_str() {
                "vec2" | "vec3" | "vec4" => {
                    let n = name.as_bytes()[3] as usize - b'0' as usize;
                    if args.len() == 1 && a.len() == 1 {
                        vec![a[0]; n]
                    } else {
                        args.into_iter().flatten().collect()
                    }
                }
                "normalize" => {
                    let length = crate::cpu::nonzero(a.iter().map(|x| x * x).sum::<f32>().sqrt());
                    a.iter().map(|x| x / length).collect()
                }
                "equal" | "different" | "less" | "greater" | "less_equal" | "greater_equal"
                | "logical_and" | "logical_or" => {
                    let test = |i| {
                        let x = lane(a, i);
                        let y = lane(&args[1], i);
                        match name.as_str() {
                            "equal" => x == y,
                            "different" => x != y,
                            "less" => x < y,
                            "greater" => x > y,
                            "less_equal" => x <= y,
                            "greater_equal" => x >= y,
                            "logical_and" => x != 0. && y != 0.,
                            _ => x != 0. || y != 0.,
                        }
                    };
                    let n = a.len().max(args[1].len());
                    let yes = if name == "different" {
                        (0..n).any(test)
                    } else {
                        (0..n).all(test)
                    };
                    vec![if yes { 1. } else { 0. }]
                }
                "length" => vec![a.iter().map(|x| x * x).sum::<f32>().sqrt()],
                "dot" => vec![a.iter().zip(&args[1]).map(|(x, y)| x * y).sum()],
                "select" => {
                    if a[0] != 0. {
                        args[1].clone()
                    } else {
                        args[2].clone()
                    }
                }
                "distance" => vec![
                    a.iter()
                        .zip(&args[1])
                        .map(|(x, y)| (x - y) * (x - y))
                        .sum::<f32>()
                        .sqrt(),
                ],
                "hash33" => crate::cpu::hash3([
                    crate::cpu::uint_bits(a[0]),
                    crate::cpu::uint_bits(a[1]),
                    crate::cpu::uint_bits(a[2]),
                ])
                .to_vec(),
                "component" => {
                    vec![a[(args[1][0].round().clamp(0., (a.len() - 1) as f32)) as usize]]
                }
                "disc_context" => vec![crate::cpu::disc_with_context(
                    [a[0], a[1]],
                    args[1][0],
                    args[2][0] != 0.,
                    args[3][0],
                    args[4][0],
                    [args[5][0], args[5][1]],
                    [args[6][0], args[6][1]],
                    [args[7][0], args[7][1], args[7][2]],
                )],
                "disc" => vec![crate::cpu::disc(
                    [a[0], a[1]],
                    args[1][0],
                    args[2][0] != 0.,
                    args[3][0],
                    args[4][0],
                    [args[5][0], args[5][1]],
                    [args[6][0], args[6][1]],
                )],
                _ => (0..args.iter().map(Vec::len).max().unwrap())
                    .map(|i| {
                        let x = lane(a, i);
                        let y = args.get(1).map(|v| lane(v, i)).unwrap_or(0.);
                        let z = args.get(2).map(|v| lane(v, i)).unwrap_or(0.);
                        match name.as_str() {
                            "sign" => {
                                if x > 0. {
                                    1.
                                } else if x < 0. {
                                    -1.
                                } else {
                                    0.
                                }
                            }
                            "ceil" => x.ceil(),
                            "round" => x.round(),
                            "trunc" => x.trunc(),
                            "tan" => x.tan(),
                            "exp" => x.exp(),
                            "exp2" => x.exp2(),
                            "log" => x.ln(),
                            "log2" => x.log2(),
                            "mod" => x - y * (x / crate::cpu::nonzero(y)).floor(),
                            "step" => {
                                if y < x {
                                    0.
                                } else {
                                    1.
                                }
                            }
                            "atan" => {
                                if args.len() == 1 {
                                    x.atan()
                                } else {
                                    x.atan2(y)
                                }
                            }
                            "atan2" => x.atan2(y),
                            "smoothstep" => {
                                let t = ((z - x) / crate::cpu::nonzero(y - x)).clamp(0., 1.);
                                t * t * (3. - 2. * t)
                            }
                            "bit_and" => (x as u32 & y as u32) as f32,
                            "bit_or" => (x as u32 | y as u32) as f32,
                            "bit_xor" => (x as u32 ^ y as u32) as f32,
                            "shift_left" => (x as u32).wrapping_shl(y as u32) as f32,
                            "shift_right" => (x as u32).wrapping_shr(y as u32) as f32,
                            "abs" => x.abs(),
                            "floor" => x.floor(),
                            "fract" => x - x.floor(),
                            "sqrt" => x.sqrt(),
                            "inversesqrt" => 1. / x.sqrt(),
                            "sin" => crate::math::sin(x),
                            "cos" => crate::math::cos(x),
                            "nonzero" => crate::cpu::nonzero(x),
                            "radians" => x.to_radians(),
                            "min" => x.min(y),
                            "max" => x.max(y),
                            "pow" => x.powf(y),
                            "clamp" => x.max(y).min(z),
                            "mix" => x * (1. - z) + y * z,
                            _ => unreachable!(),
                        }
                    })
                    .collect(),
            }
        }
    }
}
fn width(e: &Expr, widths: &[usize]) -> usize {
    match e {
        Expr::Number(_) => 1,
        Expr::Variable(i) => widths[*i],
        Expr::Neg(a) => width(a, widths),
        Expr::Swizzle(_, idx) => idx.len(),
        Expr::Binary(_, a, b) => width(a, widths).max(width(b, widths)),
        Expr::Call(name, args) => call_width(
            name,
            &args.iter().map(|a| width(a, widths)).collect::<Vec<_>>(),
        )
        .expect("Typed expression"),
    }
}
fn emit(
    e: &Expr,
    names: &[String],
    widths: &[usize],
    sample: &dyn Fn(&[String]) -> String,
) -> String {
    match e {
        Expr::Number(n) => format!("{n:?}f"),
        Expr::Variable(i) => names[*i].clone(),
        Expr::Neg(a) => format!("(-{})", emit(a, names, widths, sample)),
        Expr::Swizzle(a, idx) => {
            let value = emit(a, names, widths, sample);
            if width(a, widths) == 1 {
                if idx.len() == 1 {
                    value
                } else {
                    format!("float{}({value})", idx.len())
                }
            } else {
                format!(
                    "({value}).{}",
                    idx.iter()
                        .map(|&i| ["x", "y", "z", "w"][i])
                        .collect::<String>()
                )
            }
        }
        Expr::Binary(op, a, b) => {
            if *op == '/' {
                format!(
                    "({} / mz_nonzero({}))",
                    emit(a, names, widths, sample),
                    emit(b, names, widths, sample)
                )
            } else {
                format!(
                    "({} {op} {})",
                    emit(a, names, widths, sample),
                    emit(b, names, widths, sample)
                )
            }
        }
        Expr::Call(name, args) => {
            if matches!(
                name.as_str(),
                "bit_and" | "bit_or" | "bit_xor" | "shift_left" | "shift_right"
            ) {
                let n = width(e, widths);
                let ty = if n == 1 {
                    "uint".into()
                } else {
                    format!("uint{n}")
                };
                let out = if n == 1 {
                    "float".into()
                } else {
                    format!("float{n}")
                };
                let a = emit(&args[0], names, widths, sample);
                let b = emit(&args[1], names, widths, sample);
                let op = match name.as_str() {
                    "bit_and" => "&",
                    "bit_or" => "|",
                    "bit_xor" => "^",
                    "shift_left" => "<<",
                    _ => ">>",
                };
                return format!("{out}({ty}({a}) {op} {ty}({b}))");
            }
            if name == "sample"
                || name == "sample_context"
                || name == "sample_image"
                || name == "sample_scoped"
            {
                let mut arguments = args
                    .iter()
                    .map(|a| emit(a, names, widths, sample))
                    .collect::<Vec<_>>();
                if name == "sample_scoped" && arguments.len() == 7 {
                    arguments.push("frame".into())
                }
                if name == "sample_image" {
                    arguments.push("rgba".into())
                }
                return sample(&arguments);
            }
            let result_width = width(e, widths);
            if matches!(
                name.as_str(),
                "equal"
                    | "different"
                    | "less"
                    | "greater"
                    | "less_equal"
                    | "greater_equal"
                    | "logical_and"
                    | "logical_or"
            ) {
                let op = match name.as_str() {
                    "equal" => "==",
                    "different" => "!=",
                    "less" => "<",
                    "greater" => ">",
                    "less_equal" => "<=",
                    "greater_equal" => ">=",
                    "logical_and" => "&&",
                    _ => "||",
                };
                let n = width(&args[0], widths).max(width(&args[1], widths));
                let a = emit(&args[0], names, widths, sample);
                let b = emit(&args[1], names, widths, sample);
                let sw = |text: &str, v: &Expr, i: usize| {
                    if width(v, widths) == 1 {
                        format!("({text})")
                    } else {
                        format!("({text}).{}", ["x", "y", "z", "w"][i])
                    }
                };
                return format!(
                    "(({}) ? 1.0f : 0.0f)",
                    (0..n)
                        .map(|i| {
                            let a = sw(&a, &args[0], i);
                            let b = sw(&b, &args[1], i);
                            if matches!(name.as_str(), "logical_and" | "logical_or") {
                                format!("(({a} != 0.0f) {op} ({b} != 0.0f))")
                            } else {
                                format!("({a} {op} {b})")
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(if name == "different" { " || " } else { " && " })
                );
            }
            if name == "component" {
                let a = emit(&args[0], names, widths, sample);
                let b = emit(&args[1], names, widths, sample);
                return format!(
                    "({a})[uint(clamp(round({b}), 0.0f, {}.0f))]",
                    width(&args[0], widths) - 1
                );
            }
            if name == "normalize" {
                let a = emit(&args[0], names, widths, sample);
                return format!("({a} / mz_nonzero(mz_length({a})))");
            }
            let args = args
                .iter()
                .map(|a| {
                    let code = emit(a, names, widths, sample);
                    if matches!(
                        name.as_str(),
                        "min"
                            | "max"
                            | "clamp"
                            | "mix"
                            | "pow"
                            | "mod"
                            | "step"
                            | "smoothstep"
                            | "atan"
                    ) && width(a, widths) == 1
                        && result_width > 1
                    {
                        format!("float{result_width}({code})")
                    } else {
                        code
                    }
                })
                .collect::<Vec<_>>();
            if name == "mod" {
                return format!(
                    "({} - {} * floor({} / mz_nonzero({})))",
                    args[0], args[1], args[0], args[1]
                );
            }
            if name == "select" {
                return format!("(({} != 0.0f) ? {} : {})", args[0], args[1], args[2]);
            }
            let f = match name.as_str() {
                "mod" => "fmod",
                "atan" if args.len() == 2 => "atan2",
                "vec2" => "float2",
                "vec3" => "float3",
                "vec4" => "float4",
                "inversesqrt" => "mz_inversesqrt",
                "radians" => "mz_radians",
                "nonzero" => "mz_nonzero",
                "sin" => "mz_sin",
                "cos" => "mz_cos",
                "dot" => "mz_dot",
                "length" => "mz_length",
                "disc" => "mz_disc_scalar",
                "disc_context" => "mz_disc_context",
                "hash33" => "mz_hash33",
                other => other,
            };
            format!("{f}({})", args.join(", "))
        }
    }
}

pub fn source(kernel: Kernel) -> &'static str {
    match kernel {
        Kernel::Library(i) => crate::library::TILES[i as usize].0,
        Kernel::Perlin => include_str!("../shaders/tiles/perlin.tile"),
        Kernel::Disc => include_str!("../shaders/tiles/disc.tile"),
        Kernel::Multiply => include_str!("../shaders/tiles/multiply.tile"),
        Kernel::ColorBlend => include_str!("../shaders/tiles/color_blend.tile"),
        Kernel::Transform => include_str!("../shaders/tiles/transform.tile"),
        Kernel::Circle => include_str!("../shaders/tiles/circle.tile"),
        Kernel::RoundedRectangle => include_str!("../shaders/tiles/rectangle.tile"),
        Kernel::Color => include_str!("../shaders/tiles/color.tile"),
        Kernel::Border => include_str!("../shaders/tiles/border.tile"),
        Kernel::PeriodicProfile => include_str!("../shaders/tiles/periodic_profile.tile"),
        Kernel::CompareSelect => include_str!("../shaders/tiles/compare_select.tile"),
        Kernel::CellData => include_str!("../shaders/tiles/cell_data.tile"),
        Kernel::DirectionalGrain => include_str!("../shaders/tiles/directional_grain.tile"),
        Kernel::HeightToNormal => include_str!("../shaders/tiles/height_to_normal.tile"),
    }
}

/// Source declarations provide the UI controls; historical declarations are provenance only.
pub fn definition(source: &str) -> Result<Metadata> {
    use crate::metadata::{Group, OutputChannel, Parameter, ValueType};
    let mut m = Metadata {
        name: String::new(),
        category: String::new(),
        description: String::new(),
        references: String::new(),
        output_channels: vec![],
        output_fields: vec![],
        default_channel: None,
        clears_distance: false,
        image_execution: None,
        groups: vec![],
        parameters: vec![],
    };
    let mut group = String::new();
    for raw in source.lines() {
        let t = crate::format::tokens(raw)?;
        if t.is_empty() {
            continue;
        }
        match t[0].as_str() {
            "name" | "category" | "description" | "references" => {
                if t.len() != 2 {
                    return Err(fail("Metadata declaration needs one quoted value"));
                }
                match t[0].as_str() {
                    "name" => m.name = t[1].clone(),
                    "category" => m.category = t[1].clone(),
                    "description" => m.description = t[1].clone(),
                    _ => m.references = t[1].clone(),
                }
            }
            "execution" => {
                if t.len() != 4 || t[1] != "image" || m.image_execution.is_some() {
                    return Err(fail(
                        "execution image requires radius and units parameter IDs",
                    ));
                }
                m.image_execution = Some((t[2].clone(), t[3].clone()));
            }
            "distance" => {
                if t.len() != 2 || !matches!(t[1].as_str(), "clear" | "preserve") {
                    return Err(fail("distance expects clear or preserve"));
                }
                m.clears_distance = t[1] == "clear";
            }
            "output_field" => {
                if t.len() != 5
                    || t[1].is_empty()
                    || !t[1].chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    || !t[4].chars().all(|c| matches!(c, 'x' | 'y' | 'z' | 'w'))
                    || t[4].is_empty()
                    || t[4].len() > 4
                    || m.output_fields.iter().any(|f| f.id == t[1])
                {
                    return Err(fail(
                        "output_field needs unique ID, label, mode and component selection",
                    ));
                }
                let mode = t[3]
                    .parse::<f32>()
                    .map_err(|_| fail("Invalid field mode"))?;
                if !mode.is_finite() {
                    return Err(fail("Field mode must be finite"));
                }
                m.output_fields.push(crate::metadata::OutputField {
                    id: t[1].clone(),
                    label: t[2].clone(),
                    mode,
                    components: t[4].clone(),
                });
            }
            "output_channel" => {
                if t.len() != 3
                    || !matches!(t[1].as_str(), "x" | "y" | "z" | "w")
                    || t[2].is_empty()
                    || m.output_channels.iter().any(|o| o.component == t[1])
                {
                    return Err(fail(
                        "output_channel needs a unique x/y/z/w component and a label",
                    ));
                }
                m.output_channels.push(OutputChannel {
                    component: t[1].clone(),
                    label: t[2].clone(),
                });
            }
            "default_channel" => {
                if t.len() != 2
                    || !matches!(t[1].as_str(), "x" | "y" | "z" | "w")
                    || m.default_channel.is_some()
                {
                    return Err(fail("default_channel needs one x/y/z/w component"));
                }
                m.default_channel = Some(t[1].clone());
            }
            "choices" => {
                let p = m
                    .parameters
                    .last_mut()
                    .ok_or_else(|| fail("choices needs a parameter"))?;
                if p.kind.width() != 1 || t.len() < 2 || !p.choices.is_empty() {
                    return Err(fail("choices needs a scalar parameter and nonempty labels"));
                }
                p.choices = t[1..].to_vec();
            }
            "group" => {
                if t.len() != 3 || !matches!(t[2].as_str(), "true" | "false") {
                    return Err(fail("group needs label and expanded boolean"));
                }
                group = t[1].clone();
                m.groups.push(Group {
                    label: group.clone(),
                    expanded: t[2] == "true",
                });
            }
            "param" => {
                if t.len() < 6 || t[4] != "default" {
                    return Err(fail("param needs type, id, label and default"));
                }
                let kind = match t[1].as_str() {
                    "float" => ValueType::Float,
                    "vec2" => ValueType::Vec2,
                    "vec3" => ValueType::Vec3,
                    "color" => ValueType::Color,
                    "bool" => ValueType::Bool,
                    _ => return Err(fail("Unknown parameter type")),
                };
                let end = t[5..]
                    .iter()
                    .position(|v| v == "range")
                    .map(|i| i + 5)
                    .unwrap_or(t.len());
                let numbers = |values: &[String]| {
                    values
                        .iter()
                        .map(|v| {
                            v.parse::<f32>()
                                .ok()
                                .filter(|x| x.is_finite())
                                .ok_or_else(|| fail("Invalid parameter number"))
                        })
                        .collect::<Result<Vec<_>>>()
                };
                let default = numbers(&t[5..end])?;
                if default.len() != kind.width() {
                    return Err(fail("Parameter default width mismatch"));
                }
                let range = if end < t.len() {
                    let values = numbers(&t[end + 1..])?;
                    if values.len() != 2 {
                        return Err(fail("range needs two numbers"));
                    }
                    Some((values[0], values[1]))
                } else {
                    None
                };
                if m.parameters.iter().any(|p| p.id == t[2]) {
                    return Err(fail("Duplicate parameter id"));
                }
                m.parameters.push(Parameter {
                    id: t[2].clone(),
                    label: t[3].clone(),
                    group: group.clone(),
                    kind,
                    default,
                    range,
                    tooltip: None,
                    choices: vec![],
                });
            }
            "tooltip" => {
                if t.len() != 2 {
                    return Err(fail("tooltip needs quoted text"));
                }
                m.parameters
                    .last_mut()
                    .ok_or_else(|| fail("tooltip outside parameter"))?
                    .tooltip = Some(t[1].clone());
            }
            _ => {}
        }
    }
    if !m.output_fields.is_empty() {
        let output = m
            .parameters
            .iter()
            .find(|p| p.id == "output" && p.kind == ValueType::Float)
            .ok_or_else(|| fail("Named fields need a scalar Output selector"))?;
        for field in &m.output_fields {
            if field.mode.fract() != 0.
                || field.mode < 0.
                || field.mode as usize >= output.choices.len()
            {
                return Err(fail("Named field mode must name an Output choice"));
            }
        }
    }
    if let Some((radius, units)) = &m.image_execution {
        for key in [radius, units] {
            if !m
                .parameters
                .iter()
                .any(|p| &p.id == key && p.kind == ValueType::Float)
            {
                return Err(fail("Image execution controls must name scalar parameters"));
            }
        }
    }
    if m.default_channel
        .as_ref()
        .is_some_and(|c| !m.output_channels.iter().any(|o| &o.component == c))
    {
        return Err(fail("default_channel must name a declared output_channel"));
    }
    if m.name.is_empty() {
        return Err(fail("Missing tile name"));
    }
    Ok(m)
}

#[cfg(test)]
mod organic_regression {
    use super::*;
    use crate::{
        compile,
        cpu::{Evaluator, SampleContext},
        ir::Channel,
    };

    // Frozen pre-cleanup programs independently exercise the legacy private-input
    // protocol. They are test fixtures only, never registered in the source catalog.
    #[test]
    fn organic_sources_match_the_original_noise_and_matched_fbm() {
        let noise_source = include_str!("../tests/fixtures/organic-noise-reference.tile");
        let fbm_source = include_str!("../tests/fixtures/organic-fbm-reference.tile");
        let noise_meta = definition(noise_source).unwrap();
        let fbm_meta = definition(fbm_source).unwrap();
        let noise = Algorithm::compile(noise_source, &noise_meta, Kernel::Library(0)).unwrap();
        let fbm = Algorithm::compile(fbm_source, &fbm_meta, Kernel::Library(0)).unwrap();
        let context = |uv: [f32; 2]| SampleContext {
            position: [uv[0], uv[1], 0.],
            auxiliary: [[0.; 3]; 3],
            valid: false,
            normal: [0., 0., 1.],
            time: 0.,
            cell: [0.; 3],
            group: [0.; 3],
            instance_valid: false,
            origin: [0.; 3],
            axis_x: [1., 0., 0.],
            axis_y: [0., 1., 0.],
            footprint: None,
        };
        for (scale, layers, amplitude, gain) in [
            (4., 2., 0.4, 0.37),
            (10., 5., 0.6, 0.5),
            (13., 8., 1.3, 0.8),
        ] {
            let source = format!(
                "project 1\nmaterial \"m\" called \"Organic\"\n  row \"n\"\n    use \"Organic Noise\"\n      set \"scaleValue\" to {scale}\n  row \"f\"\n    use \"Organic fBm\"\n      set \"scaleValue\" to {scale}\n      set \"layers\" to {layers}\n      set \"amplitude\" to {amplitude}\n      set \"gain\" to {gain}\n  output base color from row \"n\" xyz\n  output roughness from row \"f\" x\n"
            );
            let program = compile(&source).unwrap();
            let evaluator = Evaluator::new(&program);
            let mut np: Vec<_> = noise_meta
                .parameters
                .iter()
                .map(|p| p.default.clone())
                .collect();
            np[0] = vec![scale];
            let mut fp: Vec<_> = fbm_meta
                .parameters
                .iter()
                .map(|p| p.default.clone())
                .collect();
            for (name, value) in [
                ("scaleValue", scale),
                ("layers", layers),
                ("amplitude", amplitude),
                ("gain", gain),
            ] {
                fp[fbm_meta
                    .parameters
                    .iter()
                    .position(|p| p.id == name)
                    .unwrap()] = vec![value];
            }
            for i in 0..31 {
                let uv = [-1.23 + i as f32 * 0.097, 0.173 - i as f32 * 0.037];
                let expected_noise = noise
                    .evaluate(uv, 0.00001, np.clone(), context(uv), |_, _| [1.; 4])
                    .0
                    .unwrap()[0];
                let expected_fbm = fbm
                    .evaluate(uv, 0.00001, fp.clone(), context(uv), |at, ctx| {
                        noise
                            .evaluate(
                                at,
                                0.00001,
                                np.clone(),
                                ctx.unwrap_or_else(|| context(at)),
                                |_, _| [1.; 4],
                            )
                            .0
                            .unwrap()
                    })
                    .0
                    .unwrap()[0];
                assert_eq!(
                    evaluator.sample("m", Channel::BaseColor, uv).unwrap()[0],
                    expected_noise
                );
                assert_eq!(
                    evaluator.sample("m", Channel::Roughness, uv).unwrap()[0],
                    expected_fbm
                );
            }
        }
    }
}
